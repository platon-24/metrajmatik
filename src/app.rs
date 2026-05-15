use eframe::egui;
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui, Vec2};
use std::path::PathBuf;

use crate::database::Veritabani;
use crate::export::{metraj_excel_aktar, metraj_json_kaydet, metraj_json_yukle};
use crate::models::{KayitliMetraj, Kitap, MetrajKalemi, Poz};
use crate::pdf_parser::{pdf_metin_cikar, pozlari_ayristir};

#[derive(Debug, Clone, PartialEq)]
enum Sekme { MetrajTablosu, Pozlar, KitapYoneticisi, PdfYukle }

pub struct MetrajApp {
    db: Option<Veritabani>,
    poz_sayisi: u32,
    kitaplar: Vec<Kitap>,
    secili_kitap: Option<Kitap>,
    metraj_kalemleri: Vec<MetrajKalemi>,
    metraj_adi: String,
    mevcut_dosya_yolu: Option<PathBuf>,
    degisiklik_var: bool,
    poz_arama_metni: String,
    arama_sonuclari: Vec<Poz>,
    secili_poz: Option<Poz>,
    aciklama_arama_metni: String,
    yeni_poz_no: String,
    yeni_miktar: String,
    yeni_kitap_adi: String,
    yeni_kitap_yil: u32,
    yeni_kitap_ay: u32,
    // Kitap düzenleme
    duzenlenen_kitap: Option<Kitap>,
    duzenleme_adi: String,
    duzenleme_yil: u32,
    duzenleme_ay: u32,
    fiyat_guncelle_hedef: Option<Kitap>,
    cift_tiklama_ekle: bool,
    pdf_durumu: String,
    pdf_yukleniyor: bool,
    aktif_sekme: Sekme,
    hata_mesaji: String,
    basarili_mesaj: String,
    kategoriler: Vec<String>,
    secili_kategori: String,
    kategori_pozlar: Vec<Poz>,
    pozlar_arama_metni: String,
    pozlar_tablosu: Vec<Poz>,
    pozlar_yuklu_kitap_id: Option<i64>,
}

impl Default for MetrajApp {
    fn default() -> Self {
        let db_yolu = PathBuf::from("metrajmatik_veriler.db");
        let (db, poz_sayisi, kitaplar) = match Veritabani::ac(&db_yolu) {
            Ok(vt) => {
                let s = vt.poz_sayisi().unwrap_or(0);
                let k = vt.kitaplari_listele().unwrap_or_default();
                (Some(vt), s, k)
            }
            Err(e) => { log::error!("{}", e); (None, 0, vec![]) }
        };
        Self {
            db, poz_sayisi, kitaplar, secili_kitap: None,
            metraj_kalemleri: vec![], metraj_adi: "Isimsiz Metraj".into(),
            mevcut_dosya_yolu: None, degisiklik_var: false,
            poz_arama_metni: String::new(), arama_sonuclari: vec![], secili_poz: None,
            aciklama_arama_metni: String::new(), yeni_poz_no: String::new(), yeni_miktar: String::new(),
            yeni_kitap_adi: String::new(), yeni_kitap_yil: 2026, yeni_kitap_ay: 5,
            duzenlenen_kitap: None, duzenleme_adi: String::new(), duzenleme_yil: 2026, duzenleme_ay: 1,
            fiyat_guncelle_hedef: None,
            cift_tiklama_ekle: false,
            pdf_durumu: String::new(), pdf_yukleniyor: false,
            aktif_sekme: Sekme::MetrajTablosu,
            hata_mesaji: String::new(), basarili_mesaj: String::new(),
            kategoriler: vec![], secili_kategori: "TÜMÜ".into(), kategori_pozlar: vec![],
            pozlar_arama_metni: String::new(), pozlar_tablosu: vec![], pozlar_yuklu_kitap_id: None,
        }
    }
}

impl eframe::App for MetrajApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) { self.metraj_kaydet(); }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) { self.metraj_yukle_diyalog(); }

        // Kitap düzenleme modal'ı
        if self.duzenlenen_kitap.is_some() {
            egui::Window::new("✏️ Kitap Düzenle")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Kitap Adı:");
                    ui.add(TextEdit::singleline(&mut self.duzenleme_adi).desired_width(300.0));
                    ui.horizontal(|ui| {
                        ui.label("Yıl:");
                        egui::ComboBox::from_id_salt("duz_yil").selected_text(format!("{}", self.duzenleme_yil)).width(70.0).show_ui(ui, |ui| {
                            for y in [2024u32, 2025, 2026, 2027, 2028] {
                                if ui.selectable_label(self.duzenleme_yil == y, format!("{}", y)).clicked() { self.duzenleme_yil = y; }
                            }
                        });
                        ui.label("Ay:");
                        egui::ComboBox::from_id_salt("duz_ay").selected_text(format!("{}", self.duzenleme_ay)).width(50.0).show_ui(ui, |ui| {
                            for a in 1u32..=12 {
                                if ui.selectable_label(self.duzenleme_ay == a, format!("{}", a)).clicked() { self.duzenleme_ay = a; }
                            }
                        });
                    });
                    ui.add_space(5.0);
                    ui.label("⚠ Yıl/Ay değişirse tüm pozlardaki yıl/ay da güncellenir.");
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        if ui.button("✅ Kaydet").clicked() {
                            if let Some(ref db) = self.db {
                                let kitap_id = self.duzenlenen_kitap.as_ref().unwrap().id;
                                let _ = db.kitap_guncelle(kitap_id, &self.duzenleme_adi, self.duzenleme_yil, self.duzenleme_ay);
                                self.basarili_mesaj = format!("'{}' güncellendi.", self.duzenleme_adi);
                                self.duzenlenen_kitap = None;
                                self.kitaplari_yenile();
                                // Seçili kitabı da güncelle
                                if let Some(ref mut sk) = self.secili_kitap {
                                    if sk.id == kitap_id { sk.ad = self.duzenleme_adi.clone(); sk.yil = self.duzenleme_yil; sk.ay = self.duzenleme_ay; }
                                }
                            }
                        }
                        if ui.button("❌ İptal").clicked() {
                            self.duzenlenen_kitap = None;
                        }
                    });
                });
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(44, 62, 80);
                ui.style_mut().visuals.widgets.active.weak_bg_fill = Color32::from_rgb(52, 73, 94);
                ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(52, 73, 94);

                let bl = if let Some(ref yol) = self.mevcut_dosya_yolu {
                    format!("🏗 METRAJMATIK - {}", yol.file_name().unwrap().to_string_lossy())
                } else { "🏗 METRAJMATIK".to_string() };
                ui.label(RichText::new(bl).color(Color32::WHITE).size(18.0).strong());
                ui.separator();

                let sekmeler = [Sekme::MetrajTablosu, Sekme::Pozlar, Sekme::KitapYoneticisi, Sekme::PdfYukle];
                let isimler = ["📋 Metraj", "🔎 Pozlar", "📚 Kitaplar", "📄 PDF Yükle"];
                for i in 0..4 {
                    let s = &sekmeler[i];
                    if ui.selectable_label(self.aktif_sekme == *s, RichText::new(isimler[i]).color(Color32::WHITE)).clicked() {
                        self.aktif_sekme = s.clone();
                        if *s == Sekme::MetrajTablosu || *s == Sekme::Pozlar || *s == Sekme::KitapYoneticisi { self.kitaplari_yenile(); }
                        if *s == Sekme::MetrajTablosu { self.kategorileri_yukle(); }
                        if *s == Sekme::Pozlar { self.pozlar_tablosu_yenile(); }
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.hata_mesaji.is_empty() { ui.colored_label(Color32::RED, &self.hata_mesaji); if ui.button("✕").clicked() { self.hata_mesaji.clear(); } }
            if !self.basarili_mesaj.is_empty() { ui.colored_label(Color32::GREEN, &self.basarili_mesaj); }
            match self.aktif_sekme {
                Sekme::MetrajTablosu => self.render_metraj_tablosu(ui),
                Sekme::Pozlar => self.render_pozlar_tablosu(ui),
                Sekme::KitapYoneticisi => self.render_kitap_yoneticisi(ui),
                Sekme::PdfYukle => self.render_pdf_yukle(ui),
            }
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let ki = self.secili_kitap.as_ref().map(|k| format!("📚 {} | ", k.ad)).unwrap_or_default();
            let da = self.mevcut_dosya_yolu.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "Kaydedilmemis".into());
            ui.label(format!("📁 {} {}| {}{} poz | 📋 {} kalem | 💰 {:.2} TL",
                da, if self.degisiklik_var { "● " } else { "" }, ki, self.poz_sayisi, self.metraj_kalemleri.len(), self.toplam_tutar()));
        });
    }
}

impl MetrajApp {
    // ==================== KITAP YONETICISI ====================
    fn render_kitap_yoneticisi(&mut self, ui: &mut Ui) {
        ui.heading("📚 Kitap Yöneticisi");
        ui.separator();

        // Yeni kitap ekleme
        ui.horizontal(|ui| {
            ui.label("Kitap Adı:");
            ui.add(TextEdit::singleline(&mut self.yeni_kitap_adi).hint_text("örn: Cevre ve Sehircilik").desired_width(220.0));
            ui.label("Yıl:");
            egui::ComboBox::from_id_salt("yil_combo").selected_text(format!("{}", self.yeni_kitap_yil)).width(70.0).show_ui(ui, |ui| {
                for y in [2024u32, 2025, 2026, 2027, 2028] { if ui.selectable_label(self.yeni_kitap_yil == y, format!("{}", y)).clicked() { self.yeni_kitap_yil = y; } }
            });
            ui.label("Ay:");
            egui::ComboBox::from_id_salt("ay_combo").selected_text(format!("{}", self.yeni_kitap_ay)).width(50.0).show_ui(ui, |ui| {
                for a in 1u32..=12 { if ui.selectable_label(self.yeni_kitap_ay == a, format!("{}", a)).clicked() { self.yeni_kitap_ay = a; } }
            });
            if ui.button("➕ Ekle").clicked() {
                let ad = self.yeni_kitap_adi.trim().to_string();
                if ad.is_empty() { self.hata_mesaji = "Kitap adi girin.".into(); }
                else if let Some(ref db) = self.db {
                    match db.kitap_ekle(&ad, self.yeni_kitap_yil, self.yeni_kitap_ay) {
                        Ok(_) => { self.basarili_mesaj = format!("'{}' ({}/{}) eklendi.", ad, self.yeni_kitap_ay, self.yeni_kitap_yil); self.yeni_kitap_adi.clear(); self.kitaplari_yenile(); }
                        Err(e) => self.hata_mesaji = format!("{}", e),
                    }
                } else { self.hata_mesaji = "Veritabani acik degil!".into(); }
            }
        });

        ui.add_space(10.0); ui.separator();
        ui.label(RichText::new("Yüklü Kitaplar:").strong().size(14.0)); ui.add_space(5.0);
        if self.kitaplar.is_empty() { ui.label("Henuz kitap yok."); return; }

        let kitaplar_snapshot = self.kitaplar.clone();
        egui::Grid::new("kitap_grid").num_columns(8).min_col_width(50.0).striped(true).show(ui, |ui: &mut egui::Ui| {
            ui.label(RichText::new("ID").strong()); ui.label(RichText::new("Kitap Adı").strong());
            ui.label(RichText::new("Yıl").strong()); ui.label(RichText::new("Ay").strong());
            ui.label(RichText::new("Poz").strong()); ui.label(RichText::new("Tarih").strong());
            ui.label(RichText::new("Düzenle").strong()); ui.label(RichText::new("Sil").strong());
            ui.end_row();

            for kitap in &kitaplar_snapshot {
                let secili = self.secili_kitap.as_ref().map(|k| k.id == kitap.id).unwrap_or(false);
                ui.label(if secili { RichText::new(format!("{}", kitap.id)).color(Color32::GREEN) } else { RichText::new(format!("{}", kitap.id)) });
                if ui.selectable_label(secili, &kitap.ad).clicked() {
                    self.secili_kitap = Some(kitap.clone());
                    self.kategorileri_yukle();
                    self.basarili_mesaj = format!("{} secildi.", kitap.ad);
                }
                ui.label(format!("{}", kitap.yil));
                ui.label(format!("{}", kitap.ay));
                ui.label(format!("{}", kitap.poz_sayisi));
                ui.label(&kitap.tarih);
                // Düzenle butonu
                if ui.button("✏️").clicked() {
                    self.duzenlenen_kitap = Some(kitap.clone());
                    self.duzenleme_adi = kitap.ad.clone();
                    self.duzenleme_yil = kitap.yil;
                    self.duzenleme_ay = kitap.ay;
                }
                if ui.button(RichText::new("🗑").color(Color32::RED)).clicked() {
                    if let Some(ref db) = self.db {
                        if db.kitap_sil(kitap.id).is_ok() {
                            if self.secili_kitap.as_ref().map(|k| k.id == kitap.id).unwrap_or(false) { self.secili_kitap = None; }
                            self.basarili_mesaj = format!("{} silindi.", kitap.ad);
                            self.kitaplari_yenile();
                        }
                    }
                }
                ui.end_row();
            }
        });

        ui.separator();
        if let Some(ref k) = self.secili_kitap {
            ui.colored_label(Color32::GREEN, format!("✅ Aktif: {} ({}/{}, {} poz)", k.ad, k.ay, k.yil, k.poz_sayisi));
        }
    }

    // ==================== METRAJ TABLOSU ====================
    fn render_metraj_tablosu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Kitap:").strong());
            let km = self.secili_kitap.as_ref().map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil)).unwrap_or_else(|| "TÜM KITAPLAR".into());
            egui::ComboBox::from_id_salt("kitap_secici").selected_text(&km).width(350.0).show_ui(ui, |ui| {
                if ui.selectable_label(self.secili_kitap.is_none(), "TÜM KİTAPLAR").clicked() { self.secili_kitap = None; self.kategorileri_yukle(); }
                for k in self.kitaplar.clone() {
                    if ui.selectable_label(false, format!("{} ({}/{})", k.ad, k.ay, k.yil)).clicked() { self.secili_kitap = Some(k); self.kategorileri_yukle(); }
                }
            });
        });
        ui.separator();

        egui::SidePanel::left("sol_panel").resizable(true).default_width(400.0).min_width(300.0).show_inside(ui, |ui| { self.render_arama_paneli(ui); });
        egui::CentralPanel::default().show_inside(ui, |ui| { self.render_metraj_listesi(ui); });
    }

    fn render_arama_paneli(&mut self, ui: &mut Ui) {
        ui.heading("🔍 Poz Arama");
        ui.horizontal(|ui| {
            ui.label("Poz No:");
            if ui.add_sized(Vec2::new(200.0, 24.0), TextEdit::singleline(&mut self.poz_arama_metni).hint_text("örn: 15.100")).changed() { self.poz_no_ara(); }
        });
        ui.horizontal(|ui| {
            ui.label("Açıklama:");
            if ui.add_sized(Vec2::new(200.0, 24.0), TextEdit::singleline(&mut self.aciklama_arama_metni).hint_text("örn: beton")).changed() && !self.aciklama_arama_metni.is_empty() { self.aciklama_ara(); }
        });
        if !self.kategoriler.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Kategori:");
                egui::ComboBox::from_id_salt("kategori_combo").selected_text(&self.secili_kategori).width(200.0).show_ui(ui, |ui| {
                    if ui.selectable_label(false, "TÜMÜ").clicked() { self.secili_kategori = "TÜMÜ".into(); self.kategori_pozlar.clear(); }
                    for kat in &self.kategoriler.clone() { if ui.selectable_label(false, kat).clicked() { self.secili_kategori = kat.clone(); self.kategori_filtrele(); } }
                });
            });
        }
        ui.separator();

        let pl = if !self.kategori_pozlar.is_empty() { &self.kategori_pozlar } else { &self.arama_sonuclari };
        if !pl.is_empty() { ui.label(format!("{} sonuc", pl.len())); }
        else if !self.poz_arama_metni.is_empty() || !self.aciklama_arama_metni.is_empty() { ui.label("Sonuc yok."); }
        else { ui.label(RichText::new("👆 Arama yapin").color(Color32::GRAY)); }

        self.cift_tiklama_ekle = false;
        ScrollArea::vertical().max_height(ui.available_height() - 50.0).show(ui, |ui| {
            for poz in pl.iter() {
                let secili = self.secili_poz.as_ref().map(|s| s.poz_no == poz.poz_no && s.kitap_id == poz.kitap_id).unwrap_or(false);
                let fm = match poz.fiyat { Some(f) => format!("{:.2}", f), None => "---".into() };
                let etiket = format!("{} | {} | {} | {}", poz.poz_no, poz.birim, fm, poz.kitap_adi);

                // Manuel seçili satır çizimi - egui selectable_label renk sorununu aşmak için
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 18.0),
                    egui::Sense::click(),
                );
                if secili {
                    ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(30, 100, 200));
                }
                let text_color = if secili { Color32::WHITE } else { ui.style().visuals.text_color() };
                ui.painter().text(
                    rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    &etiket,
                    egui::FontId::proportional(11.0),
                    text_color,
                );
                if response.clicked() {
                    self.secili_poz = Some(poz.clone());
                    self.yeni_poz_no = poz.poz_no.clone();
                }
                if response.double_clicked() {
                    self.secili_poz = Some(poz.clone());
                    self.yeni_poz_no = poz.poz_no.clone();
                    self.yeni_miktar.clear();
                    self.cift_tiklama_ekle = true;
                }
                response.on_hover_text(&format!("{}/{} | {}\nCift tikla: miktarsiz ekle", poz.ay, poz.yil, poz.tanim));
            }
        });
        if self.cift_tiklama_ekle {
            self.kalem_ekle();
        }
        ui.separator();

        if let Some(ref poz) = self.secili_poz {
            ui.heading("📌 Secili Poz");
            ui.label(RichText::new(format!("Poz No: {}", poz.poz_no)).strong().size(13.0));
            ui.label(format!("Kitap: {} ({}/{})", poz.kitap_adi, poz.ay, poz.yil));
            ui.label(format!("Aciklama: {}", poz.tanim));
            ui.label(format!("Birim: {}", poz.birim));
            match poz.fiyat { Some(f) => { ui.colored_label(Color32::GREEN, format!("Birim Fiyat: {:.2} TL", f)); } None => { ui.colored_label(Color32::RED, "Formul."); } }
            ui.label(format!("Kategori: {}", poz.kategori));
        }
    }

    fn render_metraj_listesi(&mut self, ui: &mut Ui) {
        ui.heading("📋 Metraj Tablosu");
        ui.horizontal(|ui| {
            ui.label("Metraj Adı:");
            if ui.add(TextEdit::singleline(&mut self.metraj_adi).hint_text("Metraj adi").desired_width(250.0)).changed() { self.degisiklik_var = true; }
        }); ui.separator();
        ui.horizontal(|ui| {
            ui.label("Poz No:");
            if ui.add(TextEdit::singleline(&mut self.yeni_poz_no).hint_text("15.100.1001").desired_width(140.0)).changed() { self.poz_sorgula(); }
            ui.label("Miktar:");
            ui.add(TextEdit::singleline(&mut self.yeni_miktar).hint_text("0.00").desired_width(80.0));
            if ui.button(RichText::new("➕ Kalem Ekle").color(Color32::WHITE)).highlight().clicked() { self.kalem_ekle(); }
        });
        // Fiyat güncelleme - hedef kitap seçerek tüm kalemleri yeni fiyatlarla güncelle
        if !self.metraj_kalemleri.is_empty() && self.kitaplar.len() > 1 {
            ui.horizontal(|ui| {
                ui.label("🔄 Toplu Fiyat Güncelle:");
                let hedef_metni = self.fiyat_guncelle_hedef.as_ref()
                    .map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil))
                    .unwrap_or_else(|| "Hedef kitap seçin".to_string());
                egui::ComboBox::from_id_salt("fiyat_guncelle_combo")
                    .selected_text(&hedef_metni)
                    .width(300.0)
                    .show_ui(ui, |ui| {
                        for k in &self.kitaplar.clone() {
                            if ui.selectable_label(false, format!("{} ({}/{})", k.ad, k.ay, k.yil)).clicked() {
                                self.fiyat_guncelle_hedef = Some(k.clone());
                            }
                        }
                    });
                if ui.button("✅ Güncelle").clicked() {
                    self.fiyatlari_guncelle();
                }
            });
            ui.add_space(2.0);
        }
        if let Some(ref poz) = self.secili_poz {
            if let Some(f) = poz.fiyat { let tt = self.yeni_miktar.parse::<f64>().unwrap_or(0.0) * f; ui.label(format!("{} | {} TL x {} = {:.2} TL", poz.tanim, f, self.yeni_miktar, tt)); }
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("📂 Ac (.mrj)").clicked() { self.metraj_yukle_diyalog(); }
            let lbl = if self.mevcut_dosya_yolu.is_some() { "💾 Kaydet (Ctrl+S)" } else { "💾 Farkli Kaydet (.mrj)" };
            if ui.button(lbl).clicked() { self.metraj_kaydet(); }
            if self.degisiklik_var { ui.colored_label(Color32::YELLOW, "●"); }
            if ui.button("📊 Excel").clicked() { self.metraj_excel_diyalog(); }
            if ui.button("🗑 Temizle").clicked() { self.metraj_kalemleri.clear(); self.degisiklik_var = true; self.basarili_mesaj = "Temizlendi.".into(); }
        }); ui.separator();
        ScrollArea::vertical().max_height(ui.available_height() - 80.0).show(ui, |ui| { self.render_metraj_kalem_tablosu(ui); });
        ui.separator();
        ui.horizontal(|ui| { ui.label(RichText::new(format!("GENEL TOPLAM: {:.2} TL", self.toplam_tutar())).size(16.0).strong().color(Color32::GREEN)); });
    }

    fn render_metraj_kalem_tablosu(&mut self, ui: &mut Ui) {
        if self.metraj_kalemleri.is_empty() { ui.label(RichText::new("Henuz kalem yok.").color(Color32::GRAY).size(13.0)); return; }
        egui::Grid::new("metraj_grid").num_columns(9).min_col_width(50.0).striped(true).show(ui, |ui: &mut egui::Ui| {
            ui.label(RichText::new("#").strong().size(12.0)); ui.label(RichText::new("Kitap (Ay/Yil)").strong().size(12.0));
            ui.label(RichText::new("Poz No").strong().size(12.0)); ui.label(RichText::new("Aciklama").strong().size(12.0));
            ui.label(RichText::new("Birim").strong().size(12.0)); ui.label(RichText::new("B.Fiyat").strong().size(12.0));
            ui.label(RichText::new("Miktar").strong().size(12.0)); ui.label(RichText::new("Tutar").strong().size(12.0));
            ui.label(RichText::new("").strong().size(12.0));
            ui.end_row();

            let mut sil: Option<usize> = None; let mut deg: Option<(usize, f64)> = None;
            for (idx, kalem) in self.metraj_kalemleri.iter_mut().enumerate() {
                ui.label(format!("{}", idx + 1));
                ui.label(RichText::new(format!("{}", &kalem.kitap_adi)).size(10.0));
                ui.label(RichText::new(&kalem.poz_no).size(11.0).monospace());
                let kisa = if kalem.tanim.len() > 35 { format!("{}...", &kalem.tanim[..32]) } else { kalem.tanim.clone() };
                ui.label(RichText::new(kisa).size(11.0)); ui.label(&kalem.birim); ui.label(format!("{:.2}", kalem.birim_fiyat));
                let mut ms = format!("{:.2}", kalem.miktar);
                if ui.add(TextEdit::singleline(&mut ms).desired_width(70.0)).changed() { if let Ok(y) = ms.parse::<f64>() { deg = Some((idx, y)); } }
                ui.label(RichText::new(format!("{:.2}", kalem.tutar)).size(11.0).strong().color(Color32::GREEN));
                if ui.button(RichText::new("✕").color(Color32::RED).size(11.0)).clicked() { sil = Some(idx); }
                ui.end_row();
            }
            if let Some(idx) = sil { self.metraj_kalemleri.remove(idx); self.degisiklik_var = true; }
            if let Some((idx, ym)) = deg { if idx < self.metraj_kalemleri.len() { self.metraj_kalemleri[idx].miktar = ym; self.metraj_kalemleri[idx].tutar_guncelle(); self.degisiklik_var = true; } }
        });
    }

    // ==================== POZLAR ====================
    fn render_pozlar_tablosu(&mut self, ui: &mut Ui) {
        if self.secili_kitap.as_ref().map(|k| k.id) != self.pozlar_yuklu_kitap_id {
            self.pozlar_tablosu_yenile();
        }

        ui.heading("🔎 Pozlar");
        ui.horizontal(|ui| {
            ui.label(RichText::new("Kitap:").strong());
            let km = self.secili_kitap.as_ref().map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil)).unwrap_or_else(|| "Kitap secin".into());
            egui::ComboBox::from_id_salt("pozlar_kitap_secici").selected_text(&km).width(350.0).show_ui(ui, |ui| {
                for k in self.kitaplar.clone() {
                    if ui.selectable_label(self.secili_kitap.as_ref().map(|sk| sk.id == k.id).unwrap_or(false), format!("{} ({}/{})", k.ad, k.ay, k.yil)).clicked() {
                        self.secili_kitap = Some(k);
                        self.pozlar_tablosu_yenile();
                    }
                }
            });
        });

        if self.secili_kitap.is_none() {
            ui.separator();
            ui.colored_label(Color32::YELLOW, "Poz listesini görmek için bir kitap seçin.");
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Hızlı Arama:");
            if ui.add_sized(Vec2::new(320.0, 24.0), TextEdit::singleline(&mut self.pozlar_arama_metni).hint_text("poz no, açıklama, birim veya kategori")).changed() {
                self.pozlar_tablosu_yenile();
            }
            if !self.pozlar_arama_metni.is_empty() && ui.button("Temizle").clicked() {
                self.pozlar_arama_metni.clear();
                self.pozlar_tablosu_yenile();
            }
            ui.label(format!("{} poz", self.pozlar_tablosu.len()));
        });
        ui.separator();

        if self.pozlar_tablosu.is_empty() {
            ui.label(RichText::new("Sonuç yok.").color(Color32::GRAY));
            return;
        }

        ScrollArea::vertical().max_height(ui.available_height()).show(ui, |ui| {
            egui::Grid::new("pozlar_grid").num_columns(6).min_col_width(60.0).striped(true).show(ui, |ui: &mut egui::Ui| {
                ui.label(RichText::new("Poz No").strong().size(12.0));
                ui.label(RichText::new("Açıklama").strong().size(12.0));
                ui.label(RichText::new("Birim").strong().size(12.0));
                ui.label(RichText::new("B.Fiyat").strong().size(12.0));
                ui.label(RichText::new("Kategori").strong().size(12.0));
                ui.label(RichText::new("Kitap").strong().size(12.0));
                ui.end_row();

                for poz in &self.pozlar_tablosu {
                    let fiyat = poz.fiyat.map(|f| format!("{:.2}", f)).unwrap_or_else(|| "---".into());
                    let aciklama = metni_kisalt(&poz.tanim, 85);
                    ui.label(RichText::new(&poz.poz_no).monospace().size(11.0));
                    ui.label(RichText::new(aciklama).size(11.0)).on_hover_text(&poz.tanim);
                    ui.label(RichText::new(&poz.birim).size(11.0));
                    ui.label(RichText::new(fiyat).size(11.0));
                    ui.label(RichText::new(&poz.kategori).size(10.0));
                    ui.label(RichText::new(format!("{}/{}", poz.ay, poz.yil)).size(10.0));
                    ui.end_row();
                }
            });
        });
    }

    // ==================== PDF YUKLE ====================
    fn render_pdf_yukle(&mut self, ui: &mut Ui) {
        ui.heading("📄 PDF Birim Fiyat Listesi Yükle"); ui.separator();
        ui.label("PDF'i hangi kitaba yukleyeceginizi secin.");
        if self.kitaplar.is_empty() { ui.colored_label(Color32::RED, "⚠ Once Kitap Yoneticisi'nden kitap ekleyin!"); return; }
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("Hedef Kitap:");
            let km = self.secili_kitap.as_ref().map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil)).unwrap_or_else(|| "Kitap secin".into());
            egui::ComboBox::from_id_salt("pdf_kitap_secici").selected_text(&km).width(350.0).show_ui(ui, |ui| {
                for k in &self.kitaplar.clone() { if ui.selectable_label(false, format!("{} ({}/{})", k.ad, k.ay, k.yil)).clicked() { self.secili_kitap = Some(k.clone()); } }
            });
        });
        ui.add_space(10.0);
        if self.pdf_yukleniyor { ui.spinner(); ui.label("PDF isleniyor..."); }
        else if ui.button(RichText::new("📂 PDF Dosyasi Sec ve Yukle").size(14.0)).clicked() { self.pdf_sec_ve_yukle(); }
        if !self.pdf_durumu.is_empty() { ui.add_space(5.0); ui.label(RichText::new(&self.pdf_durumu).color(Color32::GREEN)); }
        ui.separator(); ui.label("Hizli yukleme:");
        let alt = PathBuf::from("20206-05-BF.pdf");
        if alt.exists() && ui.button("📄 20206-05-BF.pdf").clicked() { self.pdf_yukle(alt); }
    }

    // ==================== YARDIMCI ====================
    fn toplam_tutar(&self) -> f64 { self.metraj_kalemleri.iter().map(|k| k.tutar).sum() }
    fn kitaplari_yenile(&mut self) { if let Some(ref db) = self.db { if let Ok(k) = db.kitaplari_listele() { self.kitaplar = k; } } }
    fn pozlar_tablosu_yenile(&mut self) {
        self.pozlar_tablosu.clear();
        self.pozlar_yuklu_kitap_id = self.secili_kitap.as_ref().map(|k| k.id);
        if let (Some(ref db), Some(ref kitap)) = (&self.db, &self.secili_kitap) {
            match db.pozlari_listele(kitap.id, &self.pozlar_arama_metni) {
                Ok(pozlar) => self.pozlar_tablosu = pozlar,
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }
    fn poz_no_ara(&mut self) { if self.poz_arama_metni.is_empty() { self.arama_sonuclari.clear(); return; } if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(s) = db.poz_no_ara(&self.poz_arama_metni, kid) { self.arama_sonuclari = s; } } }
    fn aciklama_ara(&mut self) { if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(s) = db.tam_metin_ara(&self.aciklama_arama_metni, kid) { self.arama_sonuclari = s; } } }
    fn poz_sorgula(&mut self) {
        if self.yeni_poz_no.is_empty() { self.secili_poz = None; return; }
        if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id);
            match db.poz_getir(&self.yeni_poz_no, kid) {
                Ok(Some(p)) => self.secili_poz = Some(p),
                Ok(None) => { if let Ok(s) = db.poz_no_ara(&self.yeni_poz_no, kid) { if s.len() == 1 { self.secili_poz = Some(s[0].clone()); self.yeni_poz_no = s[0].poz_no.clone(); } else { self.arama_sonuclari = s; } } }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }
    fn kalem_ekle(&mut self) {
        if let Some(ref poz) = self.secili_poz { let m = self.yeni_miktar.trim().parse::<f64>().unwrap_or(0.0); let kalem = MetrajKalemi::yeni(poz, m); let t = kalem.tutar; let bf = kalem.birim_fiyat; self.metraj_kalemleri.push(kalem);
            // Metrajı poz numarasına göre sırala (yeniden eklemede de sıralı kalır)
            self.metraj_kalemleri.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
            self.degisiklik_var = true; self.basarili_mesaj = if m == 0.0 { format!("{} eklendi.", poz.poz_no) } else { format!("{} eklendi ({} {} x {:.2} = {:.2} TL).", poz.poz_no, m, poz.birim, bf, t) }; self.yeni_miktar.clear(); self.hata_mesaji.clear(); }
        else { self.hata_mesaji = "Once bir poz secin.".into(); }
    }
    fn kategorileri_yukle(&mut self) { if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(k) = db.kategoriler(kid) { self.kategoriler = k; } } }
    fn kategori_filtrele(&mut self) { if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(t) = db.tum_pozlar(kid) { self.kategori_pozlar = t.into_iter().filter(|p| p.kategori == self.secili_kategori).collect(); } } }

    // ==================== DOSYA ====================
    fn pdf_sec_ve_yukle(&mut self) { if let Some(y) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() { self.pdf_yukle(y); } }
    fn pdf_yukle(&mut self, pdf_yolu: PathBuf) {
        let kitap = match self.secili_kitap.clone() { Some(k) => k, None => { self.hata_mesaji = "Once hedef kitap secin!".into(); return; } };
        self.pdf_yukleniyor = true; self.pdf_durumu = format!("PDF okunuyor...");
        match pdf_metin_cikar(&pdf_yolu) {
            Ok(metin) => {
                let pozlar = pozlari_ayristir(&metin, kitap.id, &kitap.ad, kitap.yil, kitap.ay);
                self.pdf_durumu = format!("{} poz ayrıştırıldı.", pozlar.len());
                if let Some(ref db) = self.db { match db.pozlari_yukle(kitap.id, &kitap, &pozlar) {
                    Ok(sayi) => { self.poz_sayisi = db.poz_sayisi().unwrap_or(0); self.basarili_mesaj = format!("✅ {} ({}/{}) kitabina {} poz yuklendi!", kitap.ad, kitap.ay, kitap.yil, sayi); self.pdf_durumu = format!("✅ {} poz yuklendi.", sayi); if let Ok(Some(yk)) = db.kitap_getir(kitap.id) { self.secili_kitap = Some(yk); } self.kitaplari_yenile(); self.pozlar_tablosu_yenile(); }
                    Err(e) => self.hata_mesaji = format!("{}", e),
                }}
            }
            Err(e) => self.hata_mesaji = format!("{}", e),
        }
        self.pdf_yukleniyor = false;
    }
    fn metraj_kaydet(&mut self) {
        let m = KayitliMetraj { ad: self.metraj_adi.clone(), kalemler: self.metraj_kalemleri.clone(), tarih: krono_tarih() };
        if let Some(ref y) = self.mevcut_dosya_yolu { match metraj_json_kaydet(&m, y) { Ok(()) => { self.degisiklik_var = false; self.basarili_mesaj = format!("Kaydedildi: {}", y.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } }
        else if let Some(d) = rfd::FileDialog::new().add_filter("Metrajmatik Projesi", &["mrj"]).set_file_name(&format!("{}.mrj", self.metraj_adi)).save_file() { match metraj_json_kaydet(&m, &d) { Ok(()) => { self.mevcut_dosya_yolu = Some(d.clone()); self.degisiklik_var = false; self.basarili_mesaj = format!("Kaydedildi: {}", d.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } }
    }
    fn metraj_yukle_diyalog(&mut self) { if let Some(d) = rfd::FileDialog::new().add_filter("Metrajmatik Projesi", &["mrj","json"]).pick_file() { match metraj_json_yukle(&d) { Ok(m) => { self.metraj_kalemleri = m.kalemler; self.metraj_adi = m.ad; self.mevcut_dosya_yolu = Some(d.clone()); self.degisiklik_var = false; self.basarili_mesaj = format!("Acildi: {}", d.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } } }
    fn metraj_excel_diyalog(&mut self) { let m = KayitliMetraj { ad: self.metraj_adi.clone(), kalemler: self.metraj_kalemleri.clone(), tarih: krono_tarih() }; if let Some(d) = rfd::FileDialog::new().add_filter("Excel", &["xlsx"]).set_file_name(&format!("{}.xlsx", self.metraj_adi)).save_file() { match metraj_excel_aktar(&m, &d) { Ok(()) => { self.basarili_mesaj = format!("Excel: {}", d.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } } }

    fn fiyatlari_guncelle(&mut self) {
        let hedef_kitap = match self.fiyat_guncelle_hedef.clone() {
            Some(k) => k,
            None => { self.hata_mesaji = "Lutfen hedef kitap secin!".into(); return; }
        };
        if let Some(ref db) = self.db {
            let mut guncellenen = 0;
            let mut bulunamayan = 0;
            // Kitap bazlı sayaç: (eski_kitap_adi, guncellenen, bulunamayan)
            let mut kitap_bazli: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();

            for kalem in self.metraj_kalemleri.iter_mut() {
                let eski_kitap = kalem.kitap_adi.clone();
                // Hedef kitapta aynı poz_no'yu ara
                if let Ok(Some(poz)) = db.poz_getir(&kalem.poz_no, Some(hedef_kitap.id)) {
                    if let Some(yeni_fiyat) = poz.fiyat {
                        kalem.birim_fiyat = yeni_fiyat;
                        kalem.kitap_adi = format!("{} ({}/{})", hedef_kitap.ad, hedef_kitap.ay, hedef_kitap.yil);
                        kalem.tutar_guncelle();
                        guncellenen += 1;
                        let entry = kitap_bazli.entry(eski_kitap).or_insert((0, 0));
                        entry.0 += 1;
                    }
                } else {
                    bulunamayan += 1;
                    let entry = kitap_bazli.entry(eski_kitap).or_insert((0, 0));
                    entry.1 += 1;
                }
            }
            self.degisiklik_var = true;
            self.fiyat_guncelle_hedef = None;

            let mut detay = String::new();
            for (kitap, (g, b)) in &kitap_bazli {
                if *g > 0 || *b > 0 {
                    detay.push_str(&format!("\n  📦 {}: {} güncellendi, {} bulunamadı", kitap, g, b));
                }
            }
            self.basarili_mesaj = format!(
                "✅ {} kalem güncellendi (→ {} fiyatlarıyla). {} kalem hedef kitapta bulunamadı.{}",
                guncellenen, hedef_kitap.ad, bulunamayan, detay
            );
        }
    }
}

fn krono_tarih() -> String {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let d = s / 86400; let y = 1970 + d / 365; let r = d % 365;
    format!("{:04}-{:02}-{:02}", y, r / 30 + 1, r % 30 + 1)
}

fn metni_kisalt(metin: &str, en_fazla: usize) -> String {
    if metin.chars().count() <= en_fazla {
        return metin.to_string();
    }
    let govde: String = metin.chars().take(en_fazla.saturating_sub(3)).collect();
    format!("{}...", govde)
}
