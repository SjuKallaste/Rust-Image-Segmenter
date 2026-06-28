use egui::{TextureOptions, Vec2};

use crate::app::App;
use crate::color::{build_color_filter_texture, build_imagej_filter_texture};

// <side panel>
pub fn show(app: &mut App, ctx: &egui::Context) {
    egui::SidePanel::right("color_filter_panel")
        .resizable(false)
        .min_width(155.0)
        .max_width(155.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.label(egui::RichText::new("🎨 Color Filter").strong());
            ui.separator();

            if app.rgb_cache.is_none() {
                ui.label(egui::RichText::new("Load an image to\ndetect colors.").italics().small().color(egui::Color32::GRAY));
                return;
            }

            ui.horizontal(|ui| {
                if ui.selectable_label(!app.imagej_mode, "Named").clicked() {
                    app.imagej_mode = false;
                    app.color_filter_tex = None;
                }
                ui.separator();
                if ui.selectable_label(app.imagej_mode, "ImageJ HSB").clicked() {
                    app.imagej_mode = true;
                    app.active_color_filters.clear();
                    rebuild_filter_texture(app, ctx);
                }
            });

            ui.separator();

            if app.imagej_mode {
                show_imagej_panel(app, ctx, ui);
            } else {
                show_named_panel(app, ctx, ui);
            }

            show_gpu_toggle(app, ui);
        });
}
// </side panel>

// <named color buttons>
fn show_named_panel(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let indices_to_show: Vec<usize> = if app.show_all_colors {
        (0..app.color_filters.len()).collect()
    } else {
        app.prominent_filter_indices.clone()
    };

    if indices_to_show.is_empty() && !app.show_all_colors {
        ui.label(egui::RichText::new("No dominant colors\ndetected (< 5%).").italics().small().color(egui::Color32::GRAY));
    } else {
        for i in indices_to_show {
            let filter = app.color_filters[i].clone();
            let is_active = app.active_color_filters.contains(&i);
            let btn_text = egui::RichText::new(filter.label).strong()
                .color(if is_active { egui::Color32::BLACK } else { egui::Color32::WHITE });
            let btn = egui::Button::new(btn_text)
                .fill(filter.swatch)
                .stroke(egui::Stroke::new(if is_active { 2.5 } else { 0.0 }, egui::Color32::WHITE))
                .min_size(Vec2::new(140.0, 22.0));
            if ui.add(btn).clicked() {
                if is_active { app.active_color_filters.remove(&i); }
                else { app.active_color_filters.insert(i); }
                rebuild_filter_texture(app, ctx);
            }
            ui.add_space(2.0);
        }
    }

    ui.add_space(4.0);
    ui.separator();

    let toggle_label = if app.show_all_colors { "▲ Show detected only" } else { "▼ Show all colors" };
    if ui.button(egui::RichText::new(toggle_label).small()).clicked() {
        app.show_all_colors = !app.show_all_colors;
    }

    ui.add_space(4.0);
    ui.separator();

    if !app.active_color_filters.is_empty() {
        if ui.button("✖  Clear filters").clicked() {
            app.active_color_filters.clear();
            app.color_filter_tex = None;
        }
    } else {
        ui.label(egui::RichText::new("No filter active").italics().small().color(egui::Color32::GRAY));
    }
}
// </named color buttons>

// <imagej hsb sliders>
fn show_imagej_panel(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let mut changed = false;
    ui.label(egui::RichText::new("Hue (0-255)").small().strong());
    changed |= ui.add(egui::Slider::new(&mut app.imagej_hue_min, 0..=255).text("min")).changed();
    changed |= ui.add(egui::Slider::new(&mut app.imagej_hue_max, 0..=255).text("max")).changed();
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Saturation (0-255)").small().strong());
    changed |= ui.add(egui::Slider::new(&mut app.imagej_sat_min, 0..=255).text("min")).changed();
    changed |= ui.add(egui::Slider::new(&mut app.imagej_sat_max, 0..=255).text("max")).changed();
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Brightness (0-255)").small().strong());
    changed |= ui.add(egui::Slider::new(&mut app.imagej_bri_min, 0..=255).text("min")).changed();
    changed |= ui.add(egui::Slider::new(&mut app.imagej_bri_max, 0..=255).text("max")).changed();
    ui.add_space(6.0);
    ui.separator();
    ui.label(egui::RichText::new(format!(
        "H: {}-{}\nS: {}-{}\nB: {}-{}",
        app.imagej_hue_min, app.imagej_hue_max,
        app.imagej_sat_min, app.imagej_sat_max,
        app.imagej_bri_min, app.imagej_bri_max,
    )).small().color(egui::Color32::from_rgb(160, 200, 160)));
    if changed { rebuild_filter_texture(app, ctx); }
}
// </imagej hsb sliders>

// <advanced settings: gpu toggle>
fn show_gpu_toggle(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.separator();
    ui.label(egui::RichText::new("Advanced").small().strong());

    if !app.gpu_available {
        ui.label(
            egui::RichText::new("No compatible GPU found.\nUsing CPU only.")
                .italics().small().color(egui::Color32::GRAY),
        );
        return;
    }

    ui.checkbox(&mut app.gpu_enabled, "GPU-accelerated scanning");

    let note = if app.gpu_is_discrete {
        "Discrete GPU detected.\nEnabled by default for\nimages 4K or larger."
    } else {
        "Integrated/virtual GPU.\nOff by default, CPU is\nusually faster here."
    };
    ui.label(egui::RichText::new(note).italics().small().color(egui::Color32::GRAY));
}
// </advanced settings: gpu toggle>

// <rebuild filter texture, reads app.rgb_cache instead of re-converting>
pub fn rebuild_filter_texture(app: &mut App, ctx: &egui::Context) {
    if app.rgb_cache.is_none() {
        return;
    }

    if app.imagej_mode {
        let ci = {
            let rgb = app.rgb_cache.as_ref().unwrap();
            build_imagej_filter_texture(rgb, app.imagej_hue_min, app.imagej_hue_max, app.imagej_sat_min, app.imagej_sat_max, app.imagej_bri_min, app.imagej_bri_max)
        };
        app.color_filter_tex = Some(ctx.load_texture("cf", ci, TextureOptions::default()));
        return;
    }

    if app.active_color_filters.is_empty() {
        app.color_filter_tex = None;
        return;
    }

    let ci = {
        let rgb = app.rgb_cache.as_ref().unwrap();
        let active_refs: Vec<&_> = app.active_color_filters.iter().map(|&idx| &app.color_filters[idx]).collect();
        build_color_filter_texture(rgb, &active_refs)
    };
    app.color_filter_tex = Some(ctx.load_texture("cf", ci, TextureOptions::default()));
}
// </rebuild filter texture, reads app.rgb_cache instead of re-converting>