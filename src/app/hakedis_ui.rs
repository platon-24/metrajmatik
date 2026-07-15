//! Hakediş sekmesi: sözleşme (keşif) kalemleri üzerinden kümülatif imalat miktarları
//! (yeşil defter), bu hakediş tutarları, kesintiler ve net ödeme. Excel raporu.

use eframe::egui;
use egui::{RichText, ScrollArea, TextEdit, Ui};
use std::collections::HashMap;

use crate::bicim::{krono_tarih, metni_kisalt, para_formatla};
use crate::hakedis::{icmal, poz_hesaplari};
use crate::models::{
    FiyatFarkiYontemi, Hakedis, HakedisSatiri, MetrajKalemi, MiktarDetay, ProjeAsamasi,
    TenzilatYontemi,
};
use crate::tema;

use super::gorunum_metraj::{detay_to_satir, satir_miktar, satir_to_detay};
use super::{MetrajApp, PopupDetaySatiri};

impl MetrajApp {
    /// Sözleşme/keşif kalemleri: iş gruplarının düzleştirilmiş tüm kalemleri.
    fn kesif_kalemleri(&mut self) -> Vec<MetrajKalemi> {
        self.aktif_grubu_senkronize();
        if self.is_gruplari.is_empty() {
            self.metraj_kalemleri.clone()
        } else {
            let mut v = Vec::new();
            for g in &self.is_gruplari {
                v.extend(g.tum_kalemler_duz());
            }
            v
        }
    }

    /// Yeni hakediş: önceki hakedişin kümülatiflerini devralır (kümülatif hep artar).
    fn hakedis_yeni(&mut self) {
        let kesif = self.kesif_kalemleri();
        let onceki = self.hakedisler.last();
        let satirlar: Vec<HakedisSatiri> = kesif
            .iter()
            .map(|k| HakedisSatiri {
                kalem_id: k.id.clone(),
                poz_no: k.poz_no.clone(),
                kumulatif_miktar: onceki.map(|h| h.kumulatif(&k.id, &k.poz_no)).unwrap_or(0.0),
                detaylar: vec![],
            })
            .collect();
        let no = self.hakedisler.len() as u32 + 1;
        let tur = if self.hakedisler.is_empty() {
            "İlk"
        } else {
            "Ara"
        };
        let mut h = Hakedis::yeni(no, tur, krono_tarih());
        h.satirlar = satirlar;
        self.hakedisler.push(h);
        self.secili_hakedis = Some(self.hakedisler.len() - 1);
        self.degisiklik_var = true;
    }

    /// Seçili hakedişin satırlarını güncel keşifle hizalar (poz eklenmiş/silinmişse).
    fn hakedis_hizala(&mut self, idx: usize, kesif: &[MetrajKalemi]) {
        if let Some(h) = self.hakedisler.get_mut(idx) {
            let eski: HashMap<String, HakedisSatiri> = h
                .satirlar
                .drain(..)
                .filter(|s| !s.kalem_id.is_empty())
                .map(|s| (s.kalem_id.clone(), s))
                .collect();
            h.satirlar = kesif
                .iter()
                .map(|k| {
                    eski.get(&k.id).cloned().unwrap_or(HakedisSatiri {
                        kalem_id: k.id.clone(),
                        poz_no: k.poz_no.clone(),
                        kumulatif_miktar: 0.0,
                        detaylar: vec![],
                    })
                })
                .collect();
        }
    }

    fn render_hakedise_donusum(&mut self, ui: &mut Ui) {
        let kesif_bedeli = self.toplam_tutar();
        ui.horizontal_wrapped(|ui| {
            tema::istatistik(
                ui,
                "Keşif bedeli",
                &format!("{} TL", para_formatla(kesif_bedeli)),
                "Dönüşümde dondurulur",
                tema::BASARI,
            );
            tema::istatistik(
                ui,
                "Metraj kalemi",
                &self.kesif_kalemleri().len().to_string(),
                "Sözleşme kapsamı",
                tema::VURGU_HOVER,
            );
            tema::istatistik(
                ui,
                "İş programı",
                "Hazır değil",
                "Dönüşümle etkinleşir",
                tema::AKSAN,
            );
        });
        ui.add_space(10.0);
        tema::bildirim_seridi(
            ui,
            "Hakediş henüz etkin değil. Metrajı tamamlayın, sözleşme bilgilerini girin ve bilinçli olarak dönüştürün.",
            tema::UYARI_KOYU,
            tema::UYARI,
            tema::UYARI,
        );
        ui.add_space(10.0);
        tema::kart(ui, |ui| {
            ui.label(RichText::new("Sözleşme bilgileri").strong().size(15.0));
            ui.label(
                RichText::new("Dönüşümden sonra metraj ve iş grupları salt okunur olur; İş Programı etkinleşir.")
                    .color(tema::METIN_SOLUK)
                    .size(11.5),
            );
            ui.add_space(8.0);
            egui::Grid::new("hakedis_donusum_kunye")
                .num_columns(2)
                .spacing(egui::vec2(14.0, 8.0))
                .show(ui, |ui| {
                    for (etiket, deger, ipucu) in [
                        (
                            "Yüklenici / Şirket",
                            &mut self.proje_bilgi.yuklenici,
                            "Firma unvanı",
                        ),
                        (
                            "Sözleşme No",
                            &mut self.proje_bilgi.sozlesme_no,
                            "Sözleşme numarası",
                        ),
                        (
                            "Sözleşme Tarihi",
                            &mut self.proje_bilgi.sozlesme_tarihi,
                            "gg.aa.yyyy",
                        ),
                    ] {
                        ui.label(RichText::new(etiket).color(tema::METIN_IKINCIL));
                        if ui
                            .add(
                                TextEdit::singleline(deger)
                                    .hint_text(ipucu)
                                    .desired_width(320.0),
                            )
                            .changed()
                        {
                            self.degisiklik_var = true;
                        }
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(RichText::new("Tenzilat yöntemi").strong());
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(
                        self.sozlesme_ayarlari.tenzilat_yontemi == TenzilatYontemi::ManuelOran,
                        "Manuel oran",
                    )
                    .clicked()
                {
                    self.sozlesme_ayarlari.tenzilat_yontemi = TenzilatYontemi::ManuelOran;
                    self.degisiklik_var = true;
                }
                if ui
                    .selectable_label(
                        self.sozlesme_ayarlari.tenzilat_yontemi
                            == TenzilatYontemi::SozlesmeBedelinden,
                        "Sözleşme bedelinden hesapla",
                    )
                    .clicked()
                {
                    self.sozlesme_ayarlari.tenzilat_yontemi = TenzilatYontemi::SozlesmeBedelinden;
                    self.degisiklik_var = true;
                }
            });
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "Dönüşüm keşif bedeli: {} TL",
                    para_formatla(kesif_bedeli)
                ));
                match self.sozlesme_ayarlari.tenzilat_yontemi {
                    TenzilatYontemi::ManuelOran => {
                        ui.label("Tenzilat (%):");
                        if ui
                            .add(
                                egui::DragValue::new(
                                    &mut self.sozlesme_ayarlari.manuel_tenzilat_orani,
                                )
                                .range(0.0..=100.0)
                                .speed(0.0001)
                                .fixed_decimals(6),
                            )
                            .changed()
                        {
                            self.degisiklik_var = true;
                        }
                        let bedel = kesif_bedeli
                            * (1.0 - self.sozlesme_ayarlari.manuel_tenzilat_orani / 100.0);
                        ui.label(format!("Sözleşme bedeli: {} TL", para_formatla(bedel)));
                    }
                    TenzilatYontemi::SozlesmeBedelinden => {
                        ui.label("Sözleşme bedeli (TL):");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.sozlesme_ayarlari.sozlesme_bedeli)
                                    .range(0.0..=f64::INFINITY)
                                    .speed(100.0)
                                    .fixed_decimals(2),
                            )
                            .changed()
                        {
                            self.degisiklik_var = true;
                        }
                        let oran = if kesif_bedeli > 0.0 {
                            (1.0 - self.sozlesme_ayarlari.sozlesme_bedeli / kesif_bedeli) * 100.0
                        } else {
                            0.0
                        };
                        ui.label(format!("Hesaplanan tenzilat: % {:.6}", oran));
                    }
                }
            });
            ui.label(
                RichText::new(
                    "Oran 6 ondalık hanede saklanır; TL sonuçları en son kuruşa yuvarlanır.",
                )
                .color(tema::METIN_SOLUK)
                .size(11.0),
            );
            ui.add_space(10.0);
            if tema::birincil_buton(ui, "Hakedişe Dönüştür").clicked() {
                self.hakedise_donusum_onayi = true;
            }
        });

        if self.hakedise_donusum_onayi {
            self.render_hakedise_donusum_onayi(ui.ctx(), kesif_bedeli);
        }
    }

    fn render_hakedise_donusum_onayi(&mut self, ctx: &egui::Context, kesif_bedeli: f64) {
        let mut kopyala_donustur = false;
        let mut donustur = false;
        let mut vazgec = false;
        egui::Window::new("Hakedişe dönüştürme onayı")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(540.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Bu işlem proje çalışma biçimini kalıcı olarak değiştirir.")
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label("• Metraj girişi, poz ve iş grubu düzenleme kilitlenecek.");
                ui.label("• Dönüşüm anındaki keşif bedeli sözleşme bazı olarak dondurulacak.");
                ui.label("• İş Programı ve hakediş girişi etkinleşecek.");
                ui.label("• Dönüşüm otomatik olarak ilk hakedişi oluşturmayacak.");
                ui.add_space(8.0);
                tema::bildirim_seridi(
                    ui,
                    "Önerilen seçenek, önce düzenlenebilir metrajın ayrı bir kopyasını kaydeder.",
                    tema::UYARI_KOYU,
                    tema::UYARI,
                    tema::UYARI,
                );
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    if tema::basari_buton(ui, "Kopya Kaydet ve Dönüştür").clicked() {
                        kopyala_donustur = true;
                    }
                    if tema::tehlike_buton(ui, "Doğrudan Dönüştür").clicked() {
                        donustur = true;
                    }
                    if ui.button("Vazgeç").clicked() {
                        vazgec = true;
                    }
                });
            });

        if vazgec {
            self.hakedise_donusum_onayi = false;
            return;
        }
        if kopyala_donustur && !self.proje_kopyasi_kaydet() {
            return;
        }
        if kopyala_donustur || donustur {
            if let Err(e) = self.hakedise_donustur(kesif_bedeli) {
                self.hata_mesaji = e;
            }
        }
    }

    fn hakedise_donustur(&mut self, kesif_bedeli: f64) -> Result<(), String> {
        if kesif_bedeli <= 0.0 {
            return Err("Dönüşüm için metraj toplamı sıfırdan büyük olmalı.".into());
        }
        if self.proje_bilgi.yuklenici.trim().is_empty()
            || self.proje_bilgi.sozlesme_no.trim().is_empty()
            || self.proje_bilgi.sozlesme_tarihi.trim().is_empty()
        {
            return Err("Yüklenici, sözleşme no ve sözleşme tarihini doldurun.".into());
        }
        self.sozlesme_ayarlari.kesif_bedeli = kesif_bedeli;
        let oran = self.sozlesme_ayarlari.tenzilat_orani();
        if !(0.0..=100.0).contains(&oran) {
            return Err(
                "Tenzilat oranı %0 ile %100 arasında olmalı; sözleşme bedelini kontrol edin."
                    .into(),
            );
        }
        self.sozlesme_ayarlari.manuel_tenzilat_orani =
            (self.sozlesme_ayarlari.manuel_tenzilat_orani * 1_000_000.0).round() / 1_000_000.0;
        self.sozlesme_ayarlari.sozlesme_bedeli =
            self.sozlesme_ayarlari.hesaplanan_sozlesme_bedeli();
        self.sozlesme_ayarlari.donusum_tarihi = krono_tarih();
        self.proje_asamasi = ProjeAsamasi::Hakedis;
        self.is_programi.normalize();
        self.geri_al_yigini.clear();
        self.yinele_yigini.clear();
        self.hakedise_donusum_onayi = false;
        self.degisiklik_var = true;
        self.basarili_mesaj =
            "Proje hakediş aşamasına dönüştürüldü. Metraj kilitlendi; İş Programı etkin.".into();
        Ok(())
    }

    fn render_sozlesme_ozeti(&mut self, ui: &mut Ui) {
        tema::kart(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("🔒 Metraj donduruldu")
                        .color(tema::UYARI)
                        .strong(),
                );
                ui.separator();
                ui.label(format!("Yüklenici: {}", self.proje_bilgi.yuklenici));
                ui.label(format!(
                    "Sözleşme: {} · {}",
                    self.proje_bilgi.sozlesme_no, self.proje_bilgi.sozlesme_tarihi
                ));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "Keşif: {} TL",
                    para_formatla(self.sozlesme_ayarlari.kesif_bedeli)
                ));
                ui.label(format!(
                    "Tenzilat: % {:.6}",
                    self.sozlesme_ayarlari.tenzilat_orani()
                ));
                ui.label(
                    RichText::new(format!(
                        "Sözleşme: {} TL",
                        para_formatla(self.sozlesme_ayarlari.hesaplanan_sozlesme_bedeli())
                    ))
                    .color(tema::BASARI)
                    .strong(),
                );
            });
        });
    }

    pub(crate) fn render_hakedis(&mut self, ui: &mut Ui) {
        tema::sayfa_basligi(
            ui,
            if self.proje_asamasi == ProjeAsamasi::Metraj {
                "Sözleşmeye geçiş"
            } else {
                "Ödeme çalışma alanı"
            },
            "Hakediş",
            if self.proje_asamasi == ProjeAsamasi::Metraj {
                "Tamamlanan metrajı sözleşmeye bağlayın ve kontrollü biçimde hakedişe dönüştürün."
            } else {
                "İmalat ilerlemesi, kesintiler, fiyat farkı ve net ödemeyi birlikte yönetin."
            },
        );

        let kesif = self.kesif_kalemleri();
        if kesif.is_empty() {
            tema::bildirim_seridi(ui, "Önce Metraj sekmesinden sözleşme (keşif) kalemlerini girin. Hakediş bunların üzerine kurulur.", tema::UYARI_KOYU, tema::UYARI, tema::UYARI);
            return;
        }

        if self.proje_asamasi == ProjeAsamasi::Metraj {
            self.render_hakedise_donusum(ui);
            return;
        }

        self.render_sozlesme_ozeti(ui);
        ui.add_space(8.0);

        // Hakediş listesi + yeni + excel + sil
        let mut secilecek: Option<usize> = None;
        let mut yeni = false;
        let mut excel = false;
        let mut sil: Option<usize> = None;
        tema::kart(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Hakedişler:")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                for (i, h) in self.hakedisler.iter().enumerate() {
                    let secili = self.secili_hakedis == Some(i);
                    if ui
                        .selectable_label(secili, format!("{}. {} ({})", h.no, h.tur, h.tarih))
                        .clicked()
                    {
                        secilecek = Some(i);
                    }
                }
                if tema::birincil_ikonlu_buton(ui, tema::ikon::YENI_HAKEDIS, "Yeni Hakediş")
                    .clicked()
                {
                    yeni = true;
                }
                if self.secili_hakedis.is_some() {
                    if ui.button("📊 Excel").clicked() {
                        excel = true;
                    }
                    if tema::tehlike_buton(ui, "🗑 Sil").clicked() {
                        sil = self.secili_hakedis;
                    }
                }
            });
        });
        if yeni {
            self.hakedis_yeni();
        }
        if let Some(i) = secilecek {
            self.secili_hakedis = Some(i);
        }
        if let Some(i) = sil {
            self.hakedisler.remove(i);
            for (j, h) in self.hakedisler.iter_mut().enumerate() {
                h.no = j as u32 + 1;
            }
            self.secili_hakedis = if self.hakedisler.is_empty() {
                None
            } else {
                Some(self.hakedisler.len() - 1)
            };
            self.degisiklik_var = true;
        }
        if excel {
            self.hakedis_excel_diyalog();
        }

        let idx = match self.secili_hakedis {
            Some(i) if i < self.hakedisler.len() => i,
            _ => {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Bir hakediş seçin veya 'Yeni Hakediş' ile oluşturun.")
                        .color(tema::METIN_SOLUK),
                );
                return;
            }
        };

        self.hakedis_hizala(idx, &kesif);
        self.hakedisler[idx].fiyat_farki_ayari.normalize();

        let onceki_kum: HashMap<String, f64> = if idx > 0 {
            self.hakedisler[idx - 1]
                .satirlar
                .iter()
                .map(|s| (s.kalem_id.clone(), s.kumulatif_miktar))
                .collect()
        } else {
            HashMap::new()
        };

        ui.add_space(8.0);
        // Tür / tarih / kesinti oranları
        let mut ayar_degisti = false;
        tema::kart(ui, |ui| {
            let h = &mut self.hakedisler[idx];
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("Hakediş No: {}", h.no))
                        .strong()
                        .color(tema::METIN),
                );
                ui.add_space(10.0);
                ui.label(RichText::new("Tür").color(tema::METIN_IKINCIL).size(12.0));
                for t in ["İlk", "Ara", "Kesin"] {
                    if ui.selectable_label(h.tur == t, t).clicked() && h.tur != t {
                        h.tur = t.to_string();
                        ayar_degisti = true;
                    }
                }
                ui.add_space(10.0);
                ui.label(RichText::new("Tarih").color(tema::METIN_IKINCIL).size(12.0));
                ayar_degisti |= ui
                    .add(TextEdit::singleline(&mut h.tarih).desired_width(100.0))
                    .changed();
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Damga (‰)")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                ayar_degisti |= ui
                    .add(
                        egui::DragValue::new(&mut h.damga_orani)
                            .speed(0.1)
                            .range(0.0..=100.0),
                    )
                    .changed();
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Teminat (%)")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                ayar_degisti |= ui
                    .add(
                        egui::DragValue::new(&mut h.teminat_orani)
                            .speed(0.1)
                            .range(0.0..=100.0),
                    )
                    .changed();
                ui.add_space(12.0);
                ui.label(
                    RichText::new("SGK (%)")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                ayar_degisti |= ui
                    .add(
                        egui::DragValue::new(&mut h.sgk_orani)
                            .speed(0.1)
                            .range(0.0..=100.0),
                    )
                    .changed();
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Avans mahsubu (TL)")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                ayar_degisti |= ui
                    .add(
                        egui::DragValue::new(&mut h.avans_mahsup)
                            .speed(10.0)
                            .range(0.0..=f64::INFINITY),
                    )
                    .changed();
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Fiyat farkı").color(tema::METIN_IKINCIL));
                egui::ComboBox::from_id_salt(format!("ff_yontem_{}", idx))
                    .selected_text(match h.fiyat_farki_ayari.yontem {
                        FiyatFarkiYontemi::Yok => "Yok",
                        FiyatFarkiYontemi::Manuel => "Manuel tutar",
                        FiyatFarkiYontemi::TekEndeks => "Tek endeks (sözleşmeye göre)",
                        FiyatFarkiYontemi::YapimAgirlikli => "Yapım işleri ağırlıklı formül",
                    })
                    .show_ui(ui, |ui| {
                        for (yontem, ad) in [
                            (FiyatFarkiYontemi::Yok, "Yok"),
                            (FiyatFarkiYontemi::Manuel, "Manuel tutar"),
                            (FiyatFarkiYontemi::TekEndeks, "Tek endeks (sözleşmeye göre)"),
                            (
                                FiyatFarkiYontemi::YapimAgirlikli,
                                "Yapım işleri ağırlıklı formül",
                            ),
                        ] {
                            if ui
                                .selectable_label(h.fiyat_farki_ayari.yontem == yontem, ad)
                                .clicked()
                            {
                                h.fiyat_farki_ayari.yontem = yontem;
                                h.ff_uygula = false;
                                ayar_degisti = true;
                            }
                        }
                    });
                if h.fiyat_farki_ayari.yontem == FiyatFarkiYontemi::Manuel {
                    ui.label("Tutar (TL)");
                    ayar_degisti |= ui
                        .add(
                            egui::DragValue::new(&mut h.fiyat_farki)
                                .speed(10.0)
                                .fixed_decimals(2),
                        )
                        .changed();
                }
                if matches!(
                    h.fiyat_farki_ayari.yontem,
                    FiyatFarkiYontemi::TekEndeks | FiyatFarkiYontemi::YapimAgirlikli
                ) {
                    ui.label("B");
                    ayar_degisti |= ui
                        .add(
                            egui::DragValue::new(&mut h.fiyat_farki_ayari.b)
                                .speed(0.01)
                                .range(0.0..=1.0)
                                .fixed_decimals(4),
                        )
                        .changed();
                    ui.label("Uygulama ayı");
                    ayar_degisti |= ui
                        .add(
                            TextEdit::singleline(&mut h.fiyat_farki_ayari.uygulama_ayi)
                                .hint_text("2026-07")
                                .desired_width(80.0),
                        )
                        .changed();
                }
                ui.hyperlink_to("TÜİK Veri Portalı ↗", "https://veriportali.tuik.gov.tr/tr/");
            });
            if h.fiyat_farki_ayari.yontem == FiyatFarkiYontemi::TekEndeks {
                ui.horizontal_wrapped(|ui| {
                    let b = &mut h.fiyat_farki_ayari.bilesenler[0];
                    ui.label("Yİ-ÜFE Genel · temel endeks");
                    ayar_degisti |= ui
                        .add(
                            egui::DragValue::new(&mut b.temel_endeks)
                                .speed(0.5)
                                .range(0.0..=f64::INFINITY)
                                .fixed_decimals(4),
                        )
                        .changed();
                    ui.label("güncel endeks");
                    ayar_degisti |= ui
                        .add(
                            egui::DragValue::new(&mut b.guncel_endeks)
                                .speed(0.5)
                                .range(0.0..=f64::INFINITY)
                                .fixed_decimals(4),
                        )
                        .changed();
                });
            }
            if h.fiyat_farki_ayari.yontem == FiyatFarkiYontemi::YapimAgirlikli {
                let katsayi_toplami: f64 = h
                    .fiyat_farki_ayari
                    .bilesenler
                    .iter()
                    .map(|b| b.katsayi)
                    .sum();
                ui.label(
                    RichText::new(format!(
                        "F = An × B × (Pn − 1) · katsayı toplamı: {:.6}{}",
                        katsayi_toplami,
                        if (katsayi_toplami - 1.0).abs() > 0.000001 {
                            "  (1,000000 olmalı)"
                        } else {
                            ""
                        }
                    ))
                    .color(if (katsayi_toplami - 1.0).abs() > 0.000001 {
                        tema::UYARI
                    } else {
                        tema::BASARI
                    })
                    .size(11.0),
                );
                egui::Grid::new(format!("ff_bilesenler_{}", idx))
                    .num_columns(5)
                    .spacing(egui::vec2(10.0, 4.0))
                    .striped(true)
                    .show(ui, |ui| {
                        for baslik in ["Kod", "Bileşen", "Katsayı", "Temel", "Güncel"] {
                            ui.label(RichText::new(baslik).strong().size(11.0));
                        }
                        ui.end_row();
                        for b in &mut h.fiyat_farki_ayari.bilesenler {
                            ui.label(RichText::new(&b.kod).monospace());
                            ui.label(&b.ad);
                            ayar_degisti |= ui
                                .add(
                                    egui::DragValue::new(&mut b.katsayi)
                                        .range(0.0..=1.0)
                                        .speed(0.001)
                                        .fixed_decimals(6),
                                )
                                .changed();
                            ayar_degisti |= ui
                                .add(
                                    egui::DragValue::new(&mut b.temel_endeks)
                                        .range(0.0..=f64::INFINITY)
                                        .speed(0.5)
                                        .fixed_decimals(4),
                                )
                                .changed();
                            ayar_degisti |= ui
                                .add(
                                    egui::DragValue::new(&mut b.guncel_endeks)
                                        .range(0.0..=f64::INFINITY)
                                        .speed(0.5)
                                        .fixed_decimals(4),
                                )
                                .changed();
                            ui.end_row();
                        }
                    });
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("KDV (%)")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                ayar_degisti |= ui
                    .add(
                        egui::DragValue::new(&mut h.kdv_orani)
                            .speed(0.5)
                            .range(0.0..=100.0),
                    )
                    .changed();
                ui.label(
                    RichText::new("Tevkifat (×)")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                ayar_degisti |= ui
                    .add(
                        egui::DragValue::new(&mut h.tevkifat_orani)
                            .speed(0.05)
                            .range(0.0..=1.0),
                    )
                    .on_hover_text("KDV tevkifatı oranı (ör. 4/10 = 0,40)")
                    .changed();
            });
        });
        if ayar_degisti {
            self.degisiklik_var = true;
        }
        ui.add_space(8.0);

        // Tablo: keşif kalemleri + editlenebilir kümülatif (yeşil defter)
        let mut degisti = false;
        let mut detay_ac: Option<usize> = None;
        let tenzilat_orani = self.sozlesme_ayarlari.tenzilat_orani();
        let sozlesme_fiyat_carpani = 1.0 - tenzilat_orani / 100.0;
        ScrollArea::both()
            .max_height((ui.available_height() - 230.0).max(140.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let bsl = |ui: &mut egui::Ui, t: &str| {
                    ui.label(
                        RichText::new(t)
                            .strong()
                            .size(11.0)
                            .color(tema::METIN_IKINCIL),
                    );
                };
                egui::Grid::new("hakedis_grid")
                    .num_columns(9)
                    .spacing(egui::vec2(9.0, 6.0))
                    .striped(true)
                    .show(ui, |ui| {
                        bsl(ui, "Poz No");
                        bsl(ui, "Açıklama");
                        bsl(ui, "Birim");
                        bsl(ui, "Sözleşme B.Fiyat");
                        bsl(ui, "Sözleşme");
                        bsl(ui, "Önceki Küm.");
                        bsl(ui, "Bu Küm. (yeşil defter)");
                        bsl(ui, "Bu Hakediş");
                        bsl(ui, "Tutar");
                        ui.end_row();
                        let h = &mut self.hakedisler[idx];
                        for (ri, (k, satir)) in kesif.iter().zip(h.satirlar.iter_mut()).enumerate()
                        {
                            let onceki = *onceki_kum.get(&k.id).unwrap_or(&0.0);
                            ui.label(
                                RichText::new(&k.poz_no)
                                    .monospace()
                                    .size(10.5)
                                    .color(tema::METIN),
                            );
                            ui.label(
                                RichText::new(metni_kisalt(&k.tanim, 34))
                                    .size(10.5)
                                    .color(tema::METIN_IKINCIL),
                            )
                            .on_hover_text(&k.tanim);
                            ui.label(RichText::new(&k.birim).size(10.5).color(tema::METIN_SOLUK));
                            ui.label(
                                RichText::new(para_formatla(
                                    k.birim_fiyat * sozlesme_fiyat_carpani,
                                ))
                                .size(10.5)
                                .color(tema::METIN_IKINCIL),
                            );
                            ui.label(
                                RichText::new(format!("{:.2}", k.miktar))
                                    .size(10.5)
                                    .color(tema::METIN_SOLUK),
                            );
                            ui.label(
                                RichText::new(format!("{:.2}", onceki))
                                    .size(10.5)
                                    .color(tema::METIN_SOLUK),
                            );
                            ui.horizontal(|ui| {
                                let kilitli = !satir.detaylar.is_empty();
                                if ui
                                    .add_enabled(
                                        !kilitli,
                                        egui::DragValue::new(&mut satir.kumulatif_miktar)
                                            .speed(0.1)
                                            .range(0.0..=f64::INFINITY),
                                    )
                                    .changed()
                                {
                                    degisti = true;
                                }
                                if ui
                                    .small_button("📐")
                                    .on_hover_text(if kilitli {
                                        "Ölçü kırılımı var — düzenle"
                                    } else {
                                        "Yeşil defter ölçü kırılımı"
                                    })
                                    .clicked()
                                {
                                    detay_ac = Some(ri);
                                }
                            });
                            let bu_miktar = satir.kumulatif_miktar - onceki;
                            let renk = if bu_miktar < 0.0 {
                                tema::TEHLIKE
                            } else {
                                tema::METIN
                            };
                            ui.label(
                                RichText::new(format!("{:.2}", bu_miktar))
                                    .size(10.5)
                                    .strong()
                                    .color(renk),
                            );
                            ui.label(
                                RichText::new(para_formatla(
                                    k.birim_fiyat * sozlesme_fiyat_carpani * bu_miktar,
                                ))
                                .size(10.5)
                                .strong()
                                .color(tema::BASARI),
                            );
                            ui.end_row();
                        }
                    });
            });
        if degisti {
            self.degisiklik_var = true;
        }
        if let Some(ri) = detay_ac {
            self.hakedis_detay_ac(idx, ri);
        }

        // İcmal (kesintiler + net ödeme)
        let guncel = self.hakedisler[idx].clone();
        let onceki_hakedis = if idx > 0 {
            Some(self.hakedisler[idx - 1].clone())
        } else {
            None
        };
        let hesaplar = poz_hesaplari(&kesif, &guncel, onceki_hakedis.as_ref(), tenzilat_orani);
        let ic = icmal(&hesaplar, &guncel);

        ui.add_space(6.0);
        egui::Frame::default()
            .fill(tema::YUZEY_2)
            .stroke(egui::Stroke::new(1.0, tema::KENAR))
            .corner_radius(egui::CornerRadius::same(tema::KOSE))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                let satir = |ui: &mut egui::Ui, etiket: &str, deger: f64, vurgu: bool| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(etiket)
                                .color(if vurgu {
                                    tema::METIN
                                } else {
                                    tema::METIN_IKINCIL
                                })
                                .size(if vurgu { 14.0 } else { 12.5 })
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{} TL", para_formatla(deger)))
                                    .color(if vurgu { tema::BASARI } else { tema::METIN })
                                    .strong()
                                    .size(if vurgu { 16.0 } else { 13.0 }),
                            );
                        });
                    });
                };
                satir(
                    ui,
                    "Kümülatif Brüt (toplam imalat)",
                    ic.kumulatif_brut,
                    false,
                );
                satir(ui, "Önceki Hakedişler Brütü", ic.onceki_brut, false);
                satir(ui, "Bu Hakediş Ham İmalat", ic.bu_hakedis_ham, false);
                if ic.tenzilat_tutari != 0.0 {
                    satir(
                        ui,
                        &format!("Tenzilat (% {:.6})", tenzilat_orani),
                        -ic.tenzilat_tutari,
                        false,
                    );
                }
                satir(
                    ui,
                    "Bu Hakediş (Tenzilat Sonrası)",
                    ic.bu_hakedis_brut,
                    false,
                );
                if ic.fiyat_farki != 0.0 {
                    satir(ui, "Fiyat Farkı (±)", ic.fiyat_farki, false);
                }
                satir(ui, "Tahakkuk (brüt + fiyat farkı)", ic.tahakkuk, false);
                ui.separator();
                satir(
                    ui,
                    &format!("Damga Vergisi (‰ {:.2})", guncel.damga_orani),
                    -ic.damga,
                    false,
                );
                if ic.teminat != 0.0 {
                    satir(
                        ui,
                        &format!("Teminat Kesintisi (% {:.1})", guncel.teminat_orani),
                        -ic.teminat,
                        false,
                    );
                }
                if ic.sgk != 0.0 {
                    satir(
                        ui,
                        &format!("SGK (% {:.1})", guncel.sgk_orani),
                        -ic.sgk,
                        false,
                    );
                }
                if ic.avans_mahsup != 0.0 {
                    satir(ui, "Avans Mahsubu", -ic.avans_mahsup, false);
                }
                satir(ui, "Kesinti Toplamı", -ic.kesinti_toplam, false);
                ui.separator();
                satir(ui, "KDV Hariç Net Tahakkuk", ic.net_odeme, false);
                if ic.kdv > 0.0 {
                    ui.add_space(2.0);
                    satir(
                        ui,
                        &format!("KDV (% {:.0})", guncel.kdv_orani),
                        ic.kdv,
                        false,
                    );
                    if ic.tevkifat != 0.0 {
                        satir(
                            ui,
                            &format!("KDV Tevkifatı (× {:.2})", guncel.tevkifat_orani),
                            -ic.tevkifat,
                            false,
                        );
                    }
                }
                ui.separator();
                satir(ui, "ÖDENECEK TUTAR", ic.odenecek_tutar, true);
                // Kesin hesap özeti
                let sozlesme_bedeli = crate::bicim::kurus_yuvarla(
                    self.sozlesme_ayarlari.hesaplanan_sozlesme_bedeli(),
                );
                ui.add_space(4.0);
                ui.separator();
                satir(ui, "Sözleşme Bedeli", sozlesme_bedeli, false);
                satir(ui, "Gerçekleşen (Kümülatif)", ic.kumulatif_brut, false);
                satir(
                    ui,
                    "Sözleşme Farkı (+ fazla / − eksik)",
                    ic.kumulatif_brut - sozlesme_bedeli,
                    false,
                );
            });
    }

    /// Seçili hakedişi Excel'e aktarır.
    pub(crate) fn hakedis_excel_diyalog(&mut self) {
        let idx = match self.secili_hakedis {
            Some(i) if i < self.hakedisler.len() => i,
            _ => return,
        };
        let kesif = self.kesif_kalemleri();
        let guncel = self.hakedisler[idx].clone();
        let onceki = if idx > 0 {
            Some(self.hakedisler[idx - 1].clone())
        } else {
            None
        };
        let proje_adi = self.metraj_adi.clone();
        let pb = self.proje_bilgi.clone();
        if let Some(d) = rfd::FileDialog::new()
            .add_filter("Excel", &["xlsx"])
            .set_file_name(format!("{} - Hakedis {}.xlsx", self.metraj_adi, guncel.no))
            .save_file()
        {
            match crate::export::hakedis_excel_aktar(
                &proje_adi,
                &pb,
                &kesif,
                &guncel,
                onceki.as_ref(),
                &self.sozlesme_ayarlari,
                &d,
            ) {
                Ok(()) => self.basarili_mesaj = format!("Hakediş Excel: {}", d.display()),
                Err(e) => self.hata_mesaji = e,
            }
        }
    }

    fn hakedis_detay_ac(&mut self, hidx: usize, ridx: usize) {
        if let Some(s) = self.hakedisler.get(hidx).and_then(|h| h.satirlar.get(ridx)) {
            self.popup_detaylar = s.detaylar.iter().map(detay_to_satir).collect();
        }
        self.popup_yeni = PopupDetaySatiri::default();
        self.hakedis_detay_satir = Some(ridx);
        self.hakedis_detay_acik = true;
    }

    pub(crate) fn render_hakedis_detay_popup(&mut self, ctx: &egui::Context) {
        if !self.hakedis_detay_acik {
            return;
        }
        let (hidx, ridx) = match (self.secili_hakedis, self.hakedis_detay_satir) {
            (Some(h), Some(r)) => (h, r),
            _ => {
                self.hakedis_detay_acik = false;
                return;
            }
        };
        let poz_no = self
            .hakedisler
            .get(hidx)
            .and_then(|h| h.satirlar.get(ridx))
            .map(|s| s.poz_no.clone())
            .unwrap_or_default();
        let mut tamam = false;
        let mut iptal = false;
        egui::Window::new("📐 Yeşil Defter — Ölçü Kırılımı")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("Poz: {}", poz_no))
                        .monospace()
                        .strong()
                        .size(14.0),
                );
                ui.label(
                    RichText::new("Boş boyut 1 sayılır · “çıkan” işaretli satır düşülür")
                        .size(11.0)
                        .color(tema::METIN_SOLUK),
                );
                ui.add_space(3.0);
                let bsl = |ui: &mut egui::Ui, t: &str| {
                    ui.label(
                        RichText::new(t)
                            .strong()
                            .size(11.0)
                            .color(tema::METIN_IKINCIL),
                    );
                };
                egui::Grid::new("hakedis_detay_grid")
                    .num_columns(8)
                    .spacing(egui::vec2(7.0, 6.0))
                    .striped(true)
                    .show(ui, |ui| {
                        bsl(ui, "Açıklama");
                        bsl(ui, "Adet");
                        bsl(ui, "En");
                        bsl(ui, "Boy");
                        bsl(ui, "Yük.");
                        bsl(ui, "Çıkan");
                        bsl(ui, "= Miktar");
                        bsl(ui, "");
                        ui.end_row();
                        let mut sil: Option<usize> = None;
                        for (i, satir) in self.popup_detaylar.iter_mut().enumerate() {
                            ui.add(
                                TextEdit::singleline(&mut satir.aciklama)
                                    .desired_width(150.0)
                                    .hint_text("açıklama"),
                            );
                            ui.add(TextEdit::singleline(&mut satir.adet).desired_width(46.0));
                            ui.add(TextEdit::singleline(&mut satir.en).desired_width(46.0));
                            ui.add(TextEdit::singleline(&mut satir.boy).desired_width(46.0));
                            ui.add(TextEdit::singleline(&mut satir.yukseklik).desired_width(46.0));
                            ui.checkbox(&mut satir.cikan, "");
                            let m = satir_miktar(satir).unwrap_or(0.0);
                            ui.label(
                                RichText::new(format!("{:.3}", m))
                                    .size(11.0)
                                    .strong()
                                    .color(if m < 0.0 { tema::UYARI } else { tema::BASARI }),
                            );
                            if ui.small_button("🗑").clicked() {
                                sil = Some(i);
                            }
                            ui.end_row();
                        }
                        if let Some(s) = sil {
                            self.popup_detaylar.remove(s);
                        }
                    });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Yeni").size(11.0).color(tema::METIN_IKINCIL));
                    ui.add(
                        TextEdit::singleline(&mut self.popup_yeni.aciklama)
                            .hint_text("açıklama")
                            .desired_width(140.0),
                    );
                    ui.add(
                        TextEdit::singleline(&mut self.popup_yeni.adet)
                            .hint_text("adet")
                            .desired_width(46.0),
                    );
                    ui.add(
                        TextEdit::singleline(&mut self.popup_yeni.en)
                            .hint_text("en")
                            .desired_width(46.0),
                    );
                    ui.add(
                        TextEdit::singleline(&mut self.popup_yeni.boy)
                            .hint_text("boy")
                            .desired_width(46.0),
                    );
                    ui.add(
                        TextEdit::singleline(&mut self.popup_yeni.yukseklik)
                            .hint_text("yük.")
                            .desired_width(46.0),
                    );
                    ui.checkbox(&mut self.popup_yeni.cikan, "çıkan");
                    let ekle = tema::birincil_buton(ui, "＋ Ekle").clicked();
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (ekle || enter) && satir_miktar(&self.popup_yeni).is_some() {
                        self.popup_detaylar
                            .push(std::mem::take(&mut self.popup_yeni));
                    }
                });
                ui.separator();
                let toplam: f64 = self.popup_detaylar.iter().filter_map(satir_miktar).sum();
                ui.label(
                    RichText::new(format!("Kümülatif miktar = {:.3}", toplam))
                        .size(14.0)
                        .strong()
                        .color(tema::BASARI),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if tema::basari_buton(ui, "✓ Tamam").clicked() {
                        tamam = true;
                    }
                    if ui.button("❌ İptal").clicked() {
                        iptal = true;
                    }
                });
            });
        if tamam {
            let detaylar: Vec<MiktarDetay> = self
                .popup_detaylar
                .iter()
                .filter_map(satir_to_detay)
                .collect();
            if detaylar.len() != self.popup_detaylar.len() {
                self.hata_mesaji =
                    "Yeşil defter satırlarında geçersiz sayı var; kayıt yapılmadı.".into();
                return;
            }
            if let Some(s) = self
                .hakedisler
                .get_mut(hidx)
                .and_then(|h| h.satirlar.get_mut(ridx))
            {
                s.detaylar = detaylar;
                s.detaylardan_tazele();
            }
            self.degisiklik_var = true;
            self.hakedis_detay_acik = false;
        }
        if iptal {
            self.hakedis_detay_acik = false;
        }
    }
}
