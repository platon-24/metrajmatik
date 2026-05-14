use eframe::egui;
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui, Vec2};
use std::path::PathBuf;

use crate::database::Veritabani;
use crate::export::{metraj_excel_aktar, metraj_json_kaydet, metraj_json_yukle};
use crate::models::{KayitliMetraj, Kitap, MetrajKalemi, Poz};
use crate::pdf_parser::{pdf_metin_cikar, pozlari_ayristir};

#[derive(Debug, Clone, PartialEq)]
enum Sekme {
    MetrajTablosu,
    KitapYoneticisi,
    PdfYukle,
}

pub struct MetrajApp {
    db: Option<Veritabani>,
    poz_sayisi: u32,

    // Kitap yönetimi
    kitaplar: Vec<Kitap>,
    secili_kitap: Option<Kitap>,

    // Metraj tablosu
    metraj_kalemleri: Vec<MetrajKalemi>,
    metraj_adi: String,
    mevcut_dosya_yolu: Option<PathBuf>,
    degisiklik_var: bool,

    // Poz arama
    poz_arama_metni: String,
    arama_sonuclari: Vec<Poz>,
    secili_poz: Option<Poz>,

    aciklama_arama_metni: String,
    yeni_poz_no: String,
    yeni_miktar: String,

    // Kitap ekleme
    yeni_kitap_adi: String,

    // PDF
    pdf_durumu: String,
    pdf_yukleniyor: bool,
    son_pdf_adi: String,

    aktif_sekme: Sekme,
    hata_mesaji: String,
    basarili_mesaj: String,

    kategoriler: Vec<String>,
    secili_kategori: String,
    kategori_pozlar: Vec<Poz>,
}

impl Default for MetrajApp {
    fn default() -> Self {
        let db_yolu = PathBuf::from("metrajmatik_veriler.db");
        let (db, poz_sayisi, kitaplar) = match Veritabani::ac(&db_yolu) {
            Ok(vt) => {
                let sayi = vt.poz_sayisi().unwrap_or(0);
                let ktp = vt.kitaplari_listele().unwrap_or_default();
                (Some(vt), sayi, ktp)
            }
            Err(e) => {
                log::error!("Veritabani acilamadi: {}", e);
                (None, 0, Vec::new())
            }
        };

        Self {
            db,
            poz_sayisi,
            kitaplar,
            secili_kitap: None,
            metraj_kalemleri: Vec::new(),
            metraj_adi: String::from("Isimsiz Metraj"),
            mevcut_dosya_yolu: None,
            degisiklik_var: false,
            poz_arama_metni: String::new(),
            arama_sonuclari: Vec::new(),
            secili_poz: None,
            aciklama_arama_metni: String::new(),
            yeni_poz_no: String::new(),
            yeni_miktar: String::new(),
            yeni_kitap_adi: String::new(),
            pdf_durumu: String::new(),
            pdf_yukleniyor: false,
            son_pdf_adi: String::new(),
            aktif_sekme: Sekme::MetrajTablosu,
            hata_mesaji: String::new(),
            basarili_mesaj: String::new(),
            kategoriler: Vec::new(),
            secili_kategori: String::from("TÜMÜ"),
            kategori_pozlar: Vec::new(),
        }
    }
}

impl eframe::App for MetrajApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.metraj_kaydet();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            self.metraj_yukle_diyalog();
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(44, 62, 80);
                ui.style_mut().visuals.widgets.active.weak_bg_fill = Color32::from_rgb(52, 73, 94);
                ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(52, 73, 94);

                let baslik = if let Some(ref yol) = self.mevcut_dosya_yolu {
                    format!("🏗 METRAJMATIK - {}", yol.file_name().unwrap().to_string_lossy())
                } else {
                    "🏗 METRAJMATIK".to_string()
                };
                ui.label(RichText::new(baslik).color(Color32::WHITE).size(18.0).strong());
                ui.separator();

                let sekmeler = [Sekme::MetrajTablosu, Sekme::KitapYoneticisi, Sekme::PdfYukle];
                let isimler = ["📋 Metraj", "📚 Kitaplar", "📄 PDF Yükle"];
                for i in 0..3 {
                    let s = &sekmeler[i];
                    if ui
                        .selectable_label(self.aktif_sekme == *s, RichText::new(isimler[i]).color(Color32::WHITE))
                        .clicked()
                    {
                        self.aktif_sekme = s.clone();
                        if *s == Sekme::MetrajTablosu || *s == Sekme::KitapYoneticisi {
                            self.kitaplari_yenile();
                        }
                        if *s == Sekme::MetrajTablosu {
                            self.kategorileri_yukle();
                        }
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.hata_mesaji.is_empty() {
                ui.colored_label(Color32::RED, &self.hata_mesaji);
                if ui.button("✕").clicked() { self.hata_mesaji.clear(); }
            }
            if !self.basarili_mesaj.is_empty() {
                ui.colored_label(Color32::GREEN, &self.basarili_mesaj);
            }
            match self.aktif_sekme {
                Sekme::MetrajTablosu => self.render_metraj_tablosu(ui),
                Sekme::KitapYoneticisi => self.render_kitap_yoneticisi(ui),
                Sekme::PdfYukle => self.render_pdf_yukle(ui),
            }
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let kitap_info = if let Some(ref k) = self.secili_kitap {
                format!("📚 {} | ", k.ad)
            } else {
                String::new()
            };
            let dosya_adi = self.mevcut_dosya_yolu.as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "Kaydedilmemis".to_string());
            ui.label(format!(
                "📁 {} {}| {}{} poz | 📋 {} kalem | 💰 {:.2} TL",
                dosya_adi,
                if self.degisiklik_var { "● " } else { "" },
                kitap_info,
                self.poz_sayisi,
                self.metraj_kalemleri.len(),
                self.toplam_tutar()
            ));
        });
    }
}

impl MetrajApp {
    // ==================== KITAP YONETICISI ====================

    fn render_kitap_yoneticisi(&mut self, ui: &mut Ui) {
        ui.heading("📚 Kitap Yöneticisi (Birim Fiyat Listeleri)");
        ui.separator();

        // Yeni kitap ekleme
        ui.horizontal(|ui| {
            ui.label("Yeni Kitap Adı:");
            ui.add(TextEdit::singleline(&mut self.yeni_kitap_adi)
                .hint_text("örn: Cevre ve Sehircilik Bakanligi")
                .desired_width(300.0));
            let ekle_clicked = ui.button("➕ Kitap Ekle").clicked();
            if ekle_clicked {
                let ad = self.yeni_kitap_adi.trim().to_string();
                if ad.is_empty() {
                    self.hata_mesaji = "Lutfen bir kitap adi girin.".to_string();
                } else if let Some(ref db) = self.db {
                    match db.kitap_ekle(&ad) {
                        Ok(_id) => {
                            self.basarili_mesaj = format!("'{}' kitabi eklendi. Simdi PDF Yukle sekmesinden pozlari yukleyin.", ad);
                            self.yeni_kitap_adi.clear();
                            self.kitaplari_yenile();
                        }
                        Err(e) => self.hata_mesaji = format!("Veritabani hatasi: {}", e),
                    }
                } else {
                    self.hata_mesaji = "Veritabani acik degil!".to_string();
                }
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new("Yüklü Kitaplar:").strong().size(14.0));
        ui.add_space(5.0);

        if self.kitaplar.is_empty() {
            ui.label("Henüz kitap eklenmedi. Yukarıdan bir kitap adı girip 'Kitap Ekle'ye tıklayın.");
            ui.label("Ardından PDF Yükle sekmesinden bu kitaba ait birim fiyat PDF'ini yükleyin.");
            return;
        }

        // Kitap listesi tablosu
        egui::Grid::new("kitap_grid")
            .num_columns(5)
            .min_col_width(100.0)
            .striped(true)
            .show(ui, |ui: &mut egui::Ui| {
                ui.label(RichText::new("ID").strong());
                ui.label(RichText::new("Kitap Adı").strong());
                ui.label(RichText::new("Poz Sayısı").strong());
                ui.label(RichText::new("Tarih").strong());
                ui.label(RichText::new("İşlem").strong());
                ui.end_row();

                let kitaplar_snapshot = self.kitaplar.clone();
                for kitap in &kitaplar_snapshot {
                    let secili = self.secili_kitap.as_ref().map(|k| k.id == kitap.id).unwrap_or(false);
                    ui.label(if secili { RichText::new(format!("{}", kitap.id)).color(Color32::GREEN) } else { RichText::new(format!("{}", kitap.id)) });
                    if ui.selectable_label(secili, &kitap.ad).clicked() {
                        self.secili_kitap = Some(kitap.clone());
                        self.kategorileri_yukle();
                        self.basarili_mesaj = format!("{} seçildi. Metraj sekmesinde bu kitabın pozları aranır.", kitap.ad);
                    }
                    ui.label(format!("{}", kitap.poz_sayisi));
                    ui.label(&kitap.tarih);
                    let kitap_id = kitap.id;
                    let kitap_ad = kitap.ad.clone();
                    if ui.button(RichText::new("🗑 Sil").color(Color32::RED)).clicked() {
                        if let Some(ref db) = self.db {
                            if let Err(e) = db.kitap_sil(kitap_id) {
                                self.hata_mesaji = format!("{}", e);
                            } else {
                                if self.secili_kitap.as_ref().map(|k| k.id == kitap_id).unwrap_or(false) {
                                    self.secili_kitap = None;
                                }
                                self.basarili_mesaj = format!("{} silindi.", kitap_ad);
                                self.kitaplari_yenile();
                            }
                        }
                    }
                    ui.end_row();
                }
            });

        ui.separator();
        if let Some(ref k) = self.secili_kitap {
            ui.colored_label(Color32::GREEN, format!("✅ Aktif kitap: {} ({} poz)", k.ad, k.poz_sayisi));
        }
        ui.label("💡 Kitap ekledikten sonra PDF Yükle sekmesine geçip ilgili PDF'i yükleyin.");
    }

    // ==================== METRAJ TABLOSU ====================

    fn render_metraj_tablosu(&mut self, ui: &mut Ui) {
        // Kitap seçici (üstte)
        ui.horizontal(|ui| {
            ui.label(RichText::new("Kitap:").strong());
            let kitap_metni = self.secili_kitap.as_ref()
                .map(|k| format!("{} ({} poz)", k.ad, k.poz_sayisi))
                .unwrap_or_else(|| "TÜM KITAPLAR".to_string());
            egui::ComboBox::from_id_salt("kitap_secici")
                .selected_text(&kitap_metni)
                .width(350.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.secili_kitap.is_none(), "TÜM KİTAPLAR").clicked() {
                        self.secili_kitap = None;
                        self.kategorileri_yukle();
                    }
                    for k in self.kitaplar.clone() {
                        if ui.selectable_label(false, format!("{} ({} poz)", k.ad, k.poz_sayisi)).clicked() {
                            self.secili_kitap = Some(k);
                            self.kategorileri_yukle();
                        }
                    }
                });
        });
        ui.separator();

        egui::SidePanel::left("sol_panel")
            .resizable(true)
            .default_width(400.0)
            .min_width(300.0)
            .show_inside(ui, |ui| { self.render_arama_paneli(ui); });

        egui::CentralPanel::default().show_inside(ui, |ui| { self.render_metraj_listesi(ui); });
    }

    fn render_arama_paneli(&mut self, ui: &mut Ui) {
        ui.heading("🔍 Poz Arama");
        ui.horizontal(|ui| {
            ui.label("Poz No:");
            if ui.add_sized(Vec2::new(200.0, 24.0), TextEdit::singleline(&mut self.poz_arama_metni).hint_text("örn: 15.100")).changed() {
                self.poz_no_ara();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Açıklama:");
            if ui.add_sized(Vec2::new(200.0, 24.0), TextEdit::singleline(&mut self.aciklama_arama_metni).hint_text("örn: beton, tugla...")).changed() && !self.aciklama_arama_metni.is_empty() {
                self.aciklama_ara();
            }
        });
        if !self.kategoriler.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Kategori:");
                egui::ComboBox::from_id_salt("kategori_combo").selected_text(&self.secili_kategori).width(200.0).show_ui(ui, |ui| {
                    if ui.selectable_label(false, "TÜMÜ").clicked() { self.secili_kategori = "TÜMÜ".to_string(); self.kategori_pozlar.clear(); }
                    for kat in &self.kategoriler.clone() {
                        if ui.selectable_label(false, kat).clicked() { self.secili_kategori = kat.clone(); self.kategori_filtrele(); }
                    }
                });
            });
        }
        ui.separator();

        let poz_listesi = if !self.kategori_pozlar.is_empty() { &self.kategori_pozlar } else { &self.arama_sonuclari };
        if !poz_listesi.is_empty() {
            ui.label(format!("{} sonuç", poz_listesi.len()));
        } else if !self.poz_arama_metni.is_empty() || !self.aciklama_arama_metni.is_empty() {
            ui.label("Sonuç bulunamadi.");
        } else {
            ui.label(RichText::new("👆 Poz no veya açıklama ile arama yapın").color(Color32::GRAY));
        }

        ScrollArea::vertical().max_height(ui.available_height() - 50.0).show(ui, |ui| {
            for poz in poz_listesi.iter() {
                let secili = self.secili_poz.as_ref().map(|s| s.poz_no == poz.poz_no && s.kitap_id == poz.kitap_id).unwrap_or(false);
                let fiyat_metin = match poz.fiyat { Some(f) => format!("{:.2} TL", f), None => "---".to_string() };
                let etiket = format!("{} | {} | {} | {}", poz.poz_no, poz.birim, fiyat_metin, poz.kitap_adi);
                let cevap = ui.selectable_label(secili, RichText::new(&etiket).size(11.0));
                if cevap.clicked() { self.secili_poz = Some(poz.clone()); self.yeni_poz_no = poz.poz_no.clone(); }
                cevap.on_hover_text(&poz.tanim);
            }
        });
        ui.separator();

        if let Some(ref poz) = self.secili_poz {
            ui.heading("📌 Seçili Poz");
            ui.colored_label(Color32::DARK_BLUE, format!("Poz No: {}", poz.poz_no));
            ui.label(format!("Kitap: {}", poz.kitap_adi));
            ui.label(format!("Açıklama: {}", poz.tanim));
            ui.label(format!("Birim: {}", poz.birim));
            match poz.fiyat {
                Some(f) => { ui.colored_label(Color32::GREEN, format!("Birim Fiyat: {:.2} TL", f)); }
                None => { ui.colored_label(Color32::RED, "Bu poz formül içermektedir."); }
            }
            ui.label(format!("Kategori: {}", poz.kategori));
        }
    }

    fn render_metraj_listesi(&mut self, ui: &mut Ui) {
        ui.heading("📋 Metraj Tablosu");
        ui.horizontal(|ui| {
            ui.label("Metraj Adı:");
            if ui.add(TextEdit::singleline(&mut self.metraj_adi).hint_text("Metraj adı").desired_width(250.0)).changed() {
                self.degisiklik_var = true;
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Poz No:");
            if ui.add(TextEdit::singleline(&mut self.yeni_poz_no).hint_text("15.100.1001").desired_width(140.0)).changed() { self.poz_sorgula(); }
            ui.label("Miktar:");
            ui.add(TextEdit::singleline(&mut self.yeni_miktar).hint_text("0.00").desired_width(80.0));
            if ui.button(RichText::new("➕ Kalem Ekle").color(Color32::WHITE)).highlight().clicked() { self.kalem_ekle(); }
        });

        if let Some(ref poz) = self.secili_poz {
            if let Some(fiyat) = poz.fiyat {
                let tt = self.yeni_miktar.parse::<f64>().unwrap_or(0.0) * fiyat;
                ui.label(format!("{} | {} | {:.2} TL x {} = {:.2} TL", poz.tanim, poz.birim, fiyat, self.yeni_miktar, tt));
            }
        }
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("📂 Aç (.mrj)").clicked() { self.metraj_yukle_diyalog(); }
            let lbl = if self.mevcut_dosya_yolu.is_some() { "💾 Kaydet (Ctrl+S)" } else { "💾 Farklı Kaydet (.mrj)" };
            if ui.button(lbl).clicked() { self.metraj_kaydet(); }
            if self.degisiklik_var { ui.colored_label(Color32::YELLOW, "●"); }
            if ui.button("📊 Excel").clicked() { self.metraj_excel_diyalog(); }
            if ui.button("🗑 Temizle").clicked() { self.metraj_kalemleri.clear(); self.degisiklik_var = true; self.basarili_mesaj = "Metraj temizlendi.".to_string(); }
        });
        ui.separator();

        ScrollArea::vertical().max_height(ui.available_height() - 80.0).show(ui, |ui| { self.render_metraj_kalem_tablosu(ui); });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("GENEL TOPLAM: {:.2} TL", self.toplam_tutar())).size(16.0).strong().color(Color32::GREEN));
        });
    }

    fn render_metraj_kalem_tablosu(&mut self, ui: &mut Ui) {
        if self.metraj_kalemleri.is_empty() {
            ui.label(RichText::new("Henüz metraj kalemi eklenmedi.").color(Color32::GRAY).size(13.0));
            return;
        }
        egui::Grid::new("metraj_grid").num_columns(9).min_col_width(50.0).striped(true).show(ui, |ui: &mut egui::Ui| {
            ui.label(RichText::new("#").strong().size(12.0));
            ui.label(RichText::new("Kitap").strong().size(12.0));
            ui.label(RichText::new("Poz No").strong().size(12.0));
            ui.label(RichText::new("Açıklama").strong().size(12.0));
            ui.label(RichText::new("Birim").strong().size(12.0));
            ui.label(RichText::new("B.Fiyat").strong().size(12.0));
            ui.label(RichText::new("Miktar").strong().size(12.0));
            ui.label(RichText::new("Tutar").strong().size(12.0));
            ui.label(RichText::new("").strong().size(12.0));
            ui.end_row();

            let mut silinecek: Option<usize> = None;
            let mut degisecek: Option<(usize, f64)> = None;
            for (idx, kalem) in self.metraj_kalemleri.iter_mut().enumerate() {
                ui.label(format!("{}", idx + 1));
                ui.label(RichText::new(&kalem.kitap_adi).size(10.0));
                ui.label(RichText::new(&kalem.poz_no).size(11.0).monospace());
                let kisa = if kalem.tanim.len() > 35 { format!("{}...", &kalem.tanim[..32]) } else { kalem.tanim.clone() };
                ui.label(RichText::new(kisa).size(11.0));
                ui.label(&kalem.birim);
                ui.label(format!("{:.2}", kalem.birim_fiyat));
                let mut ms = format!("{:.2}", kalem.miktar);
                if ui.add(TextEdit::singleline(&mut ms).desired_width(70.0)).changed() {
                    if let Ok(y) = ms.parse::<f64>() { degisecek = Some((idx, y)); }
                }
                ui.label(RichText::new(format!("{:.2}", kalem.tutar)).size(11.0).strong().color(Color32::GREEN));
                if ui.button(RichText::new("✕").color(Color32::RED).size(11.0)).clicked() { silinecek = Some(idx); }
                ui.end_row();
            }
            if let Some(idx) = silinecek { self.metraj_kalemleri.remove(idx); self.degisiklik_var = true; }
            if let Some((idx, ym)) = degisecek {
                if idx < self.metraj_kalemleri.len() { self.metraj_kalemleri[idx].miktar = ym; self.metraj_kalemleri[idx].tutar_guncelle(); self.degisiklik_var = true; }
            }
        });
    }

    // ==================== PDF YUKLE ====================

    fn render_pdf_yukle(&mut self, ui: &mut Ui) {
        ui.heading("📄 PDF Birim Fiyat Listesi Yükle");
        ui.separator();
        ui.label("Bu ekrandan kitaplara ait birim fiyat PDF'lerini yükleyin.");
        ui.label("Önce Kitap Yöneticisi'nden bir kitap ekleyin, sonra buraya gelip PDF'i seçin.");

        if self.kitaplar.is_empty() {
            ui.add_space(10.0);
            ui.colored_label(Color32::RED, "⚠ Önce Kitap Yöneticisi sekmesinden bir kitap ekleyin!");
            return;
        }

        ui.add_space(5.0);
        // Hedef kitap seçimi
        ui.horizontal(|ui| {
            ui.label("Hedef Kitap:");
            let kitap_metni = self.secili_kitap.as_ref()
                .map(|k| format!("{} ({} poz)", k.ad, k.poz_sayisi))
                .unwrap_or_else(|| "Kitap seçin".to_string());
            egui::ComboBox::from_id_salt("pdf_kitap_secici")
                .selected_text(&kitap_metni)
                .width(350.0)
                .show_ui(ui, |ui| {
                    for k in &self.kitaplar.clone() {
                        if ui.selectable_label(false, format!("{} ({} poz)", k.ad, k.poz_sayisi)).clicked() {
                            self.secili_kitap = Some(k.clone());
                        }
                    }
                });
        });

        ui.add_space(10.0);
        if self.pdf_yukleniyor {
            ui.spinner();
            ui.label("PDF işleniyor...");
        } else if ui.button(RichText::new("📂 PDF Dosyası Seç ve Yükle").size(14.0)).clicked() {
            self.pdf_sec_ve_yukle();
        }

        if !self.pdf_durumu.is_empty() {
            ui.add_space(5.0);
            ui.label(RichText::new(&self.pdf_durumu).color(Color32::GREEN));
        }

        ui.separator();
        ui.label("Hızlı yükleme:");
        let varsayilan_pdf = PathBuf::from("..\\20206-05-BF.pdf");
        let alt_pdf = PathBuf::from("20206-05-BF.pdf");
        if varsayilan_pdf.exists() {
            if ui.button(format!("📄 {}", varsayilan_pdf.file_name().unwrap().to_string_lossy())).clicked() {
                self.pdf_yukle(varsayilan_pdf.clone());
            }
        } else if alt_pdf.exists() {
            if ui.button("📄 20206-05-BF.pdf").clicked() {
                self.pdf_yukle(alt_pdf);
            }
        }
    }

    // ==================== YARDIMCI METODLAR ====================

    fn toplam_tutar(&self) -> f64 { self.metraj_kalemleri.iter().map(|k| k.tutar).sum() }

    fn kitaplari_yenile(&mut self) {
        if let Some(ref db) = self.db {
            if let Ok(kitaplar) = db.kitaplari_listele() { self.kitaplar = kitaplar; }
        }
    }

    fn poz_no_ara(&mut self) {
        if self.poz_arama_metni.is_empty() { self.arama_sonuclari.clear(); return; }
        if let Some(ref db) = self.db {
            let kitap_id = self.secili_kitap.as_ref().map(|k| k.id);
            if let Ok(s) = db.poz_no_ara(&self.poz_arama_metni, kitap_id) { self.arama_sonuclari = s; }
        }
    }

    fn aciklama_ara(&mut self) {
        if let Some(ref db) = self.db {
            let kitap_id = self.secili_kitap.as_ref().map(|k| k.id);
            if let Ok(s) = db.tam_metin_ara(&self.aciklama_arama_metni, kitap_id) { self.arama_sonuclari = s; }
        }
    }

    fn poz_sorgula(&mut self) {
        if self.yeni_poz_no.is_empty() { self.secili_poz = None; return; }
        if let Some(ref db) = self.db {
            let kitap_id = self.secili_kitap.as_ref().map(|k| k.id);
            match db.poz_getir(&self.yeni_poz_no, kitap_id) {
                Ok(Some(poz)) => self.secili_poz = Some(poz),
                Ok(None) => {
                    if let Ok(s) = db.poz_no_ara(&self.yeni_poz_no, kitap_id) {
                        if s.len() == 1 { self.secili_poz = Some(s[0].clone()); self.yeni_poz_no = s[0].poz_no.clone(); }
                        else { self.arama_sonuclari = s; self.secili_poz = None; }
                    }
                }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }

    fn kalem_ekle(&mut self) {
        if let Some(ref poz) = self.secili_poz {
            let miktar = self.yeni_miktar.trim().parse::<f64>().unwrap_or(0.0);
            let kalem = MetrajKalemi::yeni(poz, miktar);
            let tutar = kalem.tutar;
            let bf = kalem.birim_fiyat;
            self.metraj_kalemleri.push(kalem);
            self.degisiklik_var = true;
            self.basarili_mesaj = if miktar == 0.0 {
                format!("{} eklendi. Miktarı tablodan giriniz.", poz.poz_no)
            } else {
                format!("{} eklendi ({} {} x {:.2} TL = {:.2} TL).", poz.poz_no, miktar, poz.birim, bf, tutar)
            };
            self.yeni_miktar.clear();
            self.hata_mesaji.clear();
        } else {
            self.hata_mesaji = "Lütfen önce bir poz seçin.".to_string();
        }
    }

    fn kategorileri_yukle(&mut self) {
        if let Some(ref db) = self.db {
            let kitap_id = self.secili_kitap.as_ref().map(|k| k.id);
            if let Ok(kat) = db.kategoriler(kitap_id) { self.kategoriler = kat; }
        }
    }

    fn kategori_filtrele(&mut self) {
        if let Some(ref db) = self.db {
            let kitap_id = self.secili_kitap.as_ref().map(|k| k.id);
            if let Ok(tumu) = db.tum_pozlar(kitap_id) {
                self.kategori_pozlar = tumu.into_iter().filter(|p| p.kategori == self.secili_kategori).collect();
            }
        }
    }

    // ==================== DOSYA ISLEMLERI ====================

    fn pdf_sec_ve_yukle(&mut self) {
        if let Some(yol) = rfd::FileDialog::new().add_filter("PDF Dosyaları", &["pdf"]).pick_file() {
            self.pdf_yukle(yol);
        }
    }

    fn pdf_yukle(&mut self, pdf_yolu: PathBuf) {
        let hedef_kitap = match self.secili_kitap.clone() {
            Some(k) => k,
            None => { self.hata_mesaji = "Lütfen önce bir hedef kitap seçin!".to_string(); return; }
        };

        self.pdf_yukleniyor = true;
        self.pdf_durumu = format!("PDF okunuyor: {}", pdf_yolu.display());
        self.son_pdf_adi = pdf_yolu.file_name().unwrap().to_string_lossy().to_string();

        match pdf_metin_cikar(&pdf_yolu) {
            Ok(metin) => {
                self.pdf_durumu = format!("{} satır metin çıkarıldı.", metin.lines().count());
                let pozlar = pozlari_ayristir(&metin, hedef_kitap.id, &hedef_kitap.ad);
                self.pdf_durumu = format!("{} poz ayrıştırıldı.", pozlar.len());
                if let Some(ref db) = self.db {
                    let yukleme_sonucu = db.pozlari_yukle(hedef_kitap.id, &hedef_kitap.ad, &pozlar);
                    match yukleme_sonucu {
                        Ok(sayi) => {
                            self.poz_sayisi = db.poz_sayisi().unwrap_or(0);
                            self.basarili_mesaj = format!("✅ {} kitabına {} poz yüklendi!", hedef_kitap.ad, sayi);
                            self.pdf_durumu = format!("✅ {} - {} poz yüklendi.", hedef_kitap.ad, sayi);
                            if let Ok(Some(yeni_kitap)) = db.kitap_getir(hedef_kitap.id) {
                                self.secili_kitap = Some(yeni_kitap);
                            }
                            self.kitaplari_yenile();
                        }
                        Err(e) => { self.hata_mesaji = format!("{}", e); }
                    }
                }
            }
            Err(e) => { self.hata_mesaji = format!("PDF hatası: {}", e); }
        }
        self.pdf_yukleniyor = false;
    }

    fn metraj_kaydet(&mut self) {
        let metraj = KayitliMetraj { ad: self.metraj_adi.clone(), kalemler: self.metraj_kalemleri.clone(), tarih: krono_tarih() };
        if let Some(ref yol) = self.mevcut_dosya_yolu {
            match metraj_json_kaydet(&metraj, yol) {
                Ok(()) => { self.degisiklik_var = false; self.basarili_mesaj = format!("Kaydedildi: {}", yol.display()); }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        } else if let Some(dosya) = rfd::FileDialog::new().add_filter("Metrajmatik Projesi", &["mrj"]).set_file_name(&format!("{}.mrj", self.metraj_adi)).save_file() {
            match metraj_json_kaydet(&metraj, &dosya) {
                Ok(()) => { self.mevcut_dosya_yolu = Some(dosya.clone()); self.degisiklik_var = false; self.basarili_mesaj = format!("Kaydedildi: {}", dosya.display()); }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }

    fn metraj_yukle_diyalog(&mut self) {
        if let Some(dosya) = rfd::FileDialog::new().add_filter("Metrajmatik Projesi", &["mrj","json"]).pick_file() {
            match metraj_json_yukle(&dosya) {
                Ok(metraj) => { self.metraj_kalemleri = metraj.kalemler; self.metraj_adi = metraj.ad; self.mevcut_dosya_yolu = Some(dosya.clone()); self.degisiklik_var = false; self.basarili_mesaj = format!("Açıldı: {}", dosya.display()); }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }

    fn metraj_excel_diyalog(&mut self) {
        let metraj = KayitliMetraj { ad: self.metraj_adi.clone(), kalemler: self.metraj_kalemleri.clone(), tarih: krono_tarih() };
        if let Some(dosya) = rfd::FileDialog::new().add_filter("Excel", &["xlsx"]).set_file_name(&format!("{}.xlsx", self.metraj_adi)).save_file() {
            match metraj_excel_aktar(&metraj, &dosya) {
                Ok(()) => { self.basarili_mesaj = format!("Excel aktarıldı: {}", dosya.display()); }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }
}

fn krono_tarih() -> String {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let d = s / 86400;
    let y = 1970 + d / 365;
    let r = d % 365;
    format!("{:04}-{:02}-{:02}", y, r / 30 + 1, r % 30 + 1)
}