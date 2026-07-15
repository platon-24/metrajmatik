//! Proje sekmesi (workflow adım 1 — PROJE KUR): projenin idari künyesi (idare adı,
//! işin adı, İKN, iş yeri, sözleşme), hesap kipi (Kamu/Özel) ve hızlı özet panosu.
//! Künye, resmî çıktıların (yaklaşık maliyet / hakediş / teklif) başlığına akar.

use eframe::egui;
use egui::{RichText, TextEdit, Ui};

use crate::bicim::para_formatla;
use crate::models::{HesapTuru, ProjeAsamasi};
use crate::tema;

use super::{MetrajApp, MetrajPaneli, Sekme};

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

impl MetrajApp {
    pub(crate) fn render_proje(&mut self, ui: &mut Ui) {
        ui.set_max_width(1260.0);
        let kalem_sayisi: usize = self
            .is_gruplari
            .iter()
            .map(|grup| grup.tum_kalemler_duz().len())
            .sum();
        let adimlar = [
            self.proje_bilgi.dolu_mu(),
            self.poz_sayisi > 0,
            !self.is_gruplari.is_empty(),
            kalem_sayisi > 0,
        ];
        let tamamlanan = adimlar.iter().filter(|tamam| **tamam).count();

        tema::sayfa_basligi(
            ui,
            "Proje kontrol merkezi",
            if self.metraj_adi.trim().is_empty() {
                "Yeni projenizi hazırlayın"
            } else {
                &self.metraj_adi
            },
            "Künye, maliyet verileri ve sözleşme akışının tek bakışta özeti.",
        );

        tema::vurgu_karti(ui, |ui| {
            let ilerleme_genisligi = ui.available_width();
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(if self.proje_asamasi == ProjeAsamasi::Hakedis {
                            "Hakediş çalışma alanı aktif"
                        } else if tamamlanan == adimlar.len() {
                            "Metrajınız sözleşmeye hazır"
                        } else {
                            "Kurulumu tamamlayın"
                        })
                        .strong()
                        .size(18.0),
                    );
                    ui.label(
                        RichText::new(if self.proje_asamasi == ProjeAsamasi::Hakedis {
                            "Metraj donduruldu; sözleşme, ödeme ve iş programı birlikte ilerler."
                                .to_owned()
                        } else {
                            format!(
                                "{} / {} hazırlık adımı tamamlandı",
                                tamamlanan,
                                adimlar.len()
                            )
                        })
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                    );
                });
                ui.add_space(20.0);
                let etiket = if self.proje_asamasi == ProjeAsamasi::Hakedis {
                    "Hakedişe devam et  >"
                } else {
                    "Metraja devam et  >"
                };
                if tema::birincil_buton(ui, etiket).clicked() {
                    self.sekme_ac(if self.proje_asamasi == ProjeAsamasi::Hakedis {
                        Sekme::Hakedis
                    } else {
                        Sekme::MetrajTablosu
                    });
                }
            });
            ui.add_space(12.0);
            ui.add(
                egui::ProgressBar::new(tamamlanan as f32 / adimlar.len() as f32)
                    .desired_width(ilerleme_genisligi)
                    .text(format!("Hazırlık  ·  %{}", tamamlanan * 25)),
            );
        });
        ui.add_space(12.0);

        tema::bolum_basligi(ui, "↗", "Proje Akışı");
        tema::kart(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let _ = baslangic_adimi(
                    ui,
                    1,
                    "Proje bilgileri",
                    "İdare ve iş adını girin",
                    adimlar[0],
                );
                if baslangic_adimi(
                    ui,
                    2,
                    "Fiyat kaynağı",
                    "Kitap ve dönemleri yönetin",
                    adimlar[1],
                ) {
                    self.sekme_ac(Sekme::KitapYoneticisi);
                }
                if baslangic_adimi(
                    ui,
                    3,
                    "İş grupları",
                    "Metraj ağacını düzenleyin",
                    adimlar[2],
                ) {
                    self.dar_metraj_paneli = MetrajPaneli::IsGruplari;
                    self.sekme_ac(Sekme::MetrajTablosu);
                }
                if baslangic_adimi(
                    ui,
                    4,
                    "Metraja başlayın",
                    "Poz bulun ve miktar girin",
                    adimlar[3],
                ) {
                    self.dar_metraj_paneli = MetrajPaneli::PozAra;
                    self.sekme_ac(Sekme::MetrajTablosu);
                }
            });
        });
        ui.add_space(16.0);

        tema::bolum_basligi(ui, "•", "Proje Künyesi");
        ui.label(
            RichText::new(
                "Bu bilgiler yaklaşık maliyet, hakediş ve teklif çıktılarının başlığına aktarılır.",
            )
            .color(tema::METIN_SOLUK)
            .size(11.5),
        );
        ui.add_space(8.0);

        // Üst eylem çubuğu: proje adı + kaydet/aç/excel
        tema::kart(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
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
                if tema::birincil_buton(ui, "Kaydet").clicked() {
                    self.metraj_kaydet();
                }
                if tema::ikincil_buton(ui, "Proje Aç").clicked() {
                    self.metraj_yukle_diyalog();
                }
                if tema::ikincil_buton(ui, "Excel'e Aktar").clicked() {
                    self.metraj_excel_diyalog();
                }
            });
        });
        ui.add_space(8.0);

        // Temel proje bilgileri
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
        ui.add_space(8.0);
        tema::kart(ui, |ui| {
            egui::CollapsingHeader::new("Sözleşme ve hakediş bilgileri")
                .default_open(
                    !self.proje_bilgi.yuklenici.is_empty()
                        || !self.proje_bilgi.sozlesme_no.is_empty(),
                )
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Hakediş aşamasına geçtiğinizde doldurabilirsiniz.")
                            .color(tema::METIN_SOLUK)
                            .size(11.0),
                    );
                    ui.add_space(6.0);
                    egui::Grid::new("proje_sozlesme_grid")
                        .num_columns(2)
                        .spacing(egui::vec2(14.0, 8.0))
                        .show(ui, |ui| {
                            degisti |= kunye_alan(
                                ui,
                                "Yüklenici",
                                &mut self.proje_bilgi.yuklenici,
                                "Firma / yüklenici adı",
                            );
                            degisti |= kunye_alan(
                                ui,
                                "Sözleşme No",
                                &mut self.proje_bilgi.sozlesme_no,
                                "",
                            );
                            degisti |= kunye_alan(
                                ui,
                                "Sözleşme Tarihi",
                                &mut self.proje_bilgi.sozlesme_tarihi,
                                "gg.aa.yyyy",
                            );
                        });
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

fn baslangic_adimi(ui: &mut Ui, sira: usize, baslik: &str, aciklama: &str, tamam: bool) -> bool {
    let durum = if tamam {
        "OK".to_owned()
    } else {
        sira.to_string()
    };
    ui.add(
        egui::Button::new(
            RichText::new(format!("{}  {}\n    {}", durum, baslik, aciklama))
                .size(12.0)
                .color(if tamam { tema::BASARI } else { tema::METIN }),
        )
        .min_size(egui::vec2(198.0, 62.0))
        .fill(if tamam {
            tema::BASARI_KOYU
        } else {
            tema::YUZEY_3
        })
        .stroke(egui::Stroke::new(
            1.0,
            if tamam { tema::BASARI } else { tema::KENAR },
        )),
    )
    .clicked()
}

/// Küçük etiket-değer özet kutusu (pano rozetleri).
fn ozet_kutu(ui: &mut Ui, etiket: &str, deger: &str, renk: egui::Color32) {
    egui::Frame::default()
        .fill(tema::YUZEY_3)
        .stroke(egui::Stroke::new(1.0, tema::KENAR_YUMUSAK))
        .corner_radius(egui::CornerRadius::same(tema::KOSE))
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(etiket).color(tema::METIN_SOLUK).size(11.0));
                ui.label(RichText::new(deger).color(renk).strong().size(16.0));
            });
        });
}
