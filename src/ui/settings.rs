use crate::app::App;
use crate::types::{SegmentEngine, Unit};

// <settings content, rendered directly inside the Settings dropdown menu>
pub fn show_inline(app: &mut App, ui: &mut egui::Ui) {
    ui.set_min_width(230.0);

    ui.label(egui::RichText::new("Segmentation").strong());
    ui.horizontal(|ui| {
        ui.label("Engine:");
        engine_radio(app, ui, SegmentEngine::Exact, "Exact", "Single-threaded, identical to the original algorithm. Slower on large images.");
        engine_radio(app, ui, SegmentEngine::Parallel, "Parallel", "Multi-core CPU. Fast, rare seam edge cases at strip boundaries.");
        let gpu_ok = app.gpu_available;
        ui.add_enabled_ui(gpu_ok, |ui| {
            engine_radio(app, ui, SegmentEngine::Gpu, "GPU", "Neighbor-based tolerance on the GPU. Different results from Exact/Parallel on smooth gradients.");
        });
    });
    if !app.gpu_available {
        ui.label(egui::RichText::new("No compatible GPU found for segmentation.").italics().small().color(egui::Color32::GRAY));
    }

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Color Filter Scanning").strong());
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.gpu_enabled, false, "CPU");
        ui.add_enabled_ui(app.gpu_available, |ui| {
            ui.selectable_value(&mut app.gpu_enabled, true, "GPU");
        });
    });
    if app.gpu_available {
        let note = if app.gpu_is_discrete { "Discrete GPU detected." } else { "Integrated/virtual GPU, may not be faster than CPU." };
        ui.label(egui::RichText::new(note).italics().small().color(egui::Color32::GRAY));
    } else {
        ui.label(egui::RichText::new("No compatible GPU found. CPU only.").italics().small().color(egui::Color32::GRAY));
    }

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Units").strong());
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.unit, Unit::Cm2, "cm²");
        ui.selectable_value(&mut app.unit, Unit::Mm2, "mm²");
    });
}
// </settings content, rendered directly inside the Settings dropdown menu>

fn engine_radio(app: &mut App, ui: &mut egui::Ui, value: SegmentEngine, label: &str, hover: &str) {
    let selected = app.segment_engine == value;
    if ui.selectable_label(selected, label).on_hover_text(hover).clicked() {
        app.segment_engine = value;
    }
}