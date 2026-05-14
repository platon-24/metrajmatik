use eframe::egui;
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui, Vec2};
use std::path::PathBuf;

use crate::database::Veritabani;
use crate::export::{metraj_excel_aktar, metraj_json_kaydet, metraj_json_yukle};
use crate::models::{KayitliMetraj, MetrajKalemi, Poz};
use crate::pdf_parser::{pdf_metin_cikar, pozlari_ayristir};

#[derive(Debug, Clone, PartialEq)]
enum Sekme {
    MetrajTablosu,
    PdfYukle,
}

pub struct MetrajApp {
    db: Option<Veritabani>,
    poz_sayisi: u32,

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

    pdf_durumu: String,
    pdf_yukleniyor: bool,

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
        let (db, poz_sayisi) = match Veritabani::ac(&db_yolu) {
            Ok(vt) => {
                let sayi = vt.poz_sayisi().unwrap_or(0);
                (Some(vt), sayi)
            }
            Err(e) => {
                log::error!("Veritabani acilamadi: {}", e);
                (None, 0)
            }
        };

        Self {
            db,
            poz_sayisi,
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
            pdf_durumu: String::new(),
            pdf_yukleniyor: false,
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
        // Kısayollar
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
                ui.label(
                    RichText::new(baslik)
                        .color(Color32::WHITE)
                        .size(18.0)
                        .strong(),
                );
                ui.separator();

                if ui
                    .selectable_label(
                        self.aktif_sekme == Sekme::MetrajTablosu,
                        RichText::new("📋 Metraj Tablosu").color(Color32::WHITE),
                    )
                    .clicked()
                {
                    self.aktif_sekme = Sekme::MetrajTablosu;
                    self.kategorileri_yukle();
                }
                if ui
                    .selectable_label(
                        self.aktif_sekme == Sekme::PdfYukle,
                        RichText::new("📄 PDF Yükle").color(Color32::WHITE),
                    )
                    .clicked()
                {
                    self.aktif_sekme = Sekme::PdfYukle;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.hata_mesaji.is_empty() {
                ui.colored_label(Color32::RED, &self.hata_mesaji);
                if ui.button("✕").clicked() {
                    self.hata_mesaji.clear();
                }
            }
            if !self.basarili_mesaj.is_empty() {
                ui.colored_label(Color32::GREEN, &self.basarili_mesaj);
            }

            match self.aktif_sekme {
                Sekme::MetrajTablosu => self.render_metraj_tablosu(ui),
                Sekme::PdfYukle => self.render_pdf_yukle(ui),
            }
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let dosya_adi = self
                .mevcut_dosya_yolu
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "Kaydedilmemis".to_string());

            ui.label(format!(
                "📁 {} {}| 📊 {} poz | 📋 {} kalem | 💰 {:.2} TL",
                dosya_adi,
                if self.degisiklik_var { "● " } else { "" },
                self.poz_sayisi,
                self.metraj_kalemleri.len(),
                self.toplam_tutar()
            ));
        });
    }
}

impl MetrajApp {
    fn render_metraj_tablosu(&mut self, ui: &mut Ui) {
        egui::SidePanel::left("sol_panel")
            .resizable(true)
            .default_width(400.0)
            .min_width(300.0)
            .show_inside(ui, |ui| {
                self.render_arama_paneli(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_metraj_listesi(ui);
        });
    }

    fn render_arama_paneli(&mut self, ui: &mut Ui) {
        ui.heading("🔍 Poz Arama");

        ui.horizontal(|ui| {
            ui.label("Poz No:");
            let response = ui.add_sized(
                Vec2::new(200.0, 24.0),
                TextEdit::singleline(&mut self.poz_arama_metni).hint_text("örn: 15.100"),
            );
            if response.changed() {
                self.poz_no_ara();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Açıklama:");
            let response = ui.add_sized(
                Vec2::new(200.0, 24.0),
                TextEdit::singleline(&mut self.aciklama_arama_metni).hint_text("örn: beton, tugla..."),
            );
            if response.changed() && !self.aciklama_arama_metni.is_empty() {
                self.aciklama_ara();
            }
        });

        if !self.kategoriler.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Kategori:");
                egui::ComboBox::from_id_salt("kategori_combo")
                    .selected_text(&self.secili_kategori)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(false, "TÜMÜ").clicked() {
                            self.secili_kategori = "TÜMÜ".to_string();
                            self.kategori_pozlar.clear();
                        }
                        for kat in &self.kategoriler.clone() {
                            if ui.selectable_label(false, kat).clicked() {
                                self.secili_kategori = kat.clone();
                                self.kategori_filtrele();
                            }
                        }
                    });
            });
        }

        ui.separator();
        ui.label(
            RichText::new("Not: Fiyati olmayan pozlar (formül iceren) metraja eklenemez.")
                .color(Color32::DARK_GRAY)
                .size(11.0),
        );
        ui.separator();

        let poz_listesi = if !self.kategori_pozlar.is_empty() {
            &self.kategori_pozlar
        } else {
            &self.arama_sonuclari
        };

        if !poz_listesi.is_empty() {
            ui.label(format!("{} sonuç bulundu", poz_listesi.len()));
        } else if !self.poz_arama_metni.is_empty() || !self.aciklama_arama_metni.is_empty() {
            ui.label("Sonuç bulunamadi.");
        } else {
            ui.label(RichText::new("👆 Poz numarasi veya açiklama ile arama yapin").color(Color32::GRAY));
        }

        ScrollArea::vertical()
            .max_height(ui.available_height() - 50.0)
            .show(ui, |ui| {
                for poz in poz_listesi.iter() {
                    let secili = self
                        .secili_poz
                        .as_ref()
                        .map(|s| s.poz_no == poz.poz_no)
                        .unwrap_or(false);

                    let fiyat_metin = match poz.fiyat {
                        Some(f) => format!("{:.2} TL", f),
                        None => "--- (formül)".to_string(),
                    };

                    let etiket = format!("{} | {} | {}", poz.poz_no, poz.birim, fiyat_metin);

                    let cevap = ui.selectable_label(secili, RichText::new(&etiket).size(12.0));
                    if cevap.clicked() {
                        self.secili_poz = Some(poz.clone());
                        self.yeni_poz_no = poz.poz_no.clone();
                    }
                    cevap.on_hover_text(&poz.tanim);
                }
            });

        ui.separator();

        if let Some(ref poz) = self.secili_poz {
            ui.heading("📌 Seçili Poz");
            ui.colored_label(Color32::DARK_BLUE, format!("Poz No: {}", poz.poz_no));
            ui.label(format!("Açiklama: {}", poz.tanim));
            ui.label(format!("Birim: {}", poz.birim));
            match poz.fiyat {
                Some(f) => {
                    ui.colored_label(Color32::GREEN, format!("Birim Fiyat: {:.2} TL", f));
                }
                None => {
                    ui.colored_label(Color32::RED, "Bu poz formül icermektedir, metraja eklenemez.");
                }
            }
            ui.label(format!("Kategori: {}", poz.kategori));
        }
    }

    fn render_metraj_listesi(&mut self, ui: &mut Ui) {
        ui.heading("📋 Metraj Tablosu");

        ui.horizontal(|ui| {
            ui.label("Metraj Adi:");
            let resp = ui.add(
                TextEdit::singleline(&mut self.metraj_adi)
                    .hint_text("Metraj adi girin")
                    .desired_width(250.0),
            );
            if resp.changed() {
                self.degisiklik_var = true;
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Poz No:");
            let response = ui.add(
                TextEdit::singleline(&mut self.yeni_poz_no)
                    .hint_text("15.100.1001")
                    .desired_width(140.0),
            );
            if response.changed() {
                self.poz_sorgula();
            }

            ui.label("Miktar:");
            ui.add(
                TextEdit::singleline(&mut self.yeni_miktar)
                    .hint_text("0.00")
                    .desired_width(80.0),
            );

            if ui
                .button(RichText::new("➕ Kalem Ekle").color(Color32::WHITE))
                .highlight()
                .clicked()
            {
                self.kalem_ekle();
            }
        });

        if let Some(ref poz) = self.secili_poz {
            if let Some(fiyat) = poz.fiyat {
                let tahmini_tutar = match self.yeni_miktar.parse::<f64>() {
                    Ok(m) => m * fiyat,
                    Err(_) => 0.0,
                };
                ui.label(format!(
                    "{} | {} | {:.2} TL x {} = {:.2} TL",
                    poz.tanim, poz.birim, fiyat, self.yeni_miktar, tahmini_tutar
                ));
            }
        }

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("📂 Aç (.mrj)").clicked() {
                self.metraj_yukle_diyalog();
            }
            let kaydet_label = if self.mevcut_dosya_yolu.is_some() {
                "💾 Kaydet (Ctrl+S)"
            } else {
                "💾 Farklı Kaydet (.mrj)"
            };
            if ui.button(kaydet_label).clicked() {
                self.metraj_kaydet();
            }
            if self.degisiklik_var {
                ui.colored_label(Color32::YELLOW, "● Degisiklik var");
            }
            if ui.button("📊 Excel").clicked() {
                self.metraj_excel_diyalog();
            }
            if ui.button("🗑 Temizle").clicked() {
                self.metraj_kalemleri.clear();
                self.degisiklik_var = true;
                self.basarili_mesaj = "Metraj temizlendi.".to_string();
            }
        });

        ui.separator();

        ScrollArea::vertical()
            .max_height(ui.available_height() - 80.0)
            .show(ui, |ui| {
                self.render_metraj_kalem_tablosu(ui);
            });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("GENEL TOPLAM: {:.2} TL", self.toplam_tutar()))
                    .size(16.0)
                    .strong()
                    .color(Color32::GREEN),
            );
        });
    }

    fn render_metraj_kalem_tablosu(&mut self, ui: &mut Ui) {
        if self.metraj_kalemleri.is_empty() {
            ui.label(
                RichText::new("Henüz metraj kalemi eklenmedi. Soldaki panelden poz arayip ekleyin.")
                    .color(Color32::GRAY)
                    .size(13.0),
            );
            return;
        }

        egui::Grid::new("metraj_grid")
            .num_columns(8)
            .min_col_width(60.0)
            .striped(true)
            .show(ui, |ui: &mut egui::Ui| {
                ui.label(RichText::new("#").strong().size(12.0));
                ui.label(RichText::new("Poz No").strong().size(12.0));
                ui.label(RichText::new("Açiklama").strong().size(12.0));
                ui.label(RichText::new("Birim").strong().size(12.0));
                ui.label(RichText::new("B.Fiyat").strong().size(12.0));
                ui.label(RichText::new("Miktar").strong().size(12.0));
                ui.label(RichText::new("Tutar").strong().size(12.0));
                ui.label(RichText::new("").strong().size(12.0));
                ui.end_row();

                let mut silinecek: Option<usize> = None;
                let mut degisecek_miktar: Option<(usize, f64)> = None;

                for (idx, kalem) in self.metraj_kalemleri.iter_mut().enumerate() {
                    ui.label(format!("{}", idx + 1));
                    ui.label(RichText::new(&kalem.poz_no).size(11.0).monospace());

                    let kisa_tanim = if kalem.tanim.len() > 40 {
                        format!("{}...", &kalem.tanim[..37])
                    } else {
                        kalem.tanim.clone()
                    };
                    ui.label(RichText::new(kisa_tanim).size(11.0));
                    ui.label(&kalem.birim);
                    ui.label(format!("{:.2}", kalem.birim_fiyat));

                    let mut miktar_str = format!("{:.2}", kalem.miktar);
                    let resp = ui.add(TextEdit::singleline(&mut miktar_str).desired_width(70.0));
                    if resp.changed() {
                        if let Ok(yeni) = miktar_str.parse::<f64>() {
                            degisecek_miktar = Some((idx, yeni));
                        }
                    }

                    ui.label(
                        RichText::new(format!("{:.2}", kalem.tutar))
                            .size(11.0)
                            .strong()
                            .color(Color32::GREEN),
                    );

                    if ui
                        .button(RichText::new("✕").color(Color32::RED).size(11.0))
                        .clicked()
                    {
                        silinecek = Some(idx);
                    }

                    ui.end_row();
                }

                if let Some(idx) = silinecek {
                    self.metraj_kalemleri.remove(idx);
                    self.degisiklik_var = true;
                }
                if let Some((idx, yeni_miktar)) = degisecek_miktar {
                    if idx < self.metraj_kalemleri.len() {
                        self.metraj_kalemleri[idx].miktar = yeni_miktar;
                        self.metraj_kalemleri[idx].tutar_guncelle();
                        self.degisiklik_var = true;
                    }
                }
            });
    }

    fn render_pdf_yukle(&mut self, ui: &mut Ui) {
        ui.heading("📄 PDF Birim Fiyat Listesi Yükle");
        ui.separator();
        ui.label("Bu ekrandan Cevre ve Sehircilik Bakanligi birim fiyat listesini PDF formatinda yükleyebilirsiniz.");
        ui.label("PDF yüklendikten sonra Metraj Tablosu sekmesinden pozlari arayabilirsiniz.");
        ui.add_space(10.0);

        if self.pdf_yukleniyor {
            ui.spinner();
            ui.label("PDF isleniyor, lütfen bekleyin...");
        } else if ui
            .button(RichText::new("📂 PDF Dosyasi Seç ve Yükle").size(14.0))
            .clicked()
        {
            self.pdf_sec_ve_yukle();
        }

        if !self.pdf_durumu.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new(&self.pdf_durumu).color(Color32::GREEN));
        }

        ui.add_space(10.0);
        ui.label(format!("Veritabaninda su anda {} poz bulunuyor.", self.poz_sayisi));
        if self.poz_sayisi > 0 {
            ui.label(
                RichText::new("✅ Veritabani hazir. Metraj Tablosu sekmesine geçerek calismaya baslayabilirsiniz.")
                    .color(Color32::GREEN),
            );
        }

        ui.separator();
        ui.label("Hizli yükleme:");

        let varsayilan_pdf = PathBuf::from("..\\20206-05-BF.pdf");
        let alt_pdf = PathBuf::from("20206-05-BF.pdf");

        if varsayilan_pdf.exists() {
            if ui
                .button(format!(
                    "📄 Hizli Yükle: {}",
                    varsayilan_pdf.file_name().unwrap().to_string_lossy()
                ))
                .clicked()
            {
                self.pdf_yukle(varsayilan_pdf.clone());
            }
        } else if alt_pdf.exists() {
            if ui.button("📄 Hizli Yükle: 20206-05-BF.pdf").clicked() {
                self.pdf_yukle(alt_pdf);
            }
        } else {
            ui.label("Varsayilan PDF bulunamadi. Lütfen dosya seçin.");
        }
    }

    // ==================== YARDIMCI METODLAR ====================

    fn toplam_tutar(&self) -> f64 {
        self.metraj_kalemleri.iter().map(|k| k.tutar).sum()
    }

    fn poz_no_ara(&mut self) {
        if self.poz_arama_metni.is_empty() {
            self.arama_sonuclari.clear();
            return;
        }
        if let Some(ref db) = self.db {
            match db.poz_no_ara(&self.poz_arama_metni) {
                Ok(sonuc) => self.arama_sonuclari = sonuc,
                Err(e) => self.hata_mesaji = format!("Arama hatasi: {}", e),
            }
        }
    }

    fn aciklama_ara(&mut self) {
        if let Some(ref db) = self.db {
            match db.tam_metin_ara(&self.aciklama_arama_metni) {
                Ok(sonuc) => self.arama_sonuclari = sonuc,
                Err(e) => self.hata_mesaji = format!("Arama hatasi: {}", e),
            }
        }
    }

    fn poz_sorgula(&mut self) {
        if self.yeni_poz_no.is_empty() {
            self.secili_poz = None;
            return;
        }
        if let Some(ref db) = self.db {
            match db.poz_getir(&self.yeni_poz_no) {
                Ok(Some(poz)) => self.secili_poz = Some(poz),
                Ok(None) => {
                    match db.poz_no_ara(&self.yeni_poz_no) {
                        Ok(sonuc) => {
                            if sonuc.len() == 1 {
                                self.secili_poz = Some(sonuc[0].clone());
                                self.yeni_poz_no = sonuc[0].poz_no.clone();
                            } else {
                                self.arama_sonuclari = sonuc;
                                self.secili_poz = None;
                            }
                        }
                        Err(_) => self.secili_poz = None,
                    }
                }
                Err(e) => self.hata_mesaji = format!("Sorgu hatasi: {}", e),
            }
        }
    }

    fn kalem_ekle(&mut self) {
        if let Some(ref poz) = self.secili_poz {
            let miktar = self
                .yeni_miktar
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0);

            let kalem = MetrajKalemi::yeni(poz, miktar);
            let kalem_tutar = kalem.tutar;
            let kalem_bf = kalem.birim_fiyat;
            self.metraj_kalemleri.push(kalem);
            self.degisiklik_var = true;

            if miktar == 0.0 {
                self.basarili_mesaj = format!(
                    "{} eklendi. Miktari tablodaki hucreye tiklayip giriniz.",
                    poz.poz_no
                );
            } else {
                self.basarili_mesaj = format!(
                    "{} eklendi ({} {} x {:.2} TL = {:.2} TL).",
                    poz.poz_no, miktar, poz.birim, kalem_bf, kalem_tutar
                );
            }
            self.yeni_miktar.clear();
            self.hata_mesaji.clear();
        } else {
            self.hata_mesaji = "Lutfen once bir poz secin.".to_string();
        }
    }

    fn kategorileri_yukle(&mut self) {
        if let Some(ref db) = self.db {
            match db.kategoriler() {
                Ok(katlar) => self.kategoriler = katlar,
                Err(e) => self.hata_mesaji = format!("Kategori hatasi: {}", e),
            }
        }
    }

    fn kategori_filtrele(&mut self) {
        if let Some(ref db) = self.db {
            match db.tum_pozlar() {
                Ok(tumu) => {
                    self.kategori_pozlar = tumu
                        .into_iter()
                        .filter(|p| p.kategori == self.secili_kategori)
                        .collect();
                }
                Err(e) => self.hata_mesaji = format!("Liste hatasi: {}", e),
            }
        }
    }

    // ==================== DOSYA ISLEMLERI ====================

    fn pdf_sec_ve_yukle(&mut self) {
        let dosya = rfd::FileDialog::new()
            .add_filter("PDF Dosyalari", &["pdf"])
            .pick_file();

        if let Some(yol) = dosya {
            self.pdf_yukle(yol);
        }
    }

    fn pdf_yukle(&mut self, pdf_yolu: PathBuf) {
        self.pdf_yukleniyor = true;
        self.pdf_durumu = format!("PDF okunuyor: {}", pdf_yolu.display());

        match pdf_metin_cikar(&pdf_yolu) {
            Ok(metin) => {
                self.pdf_durumu = format!("{} satir metin cikarildi.", metin.lines().count());
                let pozlar = pozlari_ayristir(&metin);
                self.pdf_durumu = format!("{} poz ayrıştirildi.", pozlar.len());

                if let Some(ref db) = self.db {
                    match db.pozlari_yukle(&pozlar) {
                        Ok(sayi) => {
                            self.poz_sayisi = sayi as u32;
                            self.basarili_mesaj = format!("✅ Basariyla {} poz yüklendi!", sayi);
                            self.pdf_durumu = format!(
                                "✅ PDF basariyla islendi. {} poz veritabanina kaydedildi.",
                                sayi
                            );
                            self.kategorileri_yukle();
                        }
                        Err(e) => {
                            self.hata_mesaji = format!("Veritabani hatasi: {}", e);
                            self.pdf_durumu = format!("❌ Hata: {}", e);
                        }
                    }
                } else {
                    self.hata_mesaji = "Veritabani acik degil!".to_string();
                }
            }
            Err(e) => {
                self.hata_mesaji = format!("PDF hatasi: {}", e);
                self.pdf_durumu = format!("❌ Hata: {}", e);
            }
        }

        self.pdf_yukleniyor = false;
    }

    fn metraj_kaydet(&mut self) {
        let metraj = KayitliMetraj {
            ad: self.metraj_adi.clone(),
            kalemler: self.metraj_kalemleri.clone(),
            tarih: krono_tarih(),
        };

        if let Some(ref yol) = self.mevcut_dosya_yolu {
            match metraj_json_kaydet(&metraj, yol) {
                Ok(()) => {
                    self.degisiklik_var = false;
                    self.basarili_mesaj = format!("Kaydedildi: {}", yol.display());
                }
                Err(e) => self.hata_mesaji = format!("Kaydetme hatasi: {}", e),
            }
        } else {
            if let Some(dosya) = rfd::FileDialog::new()
                .add_filter("Metrajmatik Projesi", &["mrj"])
                .set_file_name(&format!("{}.mrj", self.metraj_adi))
                .save_file()
            {
                match metraj_json_kaydet(&metraj, &dosya) {
                    Ok(()) => {
                        self.mevcut_dosya_yolu = Some(dosya.clone());
                        self.degisiklik_var = false;
                        self.basarili_mesaj = format!("Kaydedildi: {}", dosya.display());
                    }
                    Err(e) => self.hata_mesaji = format!("Kaydetme hatasi: {}", e),
                }
            }
        }
    }

    fn metraj_yukle_diyalog(&mut self) {
        if let Some(dosya) = rfd::FileDialog::new()
            .add_filter("Metrajmatik Projesi", &["mrj", "json"])
            .pick_file()
        {
            match metraj_json_yukle(&dosya) {
                Ok(metraj) => {
                    self.metraj_kalemleri = metraj.kalemler;
                    self.metraj_adi = metraj.ad;
                    self.mevcut_dosya_yolu = Some(dosya.clone());
                    self.degisiklik_var = false;
                    self.basarili_mesaj = format!("Acildi: {}", dosya.display());
                }
                Err(e) => self.hata_mesaji = format!("Yukleme hatasi: {}", e),
            }
        }
    }

    fn metraj_excel_diyalog(&mut self) {
        let metraj = KayitliMetraj {
            ad: self.metraj_adi.clone(),
            kalemler: self.metraj_kalemleri.clone(),
            tarih: krono_tarih(),
        };

        if let Some(dosya) = rfd::FileDialog::new()
            .add_filter("Excel", &["xlsx"])
            .set_file_name(&format!("{}.xlsx", self.metraj_adi))
            .save_file()
        {
            match metraj_excel_aktar(&metraj, &dosya) {
                Ok(()) => {
                    self.basarili_mesaj = format!("Excel aktarildi: {}", dosya.display());
                }
                Err(e) => self.hata_mesaji = format!("Excel hatasi: {}", e),
            }
        }
    }
}

fn krono_tarih() -> String {
    let simdi = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = simdi.as_secs();
    let days_since_epoch = secs / 86400;
    let years = 1970 + days_since_epoch / 365;
    let remaining = days_since_epoch % 365;
    let months = remaining / 30;
    let days = remaining % 30;
    format!("{:04}-{:02}-{:02}", years, months + 1, days + 1)
}