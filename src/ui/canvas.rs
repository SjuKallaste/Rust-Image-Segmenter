use egui::{Pos2, Rect, TextureOptions, Vec2};

use crate::app::App;
use crate::imaging::build_seg_texture;
use crate::types::Mode;
use crate::ui::calib::{norm_to_screen, screen_to_norm};

const MAX_LABELED_REGIONS: usize = 5;

// <canvas panel>
pub fn show(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let tex_ref = if app.show_seg {
        app.seg_tex.as_ref().or(app.orig_tex.as_ref())
    } else {
        app.orig_tex.as_ref()
    };

    let tex = match tex_ref {
        None => {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No image loaded.\n\nClick  📂 Load Image  to begin.")
                        .size(20.0)
                        .color(egui::Color32::GRAY),
                );
            });
            // <loading card when no image yet>
            if app.task_label.is_some() {
                draw_loading_card(ctx, ui, app);
            }
            // </loading card when no image yet>
            return;
        }
        Some(t) => t,
    };

    // <fit image to panel>
    let avail = ui.available_size();
    let tex_size = tex.size_vec2();
    let fit = (avail.x / tex_size.x).min(avail.y / tex_size.y);
    let disp = tex_size * fit;
    let img_rect = Rect::from_center_size(ui.max_rect().center(), disp);
    app.img_rect = img_rect;
    // </fit image to panel>

    // <input sense and cursor>
    let busy = app.task_label.is_some();
    let sense = if busy {
        egui::Sense::hover()
    } else {
        match &app.mode {
            Mode::CalibP1 | Mode::CalibP2 { .. } | Mode::Segmented => egui::Sense::click(),
            _ => egui::Sense::hover(),
        }
    };
    let response = ui.allocate_rect(img_rect, sense);
    if !busy {
        match &app.mode {
            Mode::CalibP1 | Mode::CalibP2 { .. } => ctx.set_cursor_icon(egui::CursorIcon::Crosshair),
            Mode::Segmented => ctx.set_cursor_icon(egui::CursorIcon::PointingHand),
            _ => {}
        }
    }
    // </input sense and cursor>

    // <draw image or filter mask>
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    if !app.active_color_filters.is_empty() || app.imagej_mode {
        if let Some(cf_tex) = &app.color_filter_tex {
            ui.painter().image(cf_tex.id(), img_rect, uv, egui::Color32::WHITE);
        }
    } else {
        ui.painter().image(tex.id(), img_rect, uv, egui::Color32::WHITE);
    }
    // </draw image or filter mask>

    // <edge overlay>
    if app.show_edges {
        if let Some(et) = &app.edge_tex {
            ui.painter().image(et.id(), img_rect, uv, egui::Color32::WHITE);
        }
    }
    // </edge overlay>

    // <click handling, disabled while busy>
    if !busy && response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            handle_click(app, ctx, pos, img_rect);
        }
    }
    // </click handling, disabled while busy>

    draw_calib_overlay(app, ui, img_rect);

    // <region labels, only the top 5 largest by pixel count>
    if app.show_seg && !app.regions.is_empty() && !busy {
        let font = egui::FontId::proportional(14.0);
        let painter = ui.painter();
        let mut sorted: Vec<usize> = (0..app.regions.len()).collect();
        sorted.sort_by(|&a, &b| app.regions[b].pixel_count.cmp(&app.regions[a].pixel_count));
        for &i in sorted.iter().take(MAX_LABELED_REGIONS) {
            let r = &app.regions[i];
            let cx = img_rect.min.x + r.centroid.0 * disp.x;
            let cy = img_rect.min.y + r.centroid.1 * disp.y;
            let lbl = r.index.to_string();
            painter.text(egui::pos2(cx+1.0, cy+1.0), egui::Align2::CENTER_CENTER, &lbl, font.clone(), egui::Color32::BLACK);
            painter.text(egui::pos2(cx, cy), egui::Align2::CENTER_CENTER, &lbl, font.clone(), egui::Color32::WHITE);
        }
    }
    // </region labels, only the top 5 largest by pixel count>

    // <loading card overlay, drawn on top of image while busy>
    if busy {
        draw_loading_card(ctx, ui, app);
    }
    // </loading card overlay, drawn on top of image while busy>
}
// </canvas panel>

// <loading card>
fn draw_loading_card(ctx: &egui::Context, ui: &mut egui::Ui, app: &App) {
    let label = app.task_label.as_deref().unwrap_or("Working");
    let panel_rect = ui.max_rect();

    let compute_label = if app.gpu_enabled && app.gpu_available {
        "using GPU"
    } else {
        "using CPU"
    };

    // animated dots: . .. ... . .. ...
    let t = ctx.input(|i| i.time) as f32;
    let dot_phase = ((t * 1.5) as usize) % 3;
    let dots = match dot_phase {
        0 => ".",
        1 => "..",
        _ => "...",
    };

    let display = format!("{} {} {}", label, compute_label, dots);

    // dim the background
    ui.painter().rect_filled(panel_rect, 0.0, egui::Color32::from_black_alpha(120));

    // card — wider, shorter, less rounding = more rectangular
    let card_w = 320.0f32;
    let card_h = 52.0f32;
    let card_rect = Rect::from_center_size(panel_rect.center(), Vec2::new(card_w, card_h));

    ui.painter().rect_filled(card_rect, 6.0, egui::Color32::from_rgba_unmultiplied(30, 30, 30, 230));
    ui.painter().rect_stroke(card_rect, 6.0, egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40)));

    ui.painter().text(
        card_rect.center(),
        egui::Align2::CENTER_CENTER,
        &display,
        egui::FontId::proportional(15.0),
        egui::Color32::WHITE,
    );

    ctx.request_repaint();
}
// </loading card>

// <click handler>
fn handle_click(app: &mut App, ctx: &egui::Context, pos: Pos2, img_rect: Rect) {
    let norm = screen_to_norm(pos, img_rect);
    match app.mode.clone() {
        Mode::CalibP1 => {
            app.mode = Mode::CalibP2 { p1: norm };
            app.status = "Now click the second endpoint.".into();
        }
        Mode::CalibP2 { p1 } => {
            app.mode = Mode::CalibLen { p1, p2: norm };
            app.status = "Enter the length of this line in the toolbar above.".into();
        }
        Mode::Segmented => {
            let px = ((norm.x * app.img_w as f32) as usize).min(app.img_w as usize - 1);
            let py = ((norm.y * app.img_h as f32) as usize).min(app.img_h as usize - 1);
            if let Some(l) = app.label_map.get(py * app.img_w as usize + px).copied() {
                if l >= 0 {
                    let ri = l as usize;
                    if app.selected.contains(&ri) { app.selected.remove(&ri); }
                    else { app.selected.insert(ri); }
                    let n = app.regions.len();
                    let ci = build_seg_texture(&app.label_map, app.img_w, app.img_h, n, &app.selected);
                    app.seg_tex = Some(ctx.load_texture("seg", ci, TextureOptions::default()));
                    app.show_seg = true;
                }
            }
        }
        _ => {}
    }
}
// </click handler>

// <calibration overlay drawing>
fn draw_calib_overlay(app: &App, ui: &mut egui::Ui, img_rect: Rect) {
    let painter = ui.painter();
    let dot = |p: Pos2| {
        let s = norm_to_screen(p, img_rect);
        painter.circle_filled(s, 7.0, egui::Color32::from_rgb(255, 215, 0));
        painter.circle_stroke(s, 7.0, egui::Stroke::new(2.0, egui::Color32::BLACK));
    };
    match &app.mode {
        Mode::CalibP2 { p1 } => { dot(*p1); }
        Mode::CalibLen { p1, p2 } => {
            let s1 = norm_to_screen(*p1, img_rect);
            let s2 = norm_to_screen(*p2, img_rect);
            painter.line_segment([s1, s2], egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 215, 0)));
            dot(*p1);
            dot(*p2);
            let mid = Pos2::new((s1.x + s2.x) / 2.0, (s1.y + s2.y) / 2.0 - 18.0);
            painter.rect_filled(Rect::from_center_size(mid, Vec2::new(195.0, 22.0)), 4.0, egui::Color32::from_black_alpha(175));
            painter.text(mid, egui::Align2::CENTER_CENTER, "Enter length in toolbar", egui::FontId::proportional(13.0), egui::Color32::YELLOW);
        }
        _ => {}
    }
}
// </calibration overlay drawing>