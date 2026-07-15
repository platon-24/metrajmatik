//! İş Programı sekmesi: sözleşme bedelini süre boyunca aylara pursantaj (%) olarak
//! dağıtır. Aylık/kümülatif tablo, ilerleme (S) eğrisi grafiği ve Excel çıktısı.

use eframe::egui;
use egui::{CornerRadius, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui};

use crate::bicim::para_formatla;
use crate::export::ay_adi;
use crate::models::ProjeAsamasi;
use crate::tema;

use super::{MetrajApp, Sekme};

impl MetrajApp {
    pub(crate) fn render_is_programi(&mut self, ui: &mut Ui) {
        if self.proje_asamasi == ProjeAsamasi::Metraj {
            tema::sayfa_basligi(
                ui,
                "Sözleşme sonrası araç",
                "İş Programı kilitli",
                "Pursantaj planı, hakediş aşaması başladığında sözleşme bedeli üzerinden etkinleşir.",
            );
            ui.add_space(6.0);
            tema::bildirim_seridi(
                ui,
                "İş Programı, metraj sözleşmeye bağlanıp hakedişe dönüştürüldükten sonra etkinleşir.",
                tema::UYARI_KOYU,
                tema::UYARI,
                tema::UYARI,
            );
            ui.add_space(8.0);
            if tema::birincil_buton(ui, "Hakediş Dönüşümüne Git").clicked() {
                self.sekme_ac(Sekme::Hakedis);
            }
            return;
        }
        let toplam_bedel = self.sozlesme_ayarlari.hesaplanan_sozlesme_bedeli();
        // Dağılım uzunluğunu süre ile hizala (süre değiştiyse eşit böler).
        self.is_programi.normalize();

        tema::sayfa_basligi(
            ui,
            "Planlama çalışma alanı",
            "İş Programı",
            "Sözleşme bedelini aylara dağıtın; pursantajı ve kümülatif ilerlemeyi izleyin.",
        );
        ui.horizontal_wrapped(|ui| {
            tema::istatistik(
                ui,
                "Sözleşme bedeli",
                &format!("{} TL", para_formatla(toplam_bedel)),
                "Planlama bazı",
                tema::BASARI,
            );
            tema::istatistik(
                ui,
                "Süre",
                &format!("{} ay", self.is_programi.sure_ay),
                "Toplam takvim",
                tema::VURGU_HOVER,
            );
            tema::istatistik(
                ui,
                "Pursantaj",
                &format!("% {:.2}", self.is_programi.toplam_yuzde()),
                "Hedef %100",
                tema::AKSAN,
            );
        });
        ui.add_space(10.0);

        // ---- Ayarlar kartı ----
        let mut esit_dagit = false;
        let mut excel = false;
        tema::kart(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Başlangıç")
                        .color(tema::METIN_IKINCIL)
                        .size(12.5),
                );
                if ui
                    .add(
                        egui::DragValue::new(&mut self.is_programi.baslangic_ay)
                            .range(1..=12)
                            .custom_formatter(|n, _| ay_adi(n as u32).to_string()),
                    )
                    .changed()
                {
                    self.degisiklik_var = true;
                }
                if ui
                    .add(
                        egui::DragValue::new(&mut self.is_programi.baslangic_yil)
                            .range(2000..=2100),
                    )
                    .changed()
                {
                    self.degisiklik_var = true;
                }

                ui.add_space(14.0);
                ui.label(
                    RichText::new("Süre (ay)")
                        .color(tema::METIN_IKINCIL)
                        .size(12.5),
                );
                if ui
                    .add(egui::DragValue::new(&mut self.is_programi.sure_ay).range(1..=120))
                    .changed()
                {
                    self.degisiklik_var = true;
                }

                ui.add_space(14.0);
                if tema::ikincil_ikonlu_buton(ui, tema::ikon::ICMAL, "Eşit Dağıt")
                    .on_hover_text("Tüm ayları eşit yüzdeye böler")
                    .clicked()
                {
                    esit_dagit = true;
                }
                if tema::basari_ikonlu_buton(ui, tema::ikon::ICMAL, "Excel").clicked() {
                    excel = true;
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Sözleşme Bedeli:")
                        .color(tema::METIN_SOLUK)
                        .size(12.0),
                );
                ui.label(
                    RichText::new(format!("{} TL", para_formatla(toplam_bedel)))
                        .color(tema::BASARI)
                        .strong()
                        .size(13.5),
                );
            });
        });
        // Süre değişmiş olabilir; tabloyu çizmeden önce yeniden hizala.
        self.is_programi.normalize();
        if esit_dagit {
            self.is_programi.esit_dagit();
            self.degisiklik_var = true;
        }
        if excel {
            self.is_programi_excel_diyalog();
        }

        ui.add_space(8.0);

        if toplam_bedel <= 0.0 {
            tema::bildirim_seridi(ui, "Metraj sekmesinden kalem ekleyin — iş programı sözleşme bedeli üzerinden dağıtılır.", tema::UYARI_KOYU, tema::UYARI, tema::UYARI);
            return;
        }

        // Toplam yüzde kontrolü
        let toplam_yuzde = self.is_programi.toplam_yuzde();
        if (toplam_yuzde - 100.0).abs() > 0.05 {
            tema::bildirim_seridi(
                ui,
                &format!("Pursantaj toplamı % {:.2} (100 olmalı). 'Eşit Dağıt' ile sıfırlayabilir veya elle düzeltebilirsiniz.", toplam_yuzde),
                tema::UYARI_KOYU, tema::UYARI, tema::UYARI,
            );
            ui.add_space(6.0);
        }

        // ---- İlerleme (S) eğrisi grafiği ----
        self.is_programi_grafik(ui, toplam_bedel);
        ui.add_space(10.0);

        // ---- Aylık dağılım tablosu (düzenlenebilir) ----
        tema::kart(ui, |ui| {
            ScrollArea::both().max_height(360.0).show(ui, |ui| {
                egui::Grid::new("is_prog_grid")
                    .num_columns(6)
                    .spacing(egui::vec2(14.0, 7.0))
                    .striped(true)
                    .show(ui, |ui| {
                        for b in [
                            "Ay",
                            "Dönem",
                            "Pursantaj %",
                            "Aylık Tutar",
                            "Kümülatif %",
                            "Kümülatif Tutar",
                        ] {
                            ui.label(
                                RichText::new(b)
                                    .strong()
                                    .size(12.0)
                                    .color(tema::METIN_IKINCIL),
                            );
                        }
                        ui.end_row();

                        let mut kum = 0.0;
                        let mut degisti = false;
                        for i in 0..self.is_programi.dagilim.len() {
                            let (yil, ay) = self.is_programi.ay_etiketi(i);
                            ui.label(RichText::new(format!("{}", i + 1)).color(tema::METIN_SOLUK));
                            ui.label(
                                RichText::new(format!("{} {}", ay_adi(ay), yil))
                                    .size(12.5)
                                    .color(tema::METIN),
                            );
                            if ui
                                .add(
                                    egui::DragValue::new(&mut self.is_programi.dagilim[i])
                                        .speed(0.25)
                                        .range(0.0..=100.0)
                                        .suffix(" %"),
                                )
                                .changed()
                            {
                                degisti = true;
                            }
                            let yuzde = self.is_programi.dagilim[i];
                            kum += yuzde;
                            ui.label(
                                RichText::new(format!(
                                    "{} TL",
                                    para_formatla(toplam_bedel * yuzde / 100.0)
                                ))
                                .size(12.5)
                                .color(tema::METIN),
                            );
                            ui.label(
                                RichText::new(format!("% {:.2}", kum))
                                    .size(12.0)
                                    .color(tema::VURGU_HOVER),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{} TL",
                                    para_formatla(toplam_bedel * kum / 100.0)
                                ))
                                .size(12.5)
                                .color(tema::METIN_IKINCIL),
                            );
                            ui.end_row();
                        }
                        if degisti {
                            self.degisiklik_var = true;
                        }
                    });
            });
        });
    }

    /// İlerleme grafiği: aylık pursantaj çubukları + kümülatif S-eğrisi. Tek % ekseni (0 alt, 100 üst).
    fn is_programi_grafik(&self, ui: &mut Ui, _toplam_bedel: f64) {
        let n = self.is_programi.dagilim.len();
        if n == 0 {
            return;
        }

        let genislik = ui.available_width().min(900.0);
        let yukseklik = 220.0;
        let (yanit, painter) = ui.allocate_painter(egui::vec2(genislik, yukseklik), Sense::hover());
        let dis = yanit.rect;

        // Zemin
        painter.rect_filled(dis, CornerRadius::same(tema::KOSE), tema::YUZEY_2);
        painter.rect_stroke(
            dis,
            CornerRadius::same(tema::KOSE),
            Stroke::new(1.0, tema::KENAR),
            StrokeKind::Inside,
        );

        // Çizim alanı (eksen boşlukları)
        let sol = dis.left() + 44.0;
        let sag = dis.right() - 12.0;
        let ust = dis.top() + 14.0;
        let alt = dis.bottom() - 24.0;
        let ciz = Rect::from_min_max(Pos2::new(sol, ust), Pos2::new(sag, alt));
        let h = ciz.height();

        // Yatay ızgara + % etiketleri (0/25/50/75/100)
        for p in [0, 25, 50, 75, 100] {
            let y = alt - (p as f32 / 100.0) * h;
            painter.line_segment(
                [Pos2::new(sol, y), Pos2::new(sag, y)],
                Stroke::new(1.0, tema::KENAR_YUMUSAK),
            );
            painter.text(
                Pos2::new(sol - 6.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("%{}", p),
                FontId::proportional(10.0),
                tema::METIN_SOLUK,
            );
        }

        let adim = ciz.width() / n as f32;
        let cubuk_g = (adim * 0.6).min(46.0);

        // Aylık çubuklar
        for (i, yuzde) in self.is_programi.dagilim.iter().enumerate() {
            let orta = sol + adim * (i as f32 + 0.5);
            let yy = (*yuzde as f32 / 100.0) * h;
            let bar = Rect::from_min_max(
                Pos2::new(orta - cubuk_g / 2.0, alt - yy),
                Pos2::new(orta + cubuk_g / 2.0, alt),
            );
            painter.rect_filled(bar, CornerRadius::same(2), tema::VURGU.gamma_multiply(0.55));
            // Ay etiketi (yalnız 12 aya kadar sığar)
            if n <= 12 {
                let (_, ay) = self.is_programi.ay_etiketi(i);
                let kisa: String = ay_adi(ay).chars().take(3).collect();
                painter.text(
                    Pos2::new(orta, alt + 4.0),
                    egui::Align2::CENTER_TOP,
                    kisa,
                    FontId::proportional(9.5),
                    tema::METIN_SOLUK,
                );
            }
        }

        // Kümülatif S-eğrisi
        let mut kum = 0.0;
        let mut onceki: Option<Pos2> = None;
        for (i, yuzde) in self.is_programi.dagilim.iter().enumerate() {
            kum += *yuzde;
            let orta = sol + adim * (i as f32 + 0.5);
            let y = alt - (kum.min(100.0) as f32 / 100.0) * h;
            let nokta = Pos2::new(orta, y);
            if let Some(p) = onceki {
                painter.line_segment([p, nokta], Stroke::new(2.2, tema::BASARI));
            }
            painter.circle_filled(nokta, 3.2, tema::BASARI);
            onceki = Some(nokta);
        }
    }

    pub(crate) fn is_programi_excel_diyalog(&mut self) {
        let toplam_bedel = self.sozlesme_ayarlari.hesaplanan_sozlesme_bedeli();
        let prog = self.is_programi.clone();
        let proje_adi = self.metraj_adi.clone();
        let pb = self.proje_bilgi.clone();
        if let Some(d) = rfd::FileDialog::new()
            .add_filter("Excel", &["xlsx"])
            .set_file_name(format!("{} - Is Programi.xlsx", self.metraj_adi))
            .save_file()
        {
            match crate::export::is_programi_excel_aktar(&proje_adi, &pb, toplam_bedel, &prog, &d) {
                Ok(()) => self.basarili_mesaj = format!("İş programı Excel: {}", d.display()),
                Err(e) => self.hata_mesaji = e,
            }
        }
    }
}
