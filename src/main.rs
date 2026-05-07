#![windows_subsystem = "windows"]

mod app;
mod color;
mod export;
mod imaging;
mod segment;
mod types;
mod ui;

use app::App;

// <entry point>
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Image Segmenter",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Image Segmenter")
                .with_inner_size([1250.0, 860.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
// </entry point>
