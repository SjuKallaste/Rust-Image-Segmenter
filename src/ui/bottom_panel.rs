use egui::{ScrollArea, Vec2};

use crate::app::App;
use crate::color::{pixel_area_for_filter, pixel_area_imagej_auto};

// <bottom panel>
pub fn show(app: &App, ui: &mut egui::Ui) {
    ui.add_space(5.0);
    if app.imagej_mode {
        show_imagej_mode(app, ui);
    } else if !app.active_color_filters.is_empty() {
        show_filter_mode(app, ui);
    } else if !app.regions.is_empty() {
        show_normal_mode(app, ui);
    }
}
// </bottom panel>

// <imagej area display, reads app.rgb_cache>
fn show_imagej_mode(app: &App, ui: &mut egui::Ui) {
    let Some(rgb) = &app.rgb_cache else { return };
    let Some(scale) = app.scale_px_per_cm else { return };
    let (px_count, area_cm2) = pixel_area_imagej_auto(
        app, rgb,
        app.imagej_hue_min, app.imagej_hue_max,
        app.imagej_sat_min, app.imagej_sat_max,
        app.imagej_bri_min, app.imagej_bri_max,
        scale,
    );
    let factor = app.unit.factor();
    let unit_lbl = app.unit.label();
    ui.separator();
    egui::Grid::new("imagej_area_table").num_columns(3).spacing([20.0, 6.0]).show(ui, |ui| {
        ui.strong("Mode");
        ui.strong(format!("Area ({})", unit_lbl));
        ui.strong("Pixels");
        ui.end_row();
        ui.label(egui::RichText::new("ImageJ HSB").strong().size(15.0));
        ui.label(egui::RichText::new(format!("{:.4}", area_cm2 * factor)).strong().size(15.0));
        ui.label(egui::RichText::new(px_count.to_string()).size(13.0).color(egui::Color32::GRAY));
        ui.end_row();
    });
}
// </imagej area display, reads app.rgb_cache>

// <named filter area display, reads app.rgb_cache>
fn show_filter_mode(app: &App, ui: &mut egui::Ui) {
    let Some(rgb) = &app.rgb_cache else { return };
    let Some(scale) = app.scale_px_per_cm else { return };
    let factor = app.unit.factor();
    let unit_lbl = app.unit.label();
    ui.separator();
    let mut filter_indices: Vec<usize> = app.active_color_filters.iter().copied().collect();
    filter_indices.sort();
    egui::Grid::new("filter_area_table").num_columns(3).striped(false).spacing([20.0, 6.0]).show(ui, |ui| {
        ui.strong("Color");
        ui.strong(format!("Area ({})", unit_lbl));
        ui.strong("Pixels");
        ui.end_row();
        for fi in &filter_indices {
            let f = &app.color_filters[*fi];
            let (px_count, area_cm2) = pixel_area_for_filter(rgb, f, scale);
            ui.horizontal(|ui| {
                let (sw, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::hover());
                ui.painter().rect_filled(sw, 3.0, f.swatch);
                ui.label(egui::RichText::new(f.label).strong().size(15.0));
            });
            ui.label(egui::RichText::new(format!("{:.4}", area_cm2 * factor)).strong().size(15.0));
            ui.label(egui::RichText::new(px_count.to_string()).size(13.0).color(egui::Color32::GRAY));
            ui.end_row();
        }
    });
}
// </named filter area display, reads app.rgb_cache>

// <normal region table>
fn show_normal_mode(app: &App, ui: &mut egui::Ui) {
    let factor = app.unit.factor();
    let unit_lbl = app.unit.label();
    let sel_area: Option<f64> = if app.selected.is_empty() { None } else {
        Some(app.regions.iter().filter(|r| app.selected.contains(&(r.index - 1))).map(|r| r.area_cm2).sum())
    };
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("Total area: {:.4} {}   |   {} region(s)", app.total_area_cm2 * factor, unit_lbl, app.regions.len())).strong().size(14.0));
        if let Some(sa) = sel_area {
            ui.separator();
            ui.label(egui::RichText::new(format!("Selected: {:.4} {}  ({} region(s))", sa * factor, unit_lbl, app.selected.len())).strong().size(14.0).color(egui::Color32::from_rgb(255, 210, 60)));
        }
    });
    ui.separator();
    ScrollArea::vertical().max_height(110.0).id_salt("rtbl").show(ui, |ui| {
        egui::Grid::new("region_table").num_columns(5).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.strong("Region"); ui.strong("Colour"); ui.strong("Avg RGB"); ui.strong("Pixels"); ui.strong(format!("Area ({})", unit_lbl));
            ui.end_row();
            for r in &app.regions {
                let is_sel = app.selected.contains(&(r.index - 1));
                let [cr, cg, cb] = r.avg_color;
                let label_text = if is_sel {
                    egui::RichText::new(format!("#{}", r.index)).color(egui::Color32::from_rgb(255, 210, 60)).strong()
                } else {
                    egui::RichText::new(format!("#{}", r.index))
                };
                ui.label(label_text);
                let (sw, _) = ui.allocate_exact_size(Vec2::new(30.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(sw, 3.0, egui::Color32::from_rgb(cr, cg, cb));
                ui.label(format!("({cr},{cg},{cb})"));
                ui.label(r.pixel_count.to_string());
                ui.label(format!("{:.4}", r.area_cm2 * factor));
                ui.end_row();
            }
        });
    });
}
// </normal region table>