mod app;
mod database;
mod export;
mod models;
mod pdf_parser;
mod tema;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 800.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_title("Metrajmatik - Yaklasik Maliyet / Metraj Programi"),
        ..Default::default()
    };

    eframe::run_native(
        "Metrajmatik",
        options,
        Box::new(|cc| {
            tema::uygula(&cc.egui_ctx);
            Ok(Box::new(app::MetrajApp::default()))
        }),
    )
}