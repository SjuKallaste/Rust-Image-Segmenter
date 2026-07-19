use egui::TextureOptions;
use rfd::FileDialog;
use std::collections::HashSet;
use std::sync::mpsc;

use crate::app::{App, TaskKind, TaskPayload, TaskResult};
use crate::color::{build_color_filter_texture, compute_prominent_filters};
use crate::gpu::gpu_compute_seg_edges;
use crate::imaging::{box_blur, build_seg_texture, dyn_to_color_image, sobel_texture};
use crate::export::export_csv;
use crate::segment::{segment, segment_gpu, segment_parallel};
use crate::types::{Mode, SegmentEngine};
use crate::ui::calib::norm_to_px_dist;

// <toolbar, classic menu bar plus a slim action row>
pub fn show(app: &mut App, ctx: &egui::Context) {
    poll_task(app, ctx);
    let busy = app.task_rx.is_some();

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        show_menu_bar(app, ctx, ui, busy);
    });

    egui::TopBottomPanel::top("action_row").show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            show_load_button(app, ctx, ui, busy);
            ui.separator();
            show_calibration(app, ctx, ui, busy);
            if let Some(s) = app.scale_px_per_cm {
                ui.colored_label(egui::Color32::from_rgb(100, 200, 255), format!("✔ {:.3} px/cm", s));
            }
            ui.separator();
            show_segment_button(app, ctx, ui, busy);
        });
        ui.add_space(3.0);
        ui.horizontal_wrapped(|ui| {
            show_slider_group(ui, busy, "Colour tol", &mut app.tolerance, |ui, v| {
                ui.add(egui::Slider::new(v, 5..=255));
            });
            show_slider_group(ui, busy, "Min px", &mut app.min_pixels, |ui, v| {
                ui.add(egui::Slider::new(v, 50..=50_000));
            });
            show_slider_group(ui, busy, "Blur", &mut app.blur_radius, |ui, v| {
                ui.add(egui::Slider::new(v, 0..=15));
            });
        });
        ui.add_space(4.0);
    });
}
// </toolbar, classic menu bar plus a slim action row>

// <generic collapsible slider group box>
fn show_slider_group<T: Copy + std::fmt::Display>(
    ui: &mut egui::Ui,
    busy: bool,
    title: &str,
    value: &mut T,
    body: impl FnOnce(&mut egui::Ui, &mut T),
) {
    let current = *value;
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.add_enabled_ui(!busy, |ui| {
                egui::CollapsingHeader::new(format!("{title}: {current}"))
                    .id_source(title)
                    .default_open(false)
                    .show(ui, |ui| {
                        body(ui, value);
                    });
            });
        });
}
// </generic collapsible slider group box>

// <classic menu bar: file / edit / view / settings>
fn show_menu_bar(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui, busy: bool) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.add_enabled(!busy, egui::Button::new("📂  Load Image")).clicked() {
                trigger_load(app, ui);
                ui.close_menu();
            }
            if ui.add_enabled(!busy && !app.regions.is_empty(), egui::Button::new("💾  Export CSV")).clicked() {
                app.status = export_csv(&app.regions, &app.unit);
                ui.close_menu();
            }
            if ui.add_enabled(!busy, egui::Button::new("🔄  Reset")).clicked() {
                do_reset(app);
                ui.close_menu();
            }
            if ui.button("✖  Exit").clicked() {
                std::process::exit(0);
            }
        });

        ui.menu_button("Edit", |ui| {
            let has_regions = !app.regions.is_empty();
            if ui.add_enabled(!busy && has_regions, egui::Button::new("☑  Select All")).clicked() {
                app.selected = (0..app.regions.len()).collect();
                let n = app.regions.len();
                let ci = build_seg_texture(&app.label_map, app.img_w, app.img_h, n, &app.selected);
                app.seg_tex = Some(ctx.load_texture("seg", ci, TextureOptions::default()));
                ui.close_menu();
            }
            if ui.add_enabled(!busy && !app.selected.is_empty(), egui::Button::new("✖  Clear Selection")).clicked() {
                app.selected.clear();
                let n = app.regions.len();
                let ci = build_seg_texture(&app.label_map, app.img_w, app.img_h, n, &app.selected);
                app.seg_tex = Some(ctx.load_texture("seg", ci, TextureOptions::default()));
                ui.close_menu();
            }
        });

        ui.menu_button("View", |ui| {
            ui.add_enabled_ui(app.seg_tex.is_some(), |ui| {
                ui.checkbox(&mut app.show_seg, "Segmented view");
                ui.checkbox(&mut app.show_edges, "Edge overlay");
            });
        });

        ui.menu_button("Settings", |ui| {
            crate::ui::settings::show_inline(app, ui);
        });

        if let Some(label) = &app.task_label {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(egui::Color32::from_rgb(100, 200, 255), label);
            });
        }
    });
}
// </classic menu bar: file / edit / view / settings>

// <shared reset logic, keeps the existing gpu context alive>
fn do_reset(app: &mut App) {
    let gpu_ctx = app.gpu_ctx.take();
    let gpu_available = app.gpu_available;
    let gpu_is_discrete = app.gpu_is_discrete;
    *app = App::default();
    app.gpu_ctx = gpu_ctx;
    app.gpu_available = gpu_available;
    app.gpu_is_discrete = gpu_is_discrete;
    app.gpu_enabled = gpu_is_discrete;
}
// </shared reset logic, keeps the existing gpu context alive>

// <poll background task, call once per frame before rendering>
fn poll_task(app: &mut App, ctx: &egui::Context) {
    let result = if let Some(rx) = &app.task_rx {
        match rx.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint();
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                app.task_rx = None;
                app.task_label = None;
                return;
            }
        }
    } else {
        return;
    };

    app.task_rx = None;
    app.task_label = None;

    let result = match result {
        Some(r) => r,
        None => return,
    };

    match result.payload {
        TaskPayload::Loaded { image, rgb, prominent } => {
            let ci = dyn_to_color_image(&image);
            app.orig_tex = Some(ctx.load_texture("orig", ci, TextureOptions::default()));
            app.img_w = image.width();
            app.img_h = image.height();
            app.prominent_filter_indices = prominent;
            app.image = Some(image);
            app.rgb_cache = Some(rgb);
            app.seg_tex = None;
            app.edge_tex = None;
            app.color_filter_tex = None;
            app.show_seg = false;
            app.show_edges = false;
            app.active_color_filters.clear();
            app.scale_px_per_cm = None;
            app.label_map.clear();
            app.regions.clear();
            app.selected.clear();
            app.total_area_cm2 = 0.0;
            app.mode = Mode::Ready;
            app.show_all_colors = false;
            app.status = format!("Loaded ({} × {} px). Step 2 - Set Scale.", app.img_w, app.img_h);
        }

        TaskPayload::Segmented { labels, regions } => {
            let n = regions.len();
            let ci_seg = build_seg_texture(&labels, app.img_w, app.img_h, n, &HashSet::new());
            app.seg_tex = Some(ctx.load_texture("seg", ci_seg, TextureOptions::default()));

            if let Some(rgb) = &app.rgb_cache {
                let processed = box_blur(rgb, app.blur_radius);
                let ci_edge = sobel_texture(&processed);
                app.edge_tex = Some(ctx.load_texture("edge", ci_edge, TextureOptions::default()));

                if !app.active_color_filters.is_empty() {
                    let active_refs: Vec<&_> = app.active_color_filters.iter().map(|&i| &app.color_filters[i]).collect();
                    let ci_cf = build_color_filter_texture(rgb, &active_refs);
                    app.color_filter_tex = Some(ctx.load_texture("cf", ci_cf, TextureOptions::default()));
                }
            }

            let engine = app.segment_engine.label();
            app.total_area_cm2 = regions.iter().map(|r| r.area_cm2).sum();
            app.label_map = labels;
            app.regions = regions;
            app.selected.clear();
            app.show_seg = true;
            app.show_edges = false;
            app.mode = Mode::Segmented;
            app.status = format!("Done ({engine}) - {n} region(s) found. Click any region to select it.");
        }

        TaskPayload::Filtered => {}
    }
}
// </poll background task, call once per frame before rendering>

// <trigger load, shared by File menu and action row button>
fn trigger_load(app: &mut App, _ui: &mut egui::Ui) {
    if let Some(path) = FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tiff", "webp"])
        .pick_file()
    {
        let filters = app.color_filters.clone();
        let (tx, rx) = mpsc::channel();
        app.task_rx = Some(rx);
        app.task_label = Some("Loading image".into());

        std::thread::spawn(move || {
            if let Ok(img) = image::open(&path) {
                let rgb = img.to_rgb8();
                let prominent = compute_prominent_filters(&rgb, &filters, 0.05);
                let _ = tx.send(TaskResult {
                    kind: TaskKind::Loading,
                    payload: TaskPayload::Loaded { image: img, rgb, prominent },
                });
            }
        });
    }
}
// </trigger load, shared by File menu and action row button>

// <load button, action row>
fn show_load_button(app: &mut App, _ctx: &egui::Context, ui: &mut egui::Ui, busy: bool) {
    if ui.add_enabled(!busy, egui::Button::new("📂  Load Image")).clicked() {
        trigger_load(app, ui);
    }
}
// </load button, action row>

// <calibration controls>
fn show_calibration(app: &mut App, _ctx: &egui::Context, ui: &mut egui::Ui, busy: bool) {
    match app.mode.clone() {
        Mode::CalibP1 => {
            ui.colored_label(egui::Color32::from_rgb(255, 210, 60), "🎯 Click FIRST endpoint on image");
            if ui.button("✖ Cancel").clicked() { app.mode = Mode::Ready; }
        }
        Mode::CalibP2 { .. } => {
            ui.colored_label(egui::Color32::from_rgb(255, 210, 60), "🎯 Click SECOND endpoint on image");
            if ui.button("✖ Cancel").clicked() { app.mode = Mode::Ready; }
        }
        Mode::CalibLen { p1, p2 } => {
            ui.label("Line length:");
            let resp = ui.add(egui::TextEdit::singleline(&mut app.calib_len_buf).desired_width(65.0).hint_text("e.g. 5.0"));
            ui.label("cm");
            let confirmed = ui.button("✔ Confirm").clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if confirmed {
                match app.calib_len_buf.trim().parse::<f64>() {
                    Ok(len) if len > 0.0 => {
                        let px_dist = norm_to_px_dist(p1, p2, app.img_w, app.img_h);
                        let scale = px_dist / len;
                        app.scale_px_per_cm = Some(scale);
                        app.mode = Mode::Ready;
                        app.calib_len_buf.clear();
                        app.status = format!("Scale set: {:.3} px/cm ({:.5} cm/px). Step 3 - Segment.", scale, 1.0 / scale);
                    }
                    Ok(_) => app.status = "Length must be > 0.".into(),
                    Err(_) => app.status = "Enter a valid decimal number.".into(),
                }
            }
            if ui.button("✖ Cancel").clicked() {
                app.mode = Mode::Ready;
                app.calib_len_buf.clear();
            }
        }
        _ => {
            if ui.add_enabled(app.image.is_some() && !busy, egui::Button::new("📏  Set Scale"))
                .on_hover_text("Draw a line over a known reference length to calibrate")
                .clicked()
            {
                app.mode = Mode::CalibP1;
                app.status = "Click the first endpoint of your reference line.".into();
            }
        }
    }
}
// </calibration controls>

// <segment button, dispatches to exact / parallel / gpu on a background thread>
fn show_segment_button(app: &mut App, _ctx: &egui::Context, ui: &mut egui::Ui, busy: bool) {
    let can_seg = app.rgb_cache.is_some()
        && app.scale_px_per_cm.is_some()
        && !busy
        && !matches!(app.mode, Mode::CalibP1 | Mode::CalibP2 { .. } | Mode::CalibLen { .. });

    if ui.add_enabled(can_seg, egui::Button::new("⚙  Segment"))
        .on_hover_text("Detect coloured regions and compute their areas. Engine is chosen in Settings.")
        .clicked()
    {
        if let (Some(rgb), Some(scale)) = (app.rgb_cache.clone(), app.scale_px_per_cm) {
            let tol = app.tolerance;
            let min_px = app.min_pixels;
            let blur = app.blur_radius;
            let engine = app.segment_engine;
            let gpu_ctx = app.gpu_ctx.clone();

            app.task_label = Some(format!("Segmenting ({})", engine.label()));
            let (tx, rx) = mpsc::channel();
            app.task_rx = Some(rx);

            std::thread::spawn(move || {
                let processed = box_blur(&rgb, blur);

                let (labels, regions) = match engine {
                    SegmentEngine::Exact => segment(&processed, tol, min_px, scale),
                    SegmentEngine::Parallel => segment_parallel(&processed, tol, min_px, scale),
                    SegmentEngine::Gpu => match &gpu_ctx {
                        Some(ctx) => match gpu_compute_seg_edges(ctx, &processed, tol) {
                            Some(edges) => segment_gpu(&processed, &edges, min_px, scale),
                            None => segment_parallel(&processed, tol, min_px, scale),
                        },
                        None => segment_parallel(&processed, tol, min_px, scale),
                    },
                };

                let _ = tx.send(TaskResult {
                    kind: TaskKind::Segmenting,
                    payload: TaskPayload::Segmented { labels, regions },
                });
            });
        }
    }
}
// </segment button, dispatches to exact / parallel / gpu on a background thread>