use egui::TextureOptions;
use rfd::FileDialog;
use std::collections::HashSet;

use crate::app::App;
use crate::color::build_color_filter_texture;
use crate::color::compute_prominent_filters;
use crate::imaging::{box_blur, build_seg_texture, dyn_to_color_image, sobel_texture};
use crate::export::export_csv;
use crate::segment::segment;
use crate::types::{Mode, Unit};
use crate::ui::calib::norm_to_px_dist;

// <toolbar panel>
pub fn show(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.add_space(5.0);

        // <row 1: file, calibration, segment, view>
        ui.horizontal_wrapped(|ui| {
            show_load_button(app, ctx, ui);
            ui.separator();
            show_calibration(app, ctx, ui);

            if let Some(s) = app.scale_px_per_cm {
                ui.colored_label(egui::Color32::from_rgb(100, 220, 100), format!("✔ {:.3} px/cm", s));
            }

            ui.separator();
            show_segment_button(app, ctx, ui);
            ui.separator();

            if app.seg_tex.is_some() {
                ui.checkbox(&mut app.show_seg, "Segmented view");
                ui.checkbox(&mut app.show_edges, "Edge overlay");
                ui.separator();
            }

            ui.label("Unit:");
            egui::ComboBox::from_id_salt("unit_sel")
                .selected_text(app.unit.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.unit, Unit::Cm2, "cm²");
                    ui.selectable_value(&mut app.unit, Unit::Mm2, "mm²");
                });
        });
        // </row 1: file, calibration, segment, view>

        ui.add_space(3.0);

        // <row 2: params, selection, export>
        ui.horizontal_wrapped(|ui| {
            ui.label("Colour tol:");
            ui.add(egui::Slider::new(&mut app.tolerance, 5..=255).clamp_to_range(true));
            ui.label("Min px:");
            ui.add(egui::Slider::new(&mut app.min_pixels, 50..=50_000).clamp_to_range(true));
            ui.label("Blur:");
            ui.add(egui::Slider::new(&mut app.blur_radius, 0..=15).clamp_to_range(true))
                .on_hover_text("Box blur radius before segmentation (0 = off)");

            if !app.regions.is_empty() {
                ui.separator();
                if ui.button("☑ Select All").clicked() {
                    app.selected = (0..app.regions.len()).collect();
                    let n = app.regions.len();
                    let ci = build_seg_texture(&app.label_map, app.img_w, app.img_h, n, &app.selected);
                    app.seg_tex = Some(ctx.load_texture("seg", ci, TextureOptions::default()));
                }
                if ui.add_enabled(!app.selected.is_empty(), egui::Button::new("✖ Clear Sel.")).clicked() {
                    app.selected.clear();
                    let n = app.regions.len();
                    let ci = build_seg_texture(&app.label_map, app.img_w, app.img_h, n, &app.selected);
                    app.seg_tex = Some(ctx.load_texture("seg", ci, TextureOptions::default()));
                }
                ui.separator();
                if ui.button("💾  Export CSV").clicked() {
                    app.status = export_csv(&app.regions, &app.unit);
                }
            }
        });
        // </row 2: params, selection, export>

        ui.add_space(4.0);
    });
}
// </toolbar panel>

// <load image>
fn show_load_button(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    if ui.button("📂  Load Image").clicked() {
        if let Some(path) = FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tiff", "webp"])
            .pick_file()
        {
            match image::open(&path) {
                Ok(img) => {
                    let ci = dyn_to_color_image(&img);
                    app.orig_tex = Some(ctx.load_texture("orig", ci, TextureOptions::default()));
                    app.img_w = img.width();
                    app.img_h = img.height();
                    app.image = Some(img);
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
                    app.prominent_filter_indices = compute_prominent_filters(app.image.as_ref().unwrap(), &app.color_filters, 0.05);
                    app.show_all_colors = false;
                    app.status = format!("Loaded ({} × {} px). Step 2 - Set Scale.", app.img_w, app.img_h);
                }
                Err(e) => app.status = format!("Error: {e}"),
            }
        }
    }
}
// </load image>

// <calibration controls>
fn show_calibration(app: &mut App, _ctx: &egui::Context, ui: &mut egui::Ui) {
    match app.mode.clone() {
        Mode::CalibP1 => {
            ui.colored_label(egui::Color32::YELLOW, "🎯 Click FIRST endpoint on image");
            if ui.button("✖ Cancel").clicked() { app.mode = Mode::Ready; }
        }
        Mode::CalibP2 { .. } => {
            ui.colored_label(egui::Color32::YELLOW, "🎯 Click SECOND endpoint on image");
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
            if ui.add_enabled(app.image.is_some(), egui::Button::new("📏  Set Scale"))
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

// <segment button>
fn show_segment_button(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let can_seg = app.image.is_some()
        && app.scale_px_per_cm.is_some()
        && !matches!(app.mode, Mode::CalibP1 | Mode::CalibP2 { .. } | Mode::CalibLen { .. });

    if ui.add_enabled(can_seg, egui::Button::new("⚙  Segment"))
        .on_hover_text("Detect coloured regions and compute their areas")
        .clicked()
    {
        if let (Some(img), Some(scale)) = (&app.image, app.scale_px_per_cm) {
            let processed = box_blur(img, app.blur_radius);
            let (labels, regions) = segment(&processed, app.tolerance, app.min_pixels, scale);
            let n = regions.len();
            let ci_seg = build_seg_texture(&labels, app.img_w, app.img_h, n, &HashSet::new());
            let ci_edge = sobel_texture(&processed);

            if !app.active_color_filters.is_empty() {
                let active_refs: Vec<&_> = app.active_color_filters.iter().map(|&i| &app.color_filters[i]).collect();
                let ci_cf = build_color_filter_texture(img, &active_refs);
                app.color_filter_tex = Some(ctx.load_texture("cf", ci_cf, TextureOptions::default()));
            }

            app.seg_tex = Some(ctx.load_texture("seg", ci_seg, TextureOptions::default()));
            app.edge_tex = Some(ctx.load_texture("edge", ci_edge, TextureOptions::default()));
            app.total_area_cm2 = regions.iter().map(|r| r.area_cm2).sum();
            app.label_map = labels;
            app.regions = regions;
            app.selected.clear();
            app.show_seg = true;
            app.show_edges = false;
            app.mode = Mode::Segmented;
            app.status = format!("Done - {n} region(s) found. Click any region to select it.");
        }
    }
}
// </segment button>
