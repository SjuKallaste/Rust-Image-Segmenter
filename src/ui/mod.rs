pub mod calib;
pub mod toolbar;
pub mod side_panel;
pub mod bottom_panel;
pub mod canvas;

use crate::app::App;

// <app update loop>
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        toolbar::show(self, ctx);
        side_panel::show(self, ctx);
        egui::TopBottomPanel::bottom("results").min_height(185.0).show(ctx, |ui| {
            bottom_panel::show(self, ui);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            canvas::show(self, ctx, ui);
        });
    }
}
// </app update loop>
