//! Birim fiyat analizi popup'ı: bir pozun (özellikle fiyatsız/özel pozların) fiyatını
//! rayiç girdilerinden (işçilik + malzeme + makine) analizle üretir. Ara toplam ×
//! (1 + kâr/genel gider) = birim fiyat. Kamu yaklaşık maliyetinde %25'in uygulandığı
//! yer burasıdır (kurum birim fiyatları değil).

use eframe::egui;
use egui::{RichText, ScrollArea, TextEdit};

use crate::bicim::{metni_kisalt, para_formatla, sayi_oku};
use crate::models::{analiz_ara_toplam, analiz_birim_fiyat, AnalizGirdisi, Poz};
use crate::tema;

use super::{AnalizGirdiSatiri, MetrajApp};

const TURLER: [&str; 3] = ["İşçilik", "Malzeme", "Makine"];

impl MetrajApp {
    /// Bir poz için analiz popup'ını açar; varsa kayıtlı analizi yükler.
    pub(crate) fn analiz_popup_ac(&mut self, poz: Poz) {
        let mut satirlar = Vec::new();
        if let Some(ref db) = self.db {
            if let Ok(girdiler) = db.analiz_getir(poz.kitap_id, &poz.poz_no) {
                satirlar = girdiler.iter().map(analiz_girdi_to_satir).collect();
            }
        }
        self.analiz_girdileri = satirlar;
        self.analiz_poz = Some(poz);
        self.analiz_rayic_arama.clear();
        self.analiz_rayic_sonuc.clear();
        self.analiz_popup_acik = true;
    }

    /// Analiz girdisi (rayiç) araması — tüm kitaplarda.
    pub(crate) fn analiz_rayic_ara(&mut self) {
        let sorgu = self.analiz_rayic_arama.trim().to_string();
        if sorgu.is_empty() {
            self.analiz_rayic_sonuc.clear();
            return;
        }
        if let Some(ref db) = self.db {
            let mut sonuc: Vec<Poz> = Vec::new();
            let poz_gibi = sorgu
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false);
            if poz_gibi {
                if let Ok(mut p) = db.poz_no_ara(&sorgu, None) {
                    sonuc.append(&mut p);
                }
            }
            if !poz_gibi || sonuc.len() < 15 {
                if let Ok(p) = db.tam_metin_ara(&sorgu, None) {
                    for x in p {
                        if !sonuc
                            .iter()
                            .any(|s| s.poz_no == x.poz_no && s.kitap_id == x.kitap_id)
                        {
                            sonuc.push(x);
                        }
                    }
                }
            }
            sonuc.truncate(40);
            self.analiz_rayic_sonuc = sonuc;
        }
    }

    pub(crate) fn render_analiz_popup(&mut self, ctx: &egui::Context) {
        if !self.analiz_popup_acik {
            return;
        }
        let poz = match self.analiz_poz.clone() {
            Some(p) => p,
            None => {
                self.analiz_popup_acik = false;
                return;
            }
        };

        let mut kapat = false;
        let mut poz_fiyati_yap = false;
        let mut yalniz_kaydet = false;
        let mut eklenecek_rayic: Option<Poz> = None;
        let mut ara = false;

        egui::Window::new("🧮 Birim Fiyat Analizi")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(740.0)
            .show(ctx, |ui| {
                // Başlık: analizi yapılan poz
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&poz.poz_no)
                            .monospace()
                            .strong()
                            .size(16.0)
                            .color(tema::METIN),
                    );
                    ui.label(
                        RichText::new(metni_kisalt(&poz.tanim, 60))
                            .size(13.0)
                            .color(tema::METIN_IKINCIL),
                    )
                    .on_hover_text(&poz.tanim);
                });
                ui.label(
                    RichText::new(format!(
                        "Birim: {}  ·  Kitap: {} ({}/{})",
                        poz.birim, poz.kitap_adi, poz.ay, poz.yil
                    ))
                    .size(11.5)
                    .color(tema::METIN_SOLUK),
                );
                ui.separator();

                // Rayiç (girdi) arama
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Girdi ara")
                            .size(12.0)
                            .color(tema::METIN_IKINCIL),
                    );
                    if ui
                        .add(
                            TextEdit::singleline(&mut self.analiz_rayic_arama)
                                .hint_text(tema::alan_ipucu("işçi, çimento, 10.100…"))
                                .desired_width(300.0),
                        )
                        .changed()
                    {
                        ara = true;
                    }
                    ui.label(
                        RichText::new("→ sonuca tıkla, girdi olarak eklenir")
                            .size(11.0)
                            .color(tema::METIN_SOLUK),
                    );
                });
                if !self.analiz_rayic_sonuc.is_empty() {
                    ScrollArea::vertical()
                        .id_salt("analiz_rayic_sonuc")
                        .max_height(110.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for r in self.analiz_rayic_sonuc.clone() {
                                let fm = r
                                    .fiyat
                                    .map(|f| format!("{} TL", para_formatla(f)))
                                    .unwrap_or_else(|| "—".into());
                                let etiket = format!(
                                    "{}  {}  · {} · {}",
                                    r.poz_no,
                                    metni_kisalt(&r.tanim, 44),
                                    r.birim,
                                    fm
                                );
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new(etiket).size(11.5))
                                            .fill(tema::YUZEY_2)
                                            .stroke(egui::Stroke::new(1.0, tema::KENAR_YUMUSAK)),
                                    )
                                    .clicked()
                                {
                                    eklenecek_rayic = Some(r);
                                }
                            }
                        });
                }
                ui.separator();

                // Girdi tablosu
                if self.analiz_girdileri.is_empty() {
                    ui.label(
                        RichText::new("Henüz girdi yok. Yukarıdan rayiç arayıp ekleyin.")
                            .color(tema::METIN_SOLUK)
                            .size(12.0),
                    );
                } else {
                    let mut sil: Option<usize> = None;
                    let bsl = |ui: &mut egui::Ui, t: &str| {
                        ui.label(
                            RichText::new(t)
                                .strong()
                                .size(11.5)
                                .color(tema::METIN_IKINCIL),
                        );
                    };
                    egui::Grid::new("analiz_grid")
                        .num_columns(8)
                        .spacing(egui::vec2(8.0, 6.0))
                        .striped(true)
                        .show(ui, |ui| {
                            bsl(ui, "Tür");
                            bsl(ui, "Girdi No");
                            bsl(ui, "Tanım");
                            bsl(ui, "Birim");
                            bsl(ui, "B.Fiyat");
                            bsl(ui, "Katsayı");
                            bsl(ui, "Tutar");
                            bsl(ui, "");
                            ui.end_row();
                            for (i, satir) in self.analiz_girdileri.iter_mut().enumerate() {
                                egui::ComboBox::from_id_salt(format!("analiz_tur_{}", i))
                                    .selected_text(&satir.tur)
                                    .width(78.0)
                                    .show_ui(ui, |ui| {
                                        for t in TURLER {
                                            if ui.selectable_label(satir.tur == t, t).clicked() {
                                                satir.tur = t.to_string();
                                            }
                                        }
                                    });
                                ui.label(
                                    RichText::new(&satir.girdi_no)
                                        .monospace()
                                        .size(11.0)
                                        .color(tema::METIN),
                                );
                                ui.label(
                                    RichText::new(metni_kisalt(&satir.tanim, 26))
                                        .size(11.0)
                                        .color(tema::METIN_IKINCIL),
                                )
                                .on_hover_text(&satir.tanim);
                                ui.label(
                                    RichText::new(&satir.birim)
                                        .size(11.0)
                                        .color(tema::METIN_SOLUK),
                                );
                                ui.label(
                                    RichText::new(para_formatla(satir.birim_fiyat))
                                        .size(11.0)
                                        .color(tema::METIN_IKINCIL),
                                );
                                ui.add(
                                    TextEdit::singleline(&mut satir.miktar_metni)
                                        .desired_width(62.0),
                                );
                                let tutar = sayi_oku(&satir.miktar_metni).unwrap_or(0.0)
                                    * satir.birim_fiyat;
                                ui.label(
                                    RichText::new(para_formatla(tutar))
                                        .size(11.0)
                                        .strong()
                                        .color(tema::BASARI),
                                );
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("🗑").color(tema::TEHLIKE).size(11.0),
                                        )
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::NONE),
                                    )
                                    .clicked()
                                {
                                    sil = Some(i);
                                }
                                ui.end_row();
                            }
                        });
                    if let Some(i) = sil {
                        self.analiz_girdileri.remove(i);
                    }
                }
                ui.separator();

                // Tür bazlı döküm + kâr + sonuç
                let girdiler: Vec<AnalizGirdisi> = self
                    .analiz_girdileri
                    .iter()
                    .filter_map(satir_to_analiz_girdi)
                    .collect();
                let ara_toplam = analiz_ara_toplam(&girdiler);
                let tur_top = |tur: &str| -> f64 {
                    girdiler
                        .iter()
                        .filter(|g| g.tur == tur)
                        .map(|g| g.tutar())
                        .sum()
                };
                ui.horizontal_wrapped(|ui| {
                    tema::rozet(
                        ui,
                        &format!("İşçilik: {} TL", para_formatla(tur_top("İşçilik"))),
                        tema::METIN_IKINCIL,
                    );
                    tema::rozet(
                        ui,
                        &format!("Malzeme: {} TL", para_formatla(tur_top("Malzeme"))),
                        tema::METIN_IKINCIL,
                    );
                    tema::rozet(
                        ui,
                        &format!("Makine: {} TL", para_formatla(tur_top("Makine"))),
                        tema::METIN_IKINCIL,
                    );
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Ara Toplam")
                            .size(12.0)
                            .color(tema::METIN_IKINCIL),
                    );
                    ui.label(
                        RichText::new(format!("{} TL", para_formatla(ara_toplam)))
                            .size(13.0)
                            .strong()
                            .color(tema::METIN),
                    );
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("Genel Gider + Kâr")
                            .size(12.0)
                            .color(tema::METIN_IKINCIL),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.analiz_kar_orani)
                            .speed(0.5)
                            .range(0.0..=100.0)
                            .suffix(" %"),
                    );
                });
                let sonuc = analiz_birim_fiyat(&girdiler, self.analiz_kar_orani);
                ui.add_space(2.0);
                egui::Frame::default()
                    .fill(tema::YUZEY_2)
                    .stroke(egui::Stroke::new(1.0, tema::KENAR))
                    .corner_radius(egui::CornerRadius::same(tema::KOSE))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("SONUÇ — {} birim fiyatı", poz.birim))
                                    .size(13.0)
                                    .strong()
                                    .color(tema::METIN),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!("{} TL", para_formatla(sonuc)))
                                            .size(17.0)
                                            .strong()
                                            .color(tema::BASARI),
                                    );
                                },
                            );
                        });
                    });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if tema::basari_buton(ui, "✓ Poz Fiyatı Yap")
                        .on_hover_text("Analizi kaydeder ve sonucu pozun birim fiyatı yapar")
                        .clicked()
                    {
                        poz_fiyati_yap = true;
                    }
                    if ui.button("💾 Yalnız Analizi Kaydet").clicked() {
                        yalniz_kaydet = true;
                    }
                    if ui.button("❌ İptal").clicked() {
                        kapat = true;
                    }
                });
            });

        if ara {
            self.analiz_rayic_ara();
        }

        if let Some(r) = eklenecek_rayic {
            self.analiz_girdileri.push(AnalizGirdiSatiri {
                girdi_no: r.poz_no.clone(),
                tanim: r.tanim.clone(),
                birim: r.birim.clone(),
                birim_fiyat: r.fiyat.unwrap_or(0.0),
                miktar_metni: "1".into(),
                tur: tur_tahmin(&r.tanim),
            });
        }

        if poz_fiyati_yap || yalniz_kaydet {
            let girdiler: Vec<AnalizGirdisi> = self
                .analiz_girdileri
                .iter()
                .filter_map(satir_to_analiz_girdi)
                .collect();
            let sonuc = analiz_birim_fiyat(&girdiler, self.analiz_kar_orani);
            if let Some(ref db) = self.db {
                if let Err(e) = db.analiz_kaydet(poz.kitap_id, &poz.poz_no, &girdiler) {
                    self.hata_mesaji = format!("Analiz kaydedilemedi: {}", e);
                } else if poz_fiyati_yap {
                    // Analiz sonucunu pozun en son dönemine (poz.yil/ay) yaz.
                    match db.poz_fiyat_guncelle(poz.kitap_id, &poz.poz_no, poz.yil, poz.ay, sonuc) {
                        Ok(()) => {
                            self.basarili_mesaj = format!(
                                "{} analizi kaydedildi; birim fiyat {} TL yapıldı.",
                                poz.poz_no,
                                para_formatla(sonuc)
                            )
                        }
                        Err(e) => self.hata_mesaji = format!("Fiyat güncellenemedi: {}", e),
                    }
                } else {
                    self.basarili_mesaj = format!(
                        "{} analizi kaydedildi ({} girdi).",
                        poz.poz_no,
                        girdiler.len()
                    );
                }
            }
            self.pozlar_tablosu_yenile();
            self.analiz_popup_acik = false;
        }
        if kapat {
            self.analiz_popup_acik = false;
        }
    }
}

// ==================== YARDIMCILAR ====================
fn analiz_girdi_to_satir(g: &AnalizGirdisi) -> AnalizGirdiSatiri {
    AnalizGirdiSatiri {
        girdi_no: g.girdi_no.clone(),
        tanim: g.tanim.clone(),
        birim: g.birim.clone(),
        birim_fiyat: g.birim_fiyat,
        miktar_metni: format!("{}", g.miktar).replace('.', ","),
        tur: g.tur.clone(),
    }
}

fn satir_to_analiz_girdi(s: &AnalizGirdiSatiri) -> Option<AnalizGirdisi> {
    if s.girdi_no.is_empty() {
        return None;
    }
    let miktar = sayi_oku(&s.miktar_metni)?;
    Some(AnalizGirdisi {
        girdi_no: s.girdi_no.clone(),
        tanim: s.tanim.clone(),
        birim: s.birim.clone(),
        birim_fiyat: s.birim_fiyat,
        miktar,
        tur: if s.tur.is_empty() {
            "Malzeme".to_string()
        } else {
            s.tur.clone()
        },
    })
}

/// Tanımdan kaba tür tahmini (kullanıcı combobox'tan değiştirebilir).
fn tur_tahmin(tanim: &str) -> String {
    let t = tanim.to_lowercase();
    if t.contains("işçi") || t.contains("usta") || t.contains("saat") || t.contains("yevmiye") {
        "İşçilik".to_string()
    } else if t.contains("makine")
        || t.contains("ekskavatör")
        || t.contains("vinç")
        || t.contains("kamyon")
        || t.contains("kompresör")
    {
        "Makine".to_string()
    } else {
        "Malzeme".to_string()
    }
}
