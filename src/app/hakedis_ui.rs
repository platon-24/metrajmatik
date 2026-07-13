//! Hakediş sekmesi: sözleşme (keşif) kalemleri üzerinden kümülatif imalat miktarları
//! (yeşil defter), bu hakediş tutarları, kesintiler ve net ödeme. Excel raporu.

use eframe::egui;
use egui::{RichText, ScrollArea, TextEdit, Ui};
use std::collections::HashMap;

use crate::bicim::{krono_tarih, metni_kisalt, para_formatla};
use crate::hakedis::{icmal, poz_hesaplari};
use crate::models::{Hakedis, HakedisSatiri, MetrajKalemi};
use crate::tema;

use super::MetrajApp;

impl MetrajApp {
    /// Sözleşme/keşif kalemleri: iş gruplarının düzleştirilmiş tüm kalemleri.
    fn kesif_kalemleri(&mut self) -> Vec<MetrajKalemi> {
        self.aktif_grubu_senkronize();
        if self.is_gruplari.is_empty() {
            self.metraj_kalemleri.clone()
        } else {
            let mut v = Vec::new();
            for g in &self.is_gruplari { v.extend(g.tum_kalemler_duz()); }
            v
        }
    }

    /// Yeni hakediş: önceki hakedişin kümülatiflerini devralır (kümülatif hep artar).
    fn hakedis_yeni(&mut self) {
        let kesif = self.kesif_kalemleri();
        let onceki = self.hakedisler.last();
        let satirlar: Vec<HakedisSatiri> = kesif.iter().map(|k| HakedisSatiri {
            poz_no: k.poz_no.clone(),
            kumulatif_miktar: onceki.map(|h| h.kumulatif(&k.poz_no)).unwrap_or(0.0),
        }).collect();
        let no = self.hakedisler.len() as u32 + 1;
        let tur = if self.hakedisler.is_empty() { "İlk" } else { "Ara" };
        self.hakedisler.push(Hakedis {
            no, tarih: krono_tarih(), tur: tur.to_string(), satirlar,
            damga_orani: 9.48, teminat_orani: 0.0, sgk_orani: 0.0, avans_mahsup: 0.0, fiyat_farki: 0.0,
        });
        self.secili_hakedis = Some(self.hakedisler.len() - 1);
        self.degisiklik_var = true;
    }

    /// Seçili hakedişin satırlarını güncel keşifle hizalar (poz eklenmiş/silinmişse).
    fn hakedis_hizala(&mut self, idx: usize, kesif: &[MetrajKalemi]) {
        if let Some(h) = self.hakedisler.get_mut(idx) {
            let eski: HashMap<String, f64> = h.satirlar.iter().map(|s| (s.poz_no.clone(), s.kumulatif_miktar)).collect();
            h.satirlar = kesif.iter().map(|k| HakedisSatiri {
                poz_no: k.poz_no.clone(),
                kumulatif_miktar: *eski.get(&k.poz_no).unwrap_or(&0.0),
            }).collect();
        }
    }

    pub(crate) fn render_hakedis(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "🧾", "Hakediş");
        ui.add_space(6.0);

        let kesif = self.kesif_kalemleri();
        if kesif.is_empty() {
            tema::bildirim_seridi(ui, "Önce Metraj sekmesinden sözleşme (keşif) kalemlerini girin. Hakediş bunların üzerine kurulur.", tema::UYARI_KOYU, tema::UYARI, tema::UYARI);
            return;
        }

        // Hakediş listesi + yeni + excel + sil
        let mut secilecek: Option<usize> = None;
        let mut yeni = false;
        let mut excel = false;
        let mut sil: Option<usize> = None;
        tema::kart(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Hakedişler:").color(tema::METIN_IKINCIL).size(12.0));
                for (i, h) in self.hakedisler.iter().enumerate() {
                    let secili = self.secili_hakedis == Some(i);
                    if ui.selectable_label(secili, format!("{}. {} ({})", h.no, h.tur, h.tarih)).clicked() {
                        secilecek = Some(i);
                    }
                }
                if tema::birincil_buton(ui, "＋ Yeni Hakediş").clicked() { yeni = true; }
                if self.secili_hakedis.is_some() {
                    if ui.button("📊 Excel").clicked() { excel = true; }
                    if tema::tehlike_buton(ui, "🗑 Sil").clicked() { sil = self.secili_hakedis; }
                }
            });
        });
        if yeni { self.hakedis_yeni(); }
        if let Some(i) = secilecek { self.secili_hakedis = Some(i); }
        if let Some(i) = sil {
            self.hakedisler.remove(i);
            for (j, h) in self.hakedisler.iter_mut().enumerate() { h.no = j as u32 + 1; }
            self.secili_hakedis = if self.hakedisler.is_empty() { None } else { Some(self.hakedisler.len() - 1) };
            self.degisiklik_var = true;
        }
        if excel { self.hakedis_excel_diyalog(); }

        let idx = match self.secili_hakedis {
            Some(i) if i < self.hakedisler.len() => i,
            _ => {
                ui.add_space(8.0);
                ui.label(RichText::new("Bir hakediş seçin veya '＋ Yeni Hakediş' ile oluşturun.").color(tema::METIN_SOLUK));
                return;
            }
        };

        self.hakedis_hizala(idx, &kesif);

        let onceki_kum: HashMap<String, f64> = if idx > 0 {
            self.hakedisler[idx - 1].satirlar.iter().map(|s| (s.poz_no.clone(), s.kumulatif_miktar)).collect()
        } else { HashMap::new() };

        ui.add_space(8.0);
        // Tür / tarih / kesinti oranları
        tema::kart(ui, |ui| {
            let h = &mut self.hakedisler[idx];
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Hakediş No: {}", h.no)).strong().color(tema::METIN));
                ui.add_space(10.0);
                ui.label(RichText::new("Tür").color(tema::METIN_IKINCIL).size(12.0));
                for t in ["İlk", "Ara", "Kesin"] {
                    if ui.selectable_label(h.tur == t, t).clicked() { h.tur = t.to_string(); }
                }
                ui.add_space(10.0);
                ui.label(RichText::new("Tarih").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(TextEdit::singleline(&mut h.tarih).desired_width(100.0));
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Damga (‰)").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(egui::DragValue::new(&mut h.damga_orani).speed(0.1).range(0.0..=100.0));
                ui.add_space(12.0);
                ui.label(RichText::new("Teminat (%)").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(egui::DragValue::new(&mut h.teminat_orani).speed(0.1).range(0.0..=100.0));
                ui.add_space(12.0);
                ui.label(RichText::new("SGK (%)").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(egui::DragValue::new(&mut h.sgk_orani).speed(0.1).range(0.0..=100.0));
                ui.add_space(12.0);
                ui.label(RichText::new("Avans mahsubu (TL)").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(egui::DragValue::new(&mut h.avans_mahsup).speed(10.0).range(0.0..=f64::INFINITY));
                ui.add_space(12.0);
                ui.label(RichText::new("Fiyat farkı (TL)").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(egui::DragValue::new(&mut h.fiyat_farki).speed(10.0));
            });
        });
        ui.add_space(8.0);

        // Tablo: keşif kalemleri + editlenebilir kümülatif (yeşil defter)
        let mut degisti = false;
        ScrollArea::vertical().max_height((ui.available_height() - 230.0).max(140.0)).auto_shrink([false, false]).show(ui, |ui| {
            let bsl = |ui: &mut egui::Ui, t: &str| { ui.label(RichText::new(t).strong().size(11.0).color(tema::METIN_IKINCIL)); };
            egui::Grid::new("hakedis_grid").num_columns(9).spacing(egui::vec2(9.0, 6.0)).striped(true).show(ui, |ui| {
                bsl(ui, "Poz No"); bsl(ui, "Açıklama"); bsl(ui, "Birim"); bsl(ui, "B.Fiyat");
                bsl(ui, "Sözleşme"); bsl(ui, "Önceki Küm."); bsl(ui, "Bu Küm. (yeşil defter)"); bsl(ui, "Bu Hakediş"); bsl(ui, "Tutar");
                ui.end_row();
                let h = &mut self.hakedisler[idx];
                for (k, satir) in kesif.iter().zip(h.satirlar.iter_mut()) {
                    let onceki = *onceki_kum.get(&k.poz_no).unwrap_or(&0.0);
                    ui.label(RichText::new(&k.poz_no).monospace().size(10.5).color(tema::METIN));
                    ui.label(RichText::new(metni_kisalt(&k.tanim, 34)).size(10.5).color(tema::METIN_IKINCIL)).on_hover_text(&k.tanim);
                    ui.label(RichText::new(&k.birim).size(10.5).color(tema::METIN_SOLUK));
                    ui.label(RichText::new(para_formatla(k.birim_fiyat)).size(10.5).color(tema::METIN_IKINCIL));
                    ui.label(RichText::new(format!("{:.2}", k.miktar)).size(10.5).color(tema::METIN_SOLUK));
                    ui.label(RichText::new(format!("{:.2}", onceki)).size(10.5).color(tema::METIN_SOLUK));
                    if ui.add(egui::DragValue::new(&mut satir.kumulatif_miktar).speed(0.1).range(0.0..=f64::INFINITY)).changed() { degisti = true; }
                    let bu_miktar = satir.kumulatif_miktar - onceki;
                    let renk = if bu_miktar < 0.0 { tema::TEHLIKE } else { tema::METIN };
                    ui.label(RichText::new(format!("{:.2}", bu_miktar)).size(10.5).strong().color(renk));
                    ui.label(RichText::new(para_formatla(k.birim_fiyat * bu_miktar)).size(10.5).strong().color(tema::BASARI));
                    ui.end_row();
                }
            });
        });
        if degisti { self.degisiklik_var = true; }

        // İcmal (kesintiler + net ödeme)
        let guncel = self.hakedisler[idx].clone();
        let onceki_hakedis = if idx > 0 { Some(self.hakedisler[idx - 1].clone()) } else { None };
        let hesaplar = poz_hesaplari(&kesif, &guncel, onceki_hakedis.as_ref());
        let ic = icmal(&hesaplar, &guncel);

        ui.add_space(6.0);
        egui::Frame::default().fill(tema::YUZEY_2).stroke(egui::Stroke::new(1.0, tema::KENAR)).corner_radius(egui::CornerRadius::same(tema::KOSE)).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            let satir = |ui: &mut egui::Ui, etiket: &str, deger: f64, vurgu: bool| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(etiket).color(if vurgu { tema::METIN } else { tema::METIN_IKINCIL }).size(if vurgu { 14.0 } else { 12.5 }).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} TL", para_formatla(deger))).color(if vurgu { tema::BASARI } else { tema::METIN }).strong().size(if vurgu { 16.0 } else { 13.0 }));
                    });
                });
            };
            satir(ui, "Kümülatif Brüt (toplam imalat)", ic.kumulatif_brut, false);
            satir(ui, "Önceki Hakedişler Brütü", ic.onceki_brut, false);
            satir(ui, "Bu Hakediş Brütü", ic.bu_hakedis_brut, false);
            if ic.fiyat_farki != 0.0 { satir(ui, "Fiyat Farkı (±)", ic.fiyat_farki, false); }
            satir(ui, "Tahakkuk (brüt + fiyat farkı)", ic.tahakkuk, false);
            ui.separator();
            satir(ui, &format!("Damga Vergisi (‰ {:.2})", guncel.damga_orani), -ic.damga, false);
            if ic.teminat != 0.0 { satir(ui, &format!("Teminat Kesintisi (% {:.1})", guncel.teminat_orani), -ic.teminat, false); }
            if ic.sgk != 0.0 { satir(ui, &format!("SGK (% {:.1})", guncel.sgk_orani), -ic.sgk, false); }
            if ic.avans_mahsup != 0.0 { satir(ui, "Avans Mahsubu", -ic.avans_mahsup, false); }
            satir(ui, "Kesinti Toplamı", -ic.kesinti_toplam, false);
            ui.separator();
            satir(ui, "NET ÖDEME", ic.net_odeme, true);
        });
    }

    /// Seçili hakedişi Excel'e aktarır.
    pub(crate) fn hakedis_excel_diyalog(&mut self) {
        let idx = match self.secili_hakedis { Some(i) if i < self.hakedisler.len() => i, _ => return };
        let kesif = self.kesif_kalemleri();
        let guncel = self.hakedisler[idx].clone();
        let onceki = if idx > 0 { Some(self.hakedisler[idx - 1].clone()) } else { None };
        let proje_adi = self.metraj_adi.clone();
        if let Some(d) = rfd::FileDialog::new().add_filter("Excel", &["xlsx"]).set_file_name(&format!("{} - Hakedis {}.xlsx", self.metraj_adi, guncel.no)).save_file() {
            match crate::export::hakedis_excel_aktar(&proje_adi, &kesif, &guncel, onceki.as_ref(), &d) {
                Ok(()) => self.basarili_mesaj = format!("Hakediş Excel: {}", d.display()),
                Err(e) => self.hata_mesaji = e,
            }
        }
    }
}
