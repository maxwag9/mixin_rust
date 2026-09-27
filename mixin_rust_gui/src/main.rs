mod app;
mod model;
mod theme;

use app::MixinRustApp;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Mixin Rust")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Mixin Rust",
        native_options,
        Box::new(|cc| Ok(Box::new(MixinRustApp::new(cc)))),
    )
}
