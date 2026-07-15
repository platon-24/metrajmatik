//! Proje sekmesi (workflow adım 1 — PROJE KUR): projenin idari künyesi (idare adı,
//! işin adı, İKN, iş yeri, sözleşme), hesap kipi (Kamu/Özel) ve hızlı özet panosu.
//! Künye, resmî çıktıların (yaklaşık maliyet / hakediş / teklif) başlığına akar.

use eframe::egui;
use egui::{RichText, TextEdit, Ui};

use crate::bicim::para_formatla;
use crate::models::HesapTuru;
use crate::tema;

/// Etiket + tek satır giriş (Grid içinde bir satır). Değişiklik olduysa true döner.
fn kunye_alan(ui: &mut Ui, etiket: &str, deger: &mut String, ipucu: &str) -> bool {
    ui.label(RichText::new(etiket).color(tema::METIN_IKINCIL).size(12.5));
    let degisti = ui
        .add(
            TextEdit::singleline(deger)
                .hint_text(ipucu)
                .desired_width(440.0),
        )
        .changed();
    ui.end_row();
    degisti
}

impl super::MetrajApp {
    pub(crate) fn render_proje(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "📁", "Proje Künyesi");
        ui.label(RichText::new("Resmî çıktıların (yaklaşık maliyet cetveli, hakediş, teklif) başlığında bu bilgiler yer alır.").color(tema::METIN_SOLUK).size(11.5));
        ui.add_space(8.0);

        // Üst eylem çubuğu: proje adı + kaydet/aç/excel
        tema::kart(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Proje Adı")
                        .color(tema::METIN_IKINCIL)
                        .size(12.5),
                );
                if ui
                    .add(
                        TextEdit::singleline(&mut self.metraj_adi)
                            .hint_text("Proje / dosya adı")
                            .desired_width(300.0),
                    )
                    .changed()
                {
                    self.degisiklik_var = true;
                }
                ui.add_space(12.0);
                if tema::basari_buton(ui, "💾 Kaydet").clicked() {
                    self.metraj_kaydet();
                }
                if ui.button("📂 Aç").clicked() {
                    self.metraj_yukle_diyalog();
                }
                if ui.button("⬇ Excel").clicked() {
                    self.metraj_excel_diyalog();
                }
            });
        });
        ui.add_space(8.0);

        // Künye formu
        let mut degisti = false;
        tema::kart(ui, |ui| {
            egui::Grid::new("proje_kunye_grid")
                .num_columns(2)
                .spacing(egui::vec2(14.0, 8.0))
                .show(ui, |ui| {
                    degisti |= kunye_alan(
                        ui,
                        "İdarenin Adı",
                        &mut self.proje_bilgi.idare_adi,
                        "örn: … Belediyesi / Genel Müdürlüğü",
                    );
                    degisti |= kunye_alan(
                        ui,
                        "İşin Adı",
                        &mut self.proje_bilgi.is_adi,
                        "örn: 24 Derslikli Okul İnşaatı",
                    );
                    degisti |=
                        kunye_alan(ui, "İşin Yeri", &mut self.proje_bilgi.is_yeri, "il / ilçe");
                    degisti |= kunye_alan(
                        ui,
                        "İhale Kayıt No (İKN)",
                        &mut self.proje_bilgi.ihale_kayit_no,
                        "örn: 2026/123456",
                    );

                    // İşin türü (Yapım / Hizmet / Mal)
                    ui.label(
                        RichText::new("İşin Türü")
                            .color(tema::METIN_IKINCIL)
                            .size(12.5),
                    );
                    let gosterim = if self.proje_bilgi.is_turu.is_empty() {
                        "—".to_string()
                    } else {
                        self.proje_bilgi.is_turu.clone()
                    };
                    egui::ComboBox::from_id_salt("proje_is_turu")
                        .selected_text(&gosterim)
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for t in ["Yapım", "Hizmet", "Mal"] {
                                if ui
                                    .selectable_label(self.proje_bilgi.is_turu == t, t)
                                    .clicked()
                                {
                                    self.proje_bilgi.is_turu = t.into();
                                    degisti = true;
                                }
                            }
                        });
                    ui.end_row();

                    degisti |= kunye_alan(
                        ui,
                        "Yüklenici",
                        &mut self.proje_bilgi.yuklenici,
                        "sözleşme/hakediş aşamasında",
                    );
                    degisti |= kunye_alan(ui, "Sözleşme No", &mut self.proje_bilgi.sozlesme_no, "");
                    degisti |= kunye_alan(
                        ui,
                        "Sözleşme Tarihi",
                        &mut self.proje_bilgi.sozlesme_tarihi,
                        "gg.aa.yyyy",
                    );
                });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Hesap kipi (Kamu/Özel) — İcmal ile aynı alanı düzenler
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Hesap Kipi")
                        .color(tema::METIN_IKINCIL)
                        .size(12.5),
                );
                if ui
                    .selectable_label(self.hesap_turu == HesapTuru::Kamu, "🏛 Kamu (KDV hariç)")
                    .clicked()
                {
                    self.hesap_turu = HesapTuru::Kamu;
                    self.degisiklik_var = true;
                }
                if ui
                    .selectable_label(self.hesap_turu == HesapTuru::Ozel, "🏢 Özel (KDV dahil)")
                    .clicked()
                {
                    self.hesap_turu = HesapTuru::Ozel;
                    self.degisiklik_var = true;
                }
            });
        });
        if degisti {
            self.degisiklik_var = true;
        }
        ui.add_space(10.0);

        // Özet panosu
        let toplam = self.toplam_tutar();
        let grup_say = self.is_gruplari.len();
        let hakedis_say = self.hakedisler.len();
        let sure = self.is_programi.sure_ay;
        tema::bolum_basligi(ui, "📊", "Özet");
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ozet_kutu(
                ui,
                "Genel Toplam",
                &format!("{} TL", para_formatla(toplam)),
                tema::BASARI,
            );
            ozet_kutu(ui, "İş Grubu", &format!("{}", grup_say), tema::VURGU_HOVER);
            ozet_kutu(
                ui,
                "Hakediş",
                &format!("{}", hakedis_say),
                tema::VURGU_HOVER,
            );
            ozet_kutu(
                ui,
                "İş Programı",
                &format!("{} ay", sure),
                tema::VURGU_HOVER,
            );
            let kip = if self.hesap_turu == HesapTuru::Kamu {
                "Kamu (KDV hariç)"
            } else {
                "Özel (KDV dahil)"
            };
            ozet_kutu(ui, "Hesap Kipi", kip, tema::METIN_IKINCIL);
        });

        if !self.proje_bilgi.dolu_mu() {
            ui.add_space(10.0);
            tema::bildirim_seridi(ui, "İpucu: Künyeyi doldurun — İdarenin adı ve İşin adı resmî Excel çıktılarının başlığında görünür.", tema::UYARI_KOYU, tema::UYARI, tema::UYARI);
        }
    }
}

/// Küçük etiket-değer özet kutusu (pano rozetleri).
fn ozet_kutu(ui: &mut Ui, etiket: &str, deger: &str, renk: egui::Color32) {
    egui::Frame::default()
        .fill(tema::YUZEY_2)
        .stroke(egui::Stroke::new(1.0, tema::KENAR))
        .corner_radius(egui::CornerRadius::same(tema::KOSE))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(etiket).color(tema::METIN_SOLUK).size(11.0));
                ui.label(RichText::new(deger).color(renk).strong().size(16.0));
            });
        });
}
