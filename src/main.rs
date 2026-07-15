// Metrajmatik — yaklaşık maliyet, metraj ve hakediş programı
// Copyright (C) 2026 Enes Aydoğan
//
// Bu program özgür yazılımdır: GNU Affero Genel Kamu Lisansı'nın 3. sürümü
// (veya tercihinize göre daha sonraki bir sürümü) koşulları altında yeniden
// dağıtabilir ve/veya değiştirebilirsiniz. Ayrıntılar için depo kökündeki
// LICENSE dosyasına bakın. HİÇBİR GARANTİSİ YOKTUR.
// <https://www.gnu.org/licenses/agpl-3.0.html>

mod app;
mod bicim;
mod database;
mod export;
mod hakedis;
mod is_grubu;
mod maliyet;
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
