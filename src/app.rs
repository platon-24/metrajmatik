use eframe::egui;
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui, Vec2};
use std::path::PathBuf;

use crate::database::Veritabani;
use crate::export::{metraj_excel_aktar, metraj_json_kaydet, metraj_json_yukle};
use crate::models::{IsGrubu, KayitliMetraj, Kitap, MetrajKalemi, MiktarDetay, Poz};
use crate::pdf_parser::{pdf_metin_cikar, pozlari_ayristir};
use crate::tema;

#[derive(Debug, Clone, PartialEq)]
enum Sekme { MetrajTablosu, Icmal, Pozlar, KitapYoneticisi, PdfYukle }

/// Miktar popup'ında bir detay satırının düzenlenebilir (metin) hali.
#[derive(Default, Clone)]
struct PopupDetaySatiri {
    aciklama: String,
    adet: String,
    en: String,
    boy: String,
    yukseklik: String,
}

/// Geri al/yinele için projenin düzenlenebilir durumunun anlık görüntüsü.
#[derive(Clone)]
struct Anlik {
    is_gruplari: Vec<IsGrubu>,
    metraj_kalemleri: Vec<MetrajKalemi>,
    secili_grup_id: Option<String>,
    metraj_adi: String,
}

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
    akilli_arama_metni: String,
    arama_sonuclari: Vec<Poz>,
    secili_poz: Option<Poz>,
    aciklama_arama_metni: String,
    yeni_poz_no: String,
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
    poz_form_acik: bool,
    poz_form_duzenleme: bool,
    poz_form_eski_poz_no: String,
    poz_form_poz_no: String,
    poz_form_tanim: String,
    poz_form_birim: String,
    poz_form_fiyat: String,
    poz_form_kategori: String,
    silinecek_poz: Option<Poz>,
    // Miktar detay popup
    miktar_popup_acik: bool,
    popup_kalem_indeks: Option<usize>,
    popup_detaylar: Vec<PopupDetaySatiri>,
    popup_yeni: PopupDetaySatiri,

    // Hiyerarşik İş Grupları alanları
    is_gruplari: Vec<crate::models::IsGrubu>,
    secili_grup_id: Option<String>,
    yeni_grup_adi: String,

    // İcmal / yaklaşık maliyet oranları
    genel_gider_kar_orani: f64,
    kdv_orani: f64,

    // Geri al / yinele
    geri_al_yigini: Vec<Anlik>,
    yinele_yigini: Vec<Anlik>,

    // Otomatik kayıt
    son_autosave: f64,
    autosave_yolu: PathBuf,
    kurtarma_mevcut: bool,
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
        // Yeni/boş proje için veritabanındaki varsayılan iş grupları şablonunu yükle
        let baslangic_gruplari = db.as_ref()
            .and_then(|vt| vt.varsayilan_gruplari_getir().ok())
            .unwrap_or_default();
        let baslangic_secili = ilk_yaprak_grup_id(&baslangic_gruplari);
        let autosave_yolu = PathBuf::from("metrajmatik_autosave.mrj");
        let kurtarma_mevcut = autosave_yolu.exists();
        Self {
            db, poz_sayisi, kitaplar, secili_kitap: None,
            metraj_kalemleri: vec![], metraj_adi: "Isimsiz Metraj".into(),
            mevcut_dosya_yolu: None, degisiklik_var: false,
            poz_arama_metni: String::new(), akilli_arama_metni: String::new(), arama_sonuclari: vec![], secili_poz: None,
            aciklama_arama_metni: String::new(), yeni_poz_no: String::new(),
            yeni_kitap_adi: String::new(), yeni_kitap_yil: 2026, yeni_kitap_ay: 5,
            duzenlenen_kitap: None, duzenleme_adi: String::new(), duzenleme_yil: 2026, duzenleme_ay: 1,
            fiyat_guncelle_hedef: None,
            cift_tiklama_ekle: false,
            pdf_durumu: String::new(), pdf_yukleniyor: false,
            aktif_sekme: Sekme::MetrajTablosu,
            hata_mesaji: String::new(), basarili_mesaj: String::new(),
            kategoriler: vec![], secili_kategori: "TÜMÜ".into(), kategori_pozlar: vec![],
            pozlar_arama_metni: String::new(), pozlar_tablosu: vec![], pozlar_yuklu_kitap_id: None,
            poz_form_acik: false,
            poz_form_duzenleme: false,
            poz_form_eski_poz_no: String::new(),
            poz_form_poz_no: String::new(),
            poz_form_tanim: String::new(),
            poz_form_birim: String::new(),
            poz_form_fiyat: String::new(),
            poz_form_kategori: String::new(),
            silinecek_poz: None,
            miktar_popup_acik: false,
            popup_kalem_indeks: None,
            popup_detaylar: vec![],
            popup_yeni: PopupDetaySatiri::default(),

            // Hiyerarşik İş Grupları
            is_gruplari: baslangic_gruplari,
            secili_grup_id: baslangic_secili,
            yeni_grup_adi: String::new(),

            // İcmal oranları (varsayılan)
            genel_gider_kar_orani: 25.0,
            kdv_orani: 20.0,

            // Geri al / yinele
            geri_al_yigini: vec![],
            yinele_yigini: vec![],

            // Otomatik kayıt
            son_autosave: 0.0,
            autosave_yolu,
            kurtarma_mevcut,
        }
    }
}

impl eframe::App for MetrajApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) { self.metraj_kaydet(); }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) { self.metraj_yukle_diyalog(); }
        // Geri al / yinele (Ctrl+Z, Ctrl+Y veya Ctrl+Shift+Z)
        if ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z)) { self.geri_al(); }
        if ctx.input(|i| i.modifiers.ctrl && (i.key_pressed(egui::Key::Y) || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))) { self.yinele(); }
        // Otomatik kayıt kontrolü
        self.autosave_kontrol(ctx);

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
                    ui.label(RichText::new("⚠ Yıl/Ay değişirse tüm pozlardaki yıl/ay da güncellenir.").color(tema::UYARI).size(12.0));
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        if tema::basari_buton(ui, "✓ Kaydet").clicked() {
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

        self.render_poz_form_popup(ctx);
        self.render_poz_sil_onay_popup(ctx);

        // Miktar detay popup'ı
        self.render_miktar_popup(ctx);

        egui::TopBottomPanel::top("menu_bar")
            .frame(egui::Frame::default().fill(tema::ARKA_PLAN).inner_margin(egui::Margin::symmetric(14, 9)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Marka
                    ui.label(RichText::new("🏗").size(22.0));
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.add_space(1.0);
                        ui.label(RichText::new("METRAJMATIK").color(tema::METIN).size(18.0).strong());
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new("Yaklaşık Maliyet").color(tema::METIN_SOLUK).size(12.0));

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Sekme "pill"leri
                    let sekmeler = [Sekme::MetrajTablosu, Sekme::Icmal, Sekme::Pozlar, Sekme::KitapYoneticisi, Sekme::PdfYukle];
                    let isimler = ["📋 Metraj", "📊 İcmal", "🔎 Pozlar", "📚 Kitaplar", "📄 PDF Yükle"];
                    for i in 0..sekmeler.len() {
                        let s = &sekmeler[i];
                        let aktif = self.aktif_sekme == *s;
                        let yazi = if aktif { Color32::WHITE } else { tema::METIN_IKINCIL };
                        let buton = egui::Button::new(RichText::new(isimler[i]).color(yazi).size(14.0))
                            .fill(if aktif { tema::VURGU } else { Color32::TRANSPARENT })
                            .stroke(if aktif { egui::Stroke::NONE } else { egui::Stroke::new(1.0, tema::KENAR) })
                            .corner_radius(egui::CornerRadius::same(tema::KOSE_KUCUK));
                        if ui.add(buton).clicked() {
                            self.aktif_sekme = s.clone();
                            if *s == Sekme::MetrajTablosu || *s == Sekme::Pozlar || *s == Sekme::KitapYoneticisi { self.kitaplari_yenile(); }
                            if *s == Sekme::MetrajTablosu { self.kategorileri_yukle(); }
                            if *s == Sekme::Pozlar { self.pozlar_tablosu_yenile(); }
                        }
                    }

                    // Sağ tarafta dosya adı
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(ref yol) = self.mevcut_dosya_yolu {
                            let dosya = yol.file_name().unwrap().to_string_lossy().to_string();
                            let isaret = if self.degisiklik_var { "●  " } else { "" };
                            ui.label(RichText::new(format!("{}{}", isaret, dosya)).color(if self.degisiklik_var { tema::UYARI } else { tema::METIN_SOLUK }).size(12.5));
                            ui.label(RichText::new("📄").size(13.0));
                        } else {
                            ui.label(RichText::new("Kaydedilmemiş proje").color(tema::METIN_SOLUK).size(12.5));
                        }
                    });
                });
            });

        // ÖNEMLİ: Alt durum çubuğu CentralPanel'den ÖNCE eklenmeli; aksi halde merkez
        // içerik pencerenin en altına kadar uzar ve durum çubuğu içeriğin üzerine biner.
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::default().fill(tema::ARKA_PLAN).inner_margin(egui::Margin::symmetric(12, 5)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let durum = if self.mevcut_dosya_yolu.is_some() {
                        if self.degisiklik_var { ("● Kaydedilmedi", tema::UYARI) } else { ("✓ Kayıtlı", tema::BASARI) }
                    } else { ("○ Yeni proje", tema::METIN_SOLUK) };
                    tema::rozet(ui, durum.0, durum.1);
                    if let Some(ref k) = self.secili_kitap {
                        tema::rozet(ui, &format!("📚 {}", metni_kisalt(&k.ad, 30)), tema::METIN_IKINCIL);
                    }
                    tema::rozet(ui, &format!("🗂 {} poz", self.poz_sayisi), tema::METIN_IKINCIL);
                    tema::rozet(ui, &format!("📋 {} kalem", self.metraj_kalemleri.len()), tema::METIN_IKINCIL);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} TL", para_formatla(self.toplam_tutar()))).color(tema::BASARI).strong().size(14.0));
                        ui.label(RichText::new("Genel Toplam:").color(tema::METIN_SOLUK).size(12.0));
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(tema::ARKA_PLAN).inner_margin(egui::Margin::same(12)))
            .show(ctx, |ui| {
            let mut hata_kapat = false;
            if !self.hata_mesaji.is_empty() {
                ui.horizontal(|ui| {
                    egui::Frame::default()
                        .fill(tema::TEHLIKE_KOYU)
                        .stroke(egui::Stroke::new(1.0, tema::TEHLIKE))
                        .corner_radius(egui::CornerRadius::same(tema::KOSE_KUCUK))
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.label(RichText::new(format!("⚠  {}", self.hata_mesaji)).color(tema::TEHLIKE));
                            if ui.add(egui::Button::new(RichText::new("✕").color(tema::TEHLIKE)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::NONE)).clicked() { hata_kapat = true; }
                        });
                });
                ui.add_space(6.0);
            }
            if hata_kapat { self.hata_mesaji.clear(); }
            if !self.basarili_mesaj.is_empty() {
                tema::bildirim_seridi(ui, &format!("✓  {}", self.basarili_mesaj), tema::BASARI_KOYU, tema::BASARI, tema::BASARI);
                ui.add_space(6.0);
            }
            // Kurtarma şeridi: otomatik kayıt dosyası varsa
            if self.kurtarma_mevcut {
                let mut kurtar = false;
                let mut yoksay = false;
                egui::Frame::default()
                    .fill(tema::UYARI_KOYU)
                    .stroke(egui::Stroke::new(1.0, tema::UYARI))
                    .corner_radius(egui::CornerRadius::same(tema::KOSE_KUCUK))
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⟲  Önceki oturumdan kurtarılabilir otomatik kayıt bulundu.").color(tema::UYARI));
                            if tema::birincil_buton(ui, "Kurtar").clicked() { kurtar = true; }
                            if ui.button("Yoksay").clicked() { yoksay = true; }
                        });
                    });
                ui.add_space(6.0);
                if kurtar {
                    let yol = self.autosave_yolu.clone();
                    self.metraj_dosyadan_yukle(&yol, false);
                    self.kurtarma_mevcut = false;
                }
                if yoksay {
                    let _ = std::fs::remove_file(&self.autosave_yolu);
                    self.kurtarma_mevcut = false;
                }
            }
            match self.aktif_sekme {
                Sekme::MetrajTablosu => self.render_metraj_tablosu(ui),
                Sekme::Icmal => self.render_icmal(ui),
                Sekme::Pozlar => self.render_pozlar_tablosu(ui),
                Sekme::KitapYoneticisi => self.render_kitap_yoneticisi(ui),
                Sekme::PdfYukle => self.render_pdf_yukle(ui),
            }
        });
    }
}

impl MetrajApp {
    // ==================== KITAP YONETICISI ====================
    fn render_kitap_yoneticisi(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "📚", "Kitap Yöneticisi");
        ui.add_space(6.0);

        // Yeni kitap ekleme
        tema::kart(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Kitap Adı").color(tema::METIN_IKINCIL).size(12.0));
            ui.add(TextEdit::singleline(&mut self.yeni_kitap_adi).hint_text("örn: Çevre ve Şehircilik").desired_width(220.0));
            ui.label(RichText::new("Yıl").color(tema::METIN_IKINCIL).size(12.0));
            egui::ComboBox::from_id_salt("yil_combo").selected_text(format!("{}", self.yeni_kitap_yil)).width(70.0).show_ui(ui, |ui| {
                for y in [2024u32, 2025, 2026, 2027, 2028] { if ui.selectable_label(self.yeni_kitap_yil == y, format!("{}", y)).clicked() { self.yeni_kitap_yil = y; } }
            });
            ui.label(RichText::new("Ay").color(tema::METIN_IKINCIL).size(12.0));
            egui::ComboBox::from_id_salt("ay_combo").selected_text(format!("{}", self.yeni_kitap_ay)).width(50.0).show_ui(ui, |ui| {
                for a in 1u32..=12 { if ui.selectable_label(self.yeni_kitap_ay == a, format!("{}", a)).clicked() { self.yeni_kitap_ay = a; } }
            });
            if tema::birincil_buton(ui, "＋ Ekle").clicked() {
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
        });

        ui.add_space(10.0);
        ui.label(RichText::new("Yüklü Kitaplar").strong().size(14.0).color(tema::METIN)); ui.add_space(6.0);
        if self.kitaplar.is_empty() { ui.label(RichText::new("Henüz kitap yok.").color(tema::METIN_SOLUK)); return; }

        let kitaplar_snapshot = self.kitaplar.clone();
        egui::Grid::new("kitap_grid").num_columns(8).min_col_width(50.0).spacing(egui::vec2(12.0, 8.0)).striped(true).show(ui, |ui: &mut egui::Ui| {
            let baslik = |ui: &mut egui::Ui, t: &str| { ui.label(RichText::new(t).strong().size(12.0).color(tema::METIN_IKINCIL)); };
            baslik(ui, "ID"); baslik(ui, "Kitap Adı");
            baslik(ui, "Yıl"); baslik(ui, "Ay");
            baslik(ui, "Poz"); baslik(ui, "Tarih");
            baslik(ui, "Düzenle"); baslik(ui, "Sil");
            ui.end_row();

            for kitap in &kitaplar_snapshot {
                let secili = self.secili_kitap.as_ref().map(|k| k.id == kitap.id).unwrap_or(false);
                ui.label(if secili { RichText::new(format!("{}", kitap.id)).color(tema::BASARI) } else { RichText::new(format!("{}", kitap.id)).color(tema::METIN_SOLUK) });
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
                if ui.add(egui::Button::new(RichText::new("🗑").color(tema::TEHLIKE)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::new(1.0, tema::KENAR))).clicked() {
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

        if let Some(ref k) = self.secili_kitap {
            ui.add_space(8.0);
            tema::bildirim_seridi(ui, &format!("✓  Aktif: {} ({}/{}, {} poz)", k.ad, k.ay, k.yil, k.poz_sayisi), tema::BASARI_KOYU, tema::BASARI, tema::BASARI);
        }
    }

    // ==================== METRAJ TABLOSU ====================
    fn render_metraj_tablosu(&mut self, ui: &mut Ui) {
        tema::kart(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Fiyat Kitabı").color(tema::METIN_IKINCIL).strong());
                let km = self.secili_kitap.as_ref().map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil)).unwrap_or_else(|| "TÜM KİTAPLAR".into());
                egui::ComboBox::from_id_salt("kitap_secici").selected_text(&km).width(360.0).show_ui(ui, |ui| {
                    if ui.selectable_label(self.secili_kitap.is_none(), "TÜM KİTAPLAR").clicked() { self.secili_kitap = None; self.kategorileri_yukle(); }
                    for k in self.kitaplar.clone() {
                        if ui.selectable_label(false, format!("{} ({}/{})", k.ad, k.ay, k.yil)).clicked() { self.secili_kitap = Some(k); self.kategorileri_yukle(); }
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Arama yapılacak fiyat kaynağı").color(tema::METIN_SOLUK).size(12.0));
                });
            });
        });
        ui.add_space(8.0);

        let panel_frame = egui::Frame::default().fill(tema::YUZEY).inner_margin(egui::Margin::same(10));
        egui::SidePanel::left("sol_panel").frame(panel_frame).resizable(true).default_width(400.0).min_width(300.0).show_inside(ui, |ui| { self.render_arama_paneli(ui); });
        egui::SidePanel::left("grup_panel").frame(panel_frame).resizable(true).default_width(270.0).min_width(210.0).show_inside(ui, |ui| { self.render_is_gruplari_paneli(ui); });
        egui::CentralPanel::default().frame(egui::Frame::default().fill(tema::ARKA_PLAN).inner_margin(egui::Margin { left: 12, right: 0, top: 0, bottom: 0 })).show_inside(ui, |ui| { self.render_metraj_listesi(ui); });
    }

    fn render_is_gruplari_paneli(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "🗂", "İş Grupları");
        ui.add_space(4.0);

        tema::kart(ui, |ui| {
            ui.add(TextEdit::singleline(&mut self.yeni_grup_adi).hint_text("Yeni grup adı…").desired_width(f32::INFINITY));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if tema::birincil_buton(ui, "＋ Ana Grup").clicked() {
                    let ad = self.yeni_grup_adi.trim().to_string();
                    if ad.is_empty() {
                        self.hata_mesaji = "Grup adı boş olamaz.".into();
                    } else {
                        self.anlik_goruntu_al();
                        self.is_gruplari.push(IsGrubu::yeni(&ad));
                        self.yeni_grup_adi.clear();
                        self.degisiklik_var = true;
                        self.hata_mesaji.clear();
                    }
                }
                if ui.button("＋ Alt Grup").clicked() {
                    let ad = self.yeni_grup_adi.trim().to_string();
                    match (self.secili_grup_id.clone(), ad.is_empty()) {
                        (None, _) => self.hata_mesaji = "Önce bir üst grup seçin.".into(),
                        (_, true) => self.hata_mesaji = "Grup adı boş olamaz.".into(),
                        (Some(ust_id), false) => {
                            self.anlik_goruntu_al();
                            if let Some(ust) = grup_bul_mut(&mut self.is_gruplari, &ust_id) {
                                ust.alt_gruplar.push(IsGrubu::yeni(&ad));
                            }
                            self.yeni_grup_adi.clear();
                            self.degisiklik_var = true;
                            self.hata_mesaji.clear();
                        }
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("✏ Adlandır").clicked() {
                    let ad = self.yeni_grup_adi.trim().to_string();
                    match (self.secili_grup_id.clone(), ad.is_empty()) {
                        (None, _) => self.hata_mesaji = "Önce bir grup seçin.".into(),
                        (_, true) => self.hata_mesaji = "Yeni ad boş olamaz.".into(),
                        (Some(id), false) => {
                            self.anlik_goruntu_al();
                            if let Some(g) = grup_bul_mut(&mut self.is_gruplari, &id) { g.ad = ad; }
                            self.yeni_grup_adi.clear();
                            self.degisiklik_var = true;
                            self.hata_mesaji.clear();
                        }
                    }
                }
                if tema::tehlike_buton(ui, "🗑 Sil").clicked() {
                    if let Some(id) = self.secili_grup_id.clone() {
                        self.anlik_goruntu_al();
                        grup_sil(&mut self.is_gruplari, &id);
                        self.secili_grup_id = None;
                        self.metraj_kalemleri.clear();
                        if let Some(yeni_id) = ilk_yaprak_grup_id(&self.is_gruplari) {
                            self.grup_sec(yeni_id);
                        }
                        self.degisiklik_var = true;
                    } else {
                        self.hata_mesaji = "Silinecek grubu seçin.".into();
                    }
                }
            });
        });
        ui.add_space(8.0);

        if self.is_gruplari.is_empty() {
            ui.label(RichText::new("Henüz iş grubu yok.\nYukarıdan ekleyin.").color(tema::METIN_SOLUK).size(12.0));
            return;
        }

        let secili = self.secili_grup_id.clone();
        let mut secilen: Option<String> = None;
        ScrollArea::vertical().show(ui, |ui| {
            is_grubu_agac_ciz(ui, &self.is_gruplari, secili.as_deref(), &self.metraj_kalemleri, &mut secilen);
        });
        if let Some(id) = secilen {
            self.grup_sec(id);
        }
    }

    fn render_arama_paneli(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "🔍", "Poz Arama");
        ui.add_space(4.0);

        tema::kart(ui, |ui| {
            ui.add(TextEdit::singleline(&mut self.akilli_arama_metni).hint_text("⚡ Hızlı ara: 15.180 veya plywood kalıp").desired_width(f32::INFINITY))
                .changed().then(|| self.akilli_ara());
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Poz No").color(tema::METIN_IKINCIL).size(12.0));
                if ui.add_sized(Vec2::new(110.0, 26.0), TextEdit::singleline(&mut self.poz_arama_metni).hint_text("15.100")).changed() { self.akilli_arama_metni.clear(); self.poz_no_ara(); }
                ui.label(RichText::new("Açıklama").color(tema::METIN_IKINCIL).size(12.0));
                if ui.add_sized(Vec2::new(ui.available_width(), 26.0), TextEdit::singleline(&mut self.aciklama_arama_metni).hint_text("beton")).changed() {
                    self.akilli_arama_metni.clear();
                    if self.aciklama_arama_metni.is_empty() { self.arama_sonuclari.clear(); } else { self.aciklama_ara(); }
                }
            });
            if !self.kategoriler.is_empty() {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Kategori").color(tema::METIN_IKINCIL).size(12.0));
                    egui::ComboBox::from_id_salt("kategori_combo").selected_text(&self.secili_kategori).width(ui.available_width()).show_ui(ui, |ui| {
                        if ui.selectable_label(false, "TÜMÜ").clicked() { self.secili_kategori = "TÜMÜ".into(); self.kategori_pozlar.clear(); }
                        for kat in &self.kategoriler.clone() { if ui.selectable_label(false, kat).clicked() { self.secili_kategori = kat.clone(); self.kategori_filtrele(); } }
                    });
                });
            }
        });
        ui.add_space(8.0);

        let pl = if !self.kategori_pozlar.is_empty() { &self.kategori_pozlar } else { &self.arama_sonuclari };
        let arama_var = !self.akilli_arama_metni.is_empty() || !self.poz_arama_metni.is_empty() || !self.aciklama_arama_metni.is_empty();
        if !pl.is_empty() { ui.label(RichText::new(format!("{} sonuç", pl.len())).color(tema::METIN_SOLUK).size(12.0)); }
        else if arama_var { ui.label(RichText::new("Sonuç bulunamadı.").color(tema::METIN_SOLUK).size(12.0)); }
        else { ui.label(RichText::new("👆 Yukarıdan arama yapın").color(tema::METIN_SOLUK).size(12.0)); }
        ui.add_space(4.0);

        self.cift_tiklama_ekle = false;
        let secili_poz_var = self.secili_poz.is_some();
        let liste_yuksekligi = if secili_poz_var { (ui.available_height() - 160.0).max(120.0) } else { ui.available_height() - 8.0 };
        ScrollArea::vertical().max_height(liste_yuksekligi).auto_shrink([false, false]).show(ui, |ui| {
            for poz in pl.iter() {
                let secili = self.secili_poz.as_ref().map(|s| s.poz_no == poz.poz_no && s.kitap_id == poz.kitap_id).unwrap_or(false);
                let fm = match poz.fiyat { Some(f) => format!("{} TL", para_formatla(f)), None => "Formül".into() };
                let fiyat_rengi = if poz.fiyat.is_some() { tema::BASARI } else { tema::UYARI };

                let cerceve = egui::Frame::default()
                    .fill(if secili { tema::VURGU_SOLUK } else { tema::YUZEY_2 })
                    .stroke(egui::Stroke::new(1.0, if secili { tema::VURGU } else { tema::KENAR_YUMUSAK }))
                    .corner_radius(egui::CornerRadius::same(tema::KOSE_KUCUK))
                    .inner_margin(egui::Margin::symmetric(9, 7));
                let ic = cerceve.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&poz.poz_no).monospace().size(12.0).strong().color(if secili { Color32::WHITE } else { tema::METIN }));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(fm).size(11.5).strong().color(fiyat_rengi));
                            ui.label(RichText::new(&poz.birim).size(11.0).color(tema::METIN_SOLUK));
                        });
                    });
                    ui.label(RichText::new(&poz.tanim).size(11.5).color(if secili { tema::METIN } else { tema::METIN_IKINCIL }));
                });
                let response = ic.response.interact(egui::Sense::click());
                if response.clicked() {
                    self.secili_poz = Some(poz.clone());
                    self.yeni_poz_no = poz.poz_no.clone();
                }
                if response.double_clicked() {
                    self.secili_poz = Some(poz.clone());
                    self.yeni_poz_no = poz.poz_no.clone();
                    self.cift_tiklama_ekle = true;
                }
                response.on_hover_text(format!("{}/{} | {}\nÇift tıkla: metraja ekle", poz.ay, poz.yil, poz.tanim));
                ui.add_space(4.0);
            }
        });

        if self.cift_tiklama_ekle {
            self.kalem_ekle();
        }

        let mut secili_poz_ekle = false;
        if let Some(poz) = self.secili_poz.clone() {
            ui.add_space(6.0);
            tema::kart(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📌 Seçili Poz").color(tema::METIN_IKINCIL).size(12.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match poz.fiyat {
                            Some(f) => ui.label(RichText::new(format!("{} TL", para_formatla(f))).color(tema::BASARI).strong().size(14.0)),
                            None => ui.label(RichText::new("Formül").color(tema::UYARI).strong()),
                        };
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&poz.poz_no).monospace().strong().size(15.0).color(tema::METIN));
                    ui.label(RichText::new(format!("· {} · {} ({}/{})", poz.birim, poz.kitap_adi, poz.ay, poz.yil)).color(tema::METIN_SOLUK).size(11.5));
                });
                ui.label(RichText::new(&poz.tanim).size(12.0).color(tema::METIN_IKINCIL));
                ui.add_space(6.0);
                if tema::birincil_buton(ui, "＋ Metraja Ekle").clicked() {
                    secili_poz_ekle = true;
                }
            });
        }
        if secili_poz_ekle {
            self.kalem_ekle();
        }
    }

    fn render_metraj_listesi(&mut self, ui: &mut Ui) {
        let aktif_grup_adi = self.secili_grup_id.as_ref()
            .and_then(|id| grup_bul_ref(&self.is_gruplari, id))
            .map(|g| g.ad.clone());

        // Başlık satırı: başlık + aktif grup rozeti + dosya işlemleri
        ui.horizontal(|ui| {
            ui.label(RichText::new("📋").size(17.0));
            ui.label(RichText::new("Metraj Tablosu").size(16.0).strong().color(tema::METIN));
            match &aktif_grup_adi {
                Some(ad) => tema::rozet(ui, &format!("▸ {}", ad), tema::VURGU_HOVER),
                None if !self.is_gruplari.is_empty() => tema::rozet(ui, "▸ Grup seçili değil", tema::UYARI),
                None => {}
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑 Temizle").clicked() { self.anlik_goruntu_al(); self.metraj_kalemleri.clear(); self.aktif_grubu_senkronize(); self.degisiklik_var = true; self.basarili_mesaj = "Temizlendi.".into(); }
                if ui.button("📊 Excel").clicked() { self.metraj_excel_diyalog(); }
                let lbl = if self.mevcut_dosya_yolu.is_some() { "💾 Kaydet" } else { "💾 Kaydet" };
                if tema::basari_buton(ui, lbl).clicked() { self.metraj_kaydet(); }
                if ui.button("📂 Aç").clicked() { self.metraj_yukle_diyalog(); }
            });
        });
        ui.add_space(8.0);

        // Giriş kartı: metraj adı + hızlı poz ekleme + toplu fiyat
        tema::kart(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Metraj Adı").color(tema::METIN_IKINCIL).size(12.0));
                if ui.add(TextEdit::singleline(&mut self.metraj_adi).hint_text("Proje / metraj adı").desired_width(240.0)).changed() { self.degisiklik_var = true; }
                ui.add_space(12.0);
                ui.label(RichText::new("Poz No").color(tema::METIN_IKINCIL).size(12.0));
                let poz_no_response = ui.add(TextEdit::singleline(&mut self.yeni_poz_no).hint_text("15.100.1001").desired_width(140.0));
                if poz_no_response.changed() { self.poz_sorgula(); }
                if poz_no_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.poz_sorgula();
                    self.kalem_ekle();
                }
                if tema::birincil_buton(ui, "＋ Kalem Ekle").clicked() {
                    self.poz_sorgula();
                    self.kalem_ekle();
                }
            });
            // Fiyat güncelleme - hedef kitap seçerek tüm kalemleri yeni fiyatlarla güncelle
            if !self.metraj_kalemleri.is_empty() && self.kitaplar.len() > 1 {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔄 Toplu Fiyat Güncelle").color(tema::METIN_IKINCIL).size(12.0));
                    let hedef_metni = self.fiyat_guncelle_hedef.as_ref()
                        .map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil))
                        .unwrap_or_else(|| "Hedef kitap seçin".to_string());
                    egui::ComboBox::from_id_salt("fiyat_guncelle_combo")
                        .selected_text(&hedef_metni)
                        .width(280.0)
                        .show_ui(ui, |ui| {
                            for k in &self.kitaplar.clone() {
                                if ui.selectable_label(false, format!("{} ({}/{})", k.ad, k.ay, k.yil)).clicked() {
                                    self.fiyat_guncelle_hedef = Some(k.clone());
                                }
                            }
                        });
                    if ui.button("Güncelle").clicked() {
                        self.fiyatlari_guncelle();
                    }
                });
            }
        });
        ui.add_space(8.0);

        self.render_metraj_ozetleri(ui);
        ui.add_space(8.0);

        ScrollArea::vertical().max_height(ui.available_height() - 64.0).auto_shrink([false, false]).show(ui, |ui| { self.render_metraj_kalem_tablosu(ui); });

        // Alt toplam çubuğu
        ui.add_space(6.0);
        egui::Frame::default()
            .fill(tema::YUZEY_2)
            .stroke(egui::Stroke::new(1.0, tema::KENAR))
            .corner_radius(egui::CornerRadius::same(tema::KOSE))
            .inner_margin(egui::Margin::symmetric(14, 9))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if !self.is_gruplari.is_empty() && self.secili_grup_id.is_some() {
                        let alt_toplam: f64 = self.metraj_kalemleri.iter().map(|k| k.tutar).sum();
                        ui.label(RichText::new("Grup Alt Toplamı").color(tema::METIN_SOLUK).size(12.0));
                        ui.label(RichText::new(format!("{} TL", para_formatla(alt_toplam))).size(14.0).strong().color(tema::VURGU_HOVER));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} TL", para_formatla(self.toplam_tutar()))).size(19.0).strong().color(tema::BASARI));
                        ui.label(RichText::new("GENEL TOPLAM").color(tema::METIN_IKINCIL).size(13.0).strong());
                    });
                });
            });
    }

    fn render_metraj_kalem_tablosu(&mut self, ui: &mut Ui) {
        if self.metraj_kalemleri.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("📋").size(32.0));
                ui.add_space(6.0);
                ui.label(RichText::new("Bu grupta henüz kalem yok").color(tema::METIN_IKINCIL).size(14.0));
                ui.label(RichText::new("Soldan bir poz arayıp “Metraja Ekle” ile başlayın").color(tema::METIN_SOLUK).size(12.0));
            });
            return;
        }
        let mut popup_acilacak: Option<usize> = None;
        let baslik = |ui: &mut egui::Ui, t: &str| { ui.label(RichText::new(t).strong().size(12.0).color(tema::METIN_IKINCIL)); };
        egui::Grid::new("metraj_grid").num_columns(9).min_col_width(40.0).spacing(egui::vec2(10.0, 8.0)).striped(true).show(ui, |ui: &mut egui::Ui| {
            baslik(ui, "#"); baslik(ui, "Poz No");
            baslik(ui, "Açıklama"); baslik(ui, "Kitap");
            baslik(ui, "Birim"); baslik(ui, "B.Fiyat");
            baslik(ui, "Miktar"); baslik(ui, "Tutar"); baslik(ui, "");
            ui.end_row();

            let mut sil: Option<usize> = None;
            for (idx, kalem) in self.metraj_kalemleri.iter().enumerate() {
                ui.label(RichText::new(format!("{}", idx + 1)).color(tema::METIN_SOLUK).size(11.0));
                let poz_response = ui.label(RichText::new(&kalem.poz_no).size(11.5).monospace().color(tema::METIN));
                let kisa = metni_kisalt(&kalem.tanim, 46);
                let aciklama_response = ui.label(RichText::new(kisa).size(11.5).color(tema::METIN_IKINCIL)).on_hover_text(&kalem.tanim);
                let kitap_kisa = metni_kisalt(&kalem.kitap_adi, 18);
                ui.label(RichText::new(kitap_kisa).size(10.5).color(tema::METIN_SOLUK)).on_hover_text(&kalem.kitap_adi);
                ui.label(RichText::new(&kalem.birim).size(11.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new(para_formatla(kalem.birim_fiyat)).size(11.5).color(tema::METIN_IKINCIL));
                let miktar_renk = if kalem.miktar > 0.0 { tema::METIN } else { tema::UYARI };
                let miktar_metni = if kalem.detaylar.is_empty() { format!("{:.2}", kalem.miktar) } else { format!("📐 {:.2}", kalem.miktar) };
                let miktar_response = ui.label(RichText::new(miktar_metni).size(11.5).strong().color(miktar_renk))
                    .on_hover_text(if kalem.detaylar.is_empty() { "Ölçü detayı yok — düzenlemek için tıkla" } else { "Ölçü kırılımı var — düzenlemek için tıkla" });
                ui.label(RichText::new(para_formatla(kalem.tutar)).size(11.5).strong().color(tema::BASARI));
                if ui.add(egui::Button::new(RichText::new("✕").color(tema::TEHLIKE).size(11.0)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::NONE)).on_hover_text("Kalemi sil").clicked() { sil = Some(idx); }
                let satir_response = poz_response.union(aciklama_response).union(miktar_response);
                if satir_response.on_hover_text("Miktar detaylarını düzenle").clicked() {
                    popup_acilacak = Some(idx);
                }
                ui.end_row();
            }
            if let Some(idx) = sil { self.anlik_goruntu_al(); self.metraj_kalemleri.remove(idx); self.aktif_grubu_senkronize(); self.degisiklik_var = true; }
        });
        if let Some(idx) = popup_acilacak {
            self.popup_kalem_indeks = Some(idx);
            self.popup_detaylar = self.metraj_kalemleri[idx].detaylar.iter()
                .map(detay_to_satir)
                .collect();
            self.popup_yeni = PopupDetaySatiri::default();
            self.miktar_popup_acik = true;
        }
    }

    fn render_miktar_popup(&mut self, ctx: &egui::Context) {
        if !self.miktar_popup_acik { return; }
        let idx = match self.popup_kalem_indeks {
            Some(i) if i < self.metraj_kalemleri.len() => i,
            _ => { self.miktar_popup_acik = false; return; }
        };
        let kalem = &self.metraj_kalemleri[idx];
        let poz_no = kalem.poz_no.clone();
        let tanim = kalem.tanim.clone();
        let birim = kalem.birim.clone();
        let birim_fiyat = kalem.birim_fiyat;

        egui::Window::new("📐 Miktar Detayları")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&poz_no).monospace().strong().size(16.0));
                    ui.label(RichText::new(&tanim).size(14.0));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Birim: {}", birim)).color(tema::METIN_IKINCIL));
                    ui.label(RichText::new(format!("Birim Fiyat: {} TL", para_formatla(birim_fiyat))).color(tema::BASARI));
                });
                ui.separator();

                ui.label(RichText::new("Ölçü detayları  ·  boş bırakılan boyut 1 sayılır").color(tema::METIN_SOLUK).size(11.5));
                ui.add_space(3.0);
                let bsl = |ui: &mut egui::Ui, t: &str| { ui.label(RichText::new(t).strong().size(11.5).color(tema::METIN_IKINCIL)); };
                egui::Grid::new("popup_detay_grid").num_columns(8).spacing(egui::vec2(7.0, 6.0)).striped(true).show(ui, |ui| {
                    bsl(ui, "#"); bsl(ui, "Açıklama"); bsl(ui, "Adet"); bsl(ui, "En"); bsl(ui, "Boy"); bsl(ui, "Yük."); bsl(ui, "= Miktar"); bsl(ui, "");
                    ui.end_row();

                    let mut silinecek_satir: Option<usize> = None;
                    for (d_idx, satir) in self.popup_detaylar.iter_mut().enumerate() {
                        ui.label(RichText::new(format!("{}", d_idx + 1)).color(tema::METIN_SOLUK).size(11.0));
                        ui.add(TextEdit::singleline(&mut satir.aciklama).desired_width(170.0).hint_text("açıklama"));
                        ui.add(TextEdit::singleline(&mut satir.adet).desired_width(48.0));
                        ui.add(TextEdit::singleline(&mut satir.en).desired_width(48.0));
                        ui.add(TextEdit::singleline(&mut satir.boy).desired_width(48.0));
                        ui.add(TextEdit::singleline(&mut satir.yukseklik).desired_width(48.0));
                        let m = satir_miktar(satir).unwrap_or(0.0);
                        ui.label(RichText::new(format!("{:.3}", m)).size(11.5).strong().color(tema::BASARI));
                        if ui.add(egui::Button::new(RichText::new("🗑").color(tema::TEHLIKE).size(11.0)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::NONE)).clicked() {
                            silinecek_satir = Some(d_idx);
                        }
                        ui.end_row();
                    }
                    if let Some(s) = silinecek_satir { self.popup_detaylar.remove(s); }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Yeni satır").color(tema::METIN_IKINCIL).size(11.5));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.aciklama).hint_text("açıklama").desired_width(160.0));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.adet).hint_text("adet").desired_width(48.0));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.en).hint_text("en").desired_width(48.0));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.boy).hint_text("boy").desired_width(48.0));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.yukseklik).hint_text("yük.").desired_width(48.0));
                    let ekle = tema::birincil_buton(ui, "＋ Ekle").clicked();
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (ekle || enter) && satir_miktar(&self.popup_yeni).is_some() {
                        self.popup_detaylar.push(std::mem::take(&mut self.popup_yeni));
                    }
                });
                ui.separator();

                let toplam_miktar: f64 = self.popup_detaylar.iter().filter_map(satir_miktar).sum();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Toplam Miktar: {:.3} {}", toplam_miktar, birim)).size(14.0).strong().color(tema::BASARI));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("≈ {} TL", para_formatla(toplam_miktar * birim_fiyat))).size(13.0).color(tema::METIN_IKINCIL));
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if tema::basari_buton(ui, "✓ Tamam").clicked() {
                        let detaylar: Vec<MiktarDetay> = self.popup_detaylar.iter()
                            .filter_map(satir_to_detay)
                            .collect();
                        self.anlik_goruntu_al();
                        if let Some(kalem) = self.metraj_kalemleri.get_mut(idx) {
                            kalem.detaylar = detaylar;
                            kalem.detaylardan_miktar_hesapla();
                            self.degisiklik_var = true;
                        }
                        self.aktif_grubu_senkronize();
                        self.miktar_popup_acik = false;
                    }
                    if ui.button("❌ İptal").clicked() {
                        self.miktar_popup_acik = false;
                    }
                });
            });
    }

    fn render_metraj_ozetleri(&self, ui: &mut Ui) {
        let toplam_kalem = self.metraj_kalemleri.len();
        let fiyatsiz = self.metraj_kalemleri.iter().filter(|k| k.birim_fiyat <= 0.0).count();
        let secili_kitap_tutari = self.secili_kitap.as_ref().map(|kitap| {
            self.metraj_kalemleri.iter()
                .filter(|k| k.kitap_adi.starts_with(&kitap.ad))
                .map(|k| k.tutar)
                .sum::<f64>()
        }).unwrap_or(0.0);
        let mut kitap_sayisi: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for kalem in &self.metraj_kalemleri {
            kitap_sayisi.insert(&kalem.kitap_adi);
        }

        ui.horizontal_wrapped(|ui| {
            tema::rozet(ui, &format!("📋 {} kalem", toplam_kalem), tema::METIN_IKINCIL);
            tema::rozet(ui, &format!("📚 {} kitap", kitap_sayisi.len()), tema::METIN_IKINCIL);
            tema::rozet(ui, &format!("⚠ {} fiyatsız", fiyatsiz), if fiyatsiz > 0 { tema::UYARI } else { tema::METIN_SOLUK });
            if self.secili_kitap.is_some() {
                tema::rozet(ui, &format!("Seçili kitap: {} TL", para_formatla(secili_kitap_tutari)), tema::BASARI);
            }
        });

        if !self.metraj_kalemleri.is_empty() {
            ui.add_space(4.0);
            egui::CollapsingHeader::new(RichText::new("Özet döküm").color(tema::METIN_IKINCIL)).default_open(false).show(ui, |ui| {
                let mut kitap_toplamlari: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
                let mut birim_toplamlari: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
                for kalem in &self.metraj_kalemleri {
                    *kitap_toplamlari.entry(kalem.kitap_adi.clone()).or_insert(0.0) += kalem.tutar;
                    *birim_toplamlari.entry(kalem.birim.clone()).or_insert(0.0) += kalem.tutar;
                }
                ui.columns(2, |cols| {
                    cols[0].label(RichText::new("Kitap").strong());
                    for (kitap, toplam) in kitap_toplamlari.iter().take(6) {
                        cols[0].label(format!("{}: {} TL", metni_kisalt(kitap, 28), para_formatla(*toplam)));
                    }
                    cols[1].label(RichText::new("Birim").strong());
                    for (birim, toplam) in birim_toplamlari.iter().take(6) {
                        cols[1].label(format!("{}: {} TL", birim, para_formatla(*toplam)));
                    }
                });
            });
        }
    }

    // ==================== İCMAL / YAKLAŞIK MALİYET ====================
    fn render_icmal(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "📊", "Keşif İcmali / Yaklaşık Maliyet");
        ui.add_space(6.0);

        let secili = self.secili_grup_id.as_deref();
        // Üst düzey grupların canlı toplamları
        let grup_satirlari: Vec<(String, f64)> = self.is_gruplari.iter()
            .map(|g| (g.ad.clone(), grup_canli_toplam(g, secili, &self.metraj_kalemleri)))
            .collect();
        let ara_toplam: f64 = self.toplam_tutar();

        // Oran ayarları
        tema::kart(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Genel Gider + Müteahhit Kârı").color(tema::METIN_IKINCIL).size(12.0));
                if ui.add(egui::DragValue::new(&mut self.genel_gider_kar_orani).speed(0.5).range(0.0..=100.0).suffix(" %")).changed() { self.degisiklik_var = true; }
                ui.add_space(20.0);
                ui.label(RichText::new("KDV").color(tema::METIN_IKINCIL).size(12.0));
                if ui.add(egui::DragValue::new(&mut self.kdv_orani).speed(0.5).range(0.0..=100.0).suffix(" %")).changed() { self.degisiklik_var = true; }
            });
        });
        ui.add_space(8.0);

        if grup_satirlari.is_empty() {
            tema::bildirim_seridi(ui, "Henüz iş grubu yok. Metraj sekmesinden grup ve poz ekleyin.", tema::UYARI_KOYU, tema::UYARI, tema::UYARI);
            return;
        }

        // İş grupları icmal tablosu
        tema::kart(ui, |ui| {
            egui::Grid::new("icmal_grid").num_columns(4).spacing(egui::vec2(16.0, 9.0)).striped(true).show(ui, |ui| {
                ui.label(RichText::new("Sıra").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("İş Grubu").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Tutar (TL)").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Oran").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.end_row();
                for (i, (ad, tutar)) in grup_satirlari.iter().enumerate() {
                    let yuzde = if ara_toplam > 0.0 { tutar / ara_toplam * 100.0 } else { 0.0 };
                    ui.label(RichText::new(format!("{}", i + 1)).color(tema::METIN_SOLUK));
                    ui.label(RichText::new(ad).size(13.5).strong().color(tema::METIN));
                    ui.label(RichText::new(para_formatla(*tutar)).size(13.0).color(tema::METIN));
                    ui.label(RichText::new(format!("% {:.1}", yuzde)).size(12.5).color(tema::VURGU_HOVER));
                    ui.end_row();
                }
            });
        });
        ui.add_space(10.0);

        // Yaklaşık maliyet özeti
        let kar = ara_toplam * self.genel_gider_kar_orani / 100.0;
        let kdv_matrahi = ara_toplam + kar;
        let kdv = kdv_matrahi * self.kdv_orani / 100.0;
        let genel_toplam = kdv_matrahi + kdv;

        egui::Frame::default()
            .fill(tema::YUZEY_2)
            .stroke(egui::Stroke::new(1.0, tema::KENAR))
            .corner_radius(egui::CornerRadius::same(tema::KOSE))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                let satir = |ui: &mut egui::Ui, etiket: &str, deger: f64, vurgulu: bool| {
                    ui.horizontal(|ui| {
                        let renk = if vurgulu { tema::METIN } else { tema::METIN_IKINCIL };
                        ui.label(RichText::new(etiket).color(renk).size(if vurgulu { 14.0 } else { 13.0 }).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{} TL", para_formatla(deger))).color(if vurgulu { tema::BASARI } else { tema::METIN }).strong().size(if vurgulu { 16.0 } else { 13.0 }));
                        });
                    });
                };
                satir(ui, "Ara Toplam (İşçilik + Malzeme)", ara_toplam, false);
                ui.add_space(3.0);
                satir(ui, &format!("Genel Gider & Müteahhit Kârı (% {:.1})", self.genel_gider_kar_orani), kar, false);
                ui.add_space(3.0);
                satir(ui, "KDV Matrahı", kdv_matrahi, false);
                ui.add_space(3.0);
                satir(ui, &format!("KDV (% {:.1})", self.kdv_orani), kdv, false);
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                satir(ui, "TOPLAM YAKLAŞIK MALİYET", genel_toplam, true);
            });
    }

    // ==================== POZLAR ====================
    fn poz_formunu_yeni_icin_ac(&mut self) {
        if self.secili_kitap.is_none() {
            self.hata_mesaji = "Once pozun eklenecegi kitabi secin.".into();
            return;
        }
        self.poz_form_acik = true;
        self.poz_form_duzenleme = false;
        self.poz_form_eski_poz_no.clear();
        self.poz_form_poz_no.clear();
        self.poz_form_tanim.clear();
        self.poz_form_birim.clear();
        self.poz_form_fiyat.clear();
        self.poz_form_kategori = "Özel Poz".into();
    }

    fn poz_formunu_duzenleme_icin_ac(&mut self, poz: Poz) {
        self.poz_form_acik = true;
        self.poz_form_duzenleme = true;
        self.poz_form_eski_poz_no = poz.poz_no.clone();
        self.poz_form_poz_no = poz.poz_no;
        self.poz_form_tanim = poz.tanim;
        self.poz_form_birim = poz.birim;
        self.poz_form_fiyat = poz.fiyat.map(para_formatla).unwrap_or_default();
        self.poz_form_kategori = poz.kategori;
    }

    fn poz_form_fiyat_degeri(&mut self) -> Option<Option<f64>> {
        let fiyat = self.poz_form_fiyat.trim();
        if fiyat.is_empty() {
            return Some(None);
        }
        match fiyat.replace(',', ".").parse::<f64>() {
            Ok(deger) => Some(Some(deger)),
            Err(_) => {
                self.hata_mesaji = "Birim fiyat sayi olmali. Ornek: 1250,50".into();
                None
            }
        }
    }

    fn poz_form_kaydet(&mut self) {
        let kitap = match self.secili_kitap.clone() {
            Some(kitap) => kitap,
            None => {
                self.hata_mesaji = "Once kitap secin.".into();
                return;
            }
        };
        let poz_no = self.poz_form_poz_no.trim().to_string();
        let tanim = self.poz_form_tanim.trim().to_string();
        let birim = self.poz_form_birim.trim().to_string();
        let kategori = self.poz_form_kategori.trim().to_string();
        if poz_no.is_empty() || tanim.is_empty() || birim.is_empty() || kategori.is_empty() {
            self.hata_mesaji = "Poz no, aciklama, birim ve kategori zorunlu.".into();
            return;
        }
        let fiyat = match self.poz_form_fiyat_degeri() {
            Some(fiyat) => fiyat,
            None => return,
        };
        if let Some(ref db) = self.db {
            let sonuc = if self.poz_form_duzenleme {
                db.poz_guncelle(&kitap, &self.poz_form_eski_poz_no, &poz_no, &tanim, &birim, fiyat, &kategori)
            } else {
                db.poz_ekle(&kitap, &poz_no, &tanim, &birim, fiyat, &kategori)
            };
            match sonuc {
                Ok(()) => {
                    self.poz_form_acik = false;
                    self.basarili_mesaj = if self.poz_form_duzenleme {
                        format!("{} guncellendi.", poz_no)
                    } else {
                        format!("{} eklendi.", poz_no)
                    };
                    self.hata_mesaji.clear();
                    self.kitaplari_yenile();
                    self.kategorileri_yukle();
                    self.pozlar_tablosu_yenile();
                }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }

    fn render_poz_form_popup(&mut self, ctx: &egui::Context) {
        if !self.poz_form_acik {
            return;
        }
        let baslik = if self.poz_form_duzenleme { "Poz Düzenle" } else { "Poz Ekle" };
        let mut acik = self.poz_form_acik;
        let mut kaydet = false;
        let mut iptal = false;
        egui::Window::new(baslik)
            .collapsible(false)
            .resizable(false)
            .open(&mut acik)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                if let Some(ref kitap) = self.secili_kitap {
                    ui.label(RichText::new(format!("📚 {} ({}/{})", kitap.ad, kitap.ay, kitap.yil)).color(tema::METIN_IKINCIL).size(12.0));
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Poz No").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_poz_no).desired_width(220.0));
                });
                ui.label(RichText::new("Açıklama").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(TextEdit::multiline(&mut self.poz_form_tanim).desired_width(420.0).desired_rows(3));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Birim").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_birim).desired_width(80.0));
                    ui.label(RichText::new("B.Fiyat").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_fiyat).hint_text("boş olabilir").desired_width(120.0));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Kategori").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_kategori).desired_width(260.0));
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if tema::basari_buton(ui, "💾 Kaydet").clicked() { kaydet = true; }
                    if ui.button("İptal").clicked() { iptal = true; }
                });
            });
        self.poz_form_acik = acik;
        if iptal {
            self.poz_form_acik = false;
        }
        if kaydet {
            self.poz_form_kaydet();
        }
    }

    fn render_poz_sil_onay_popup(&mut self, ctx: &egui::Context) {
        let poz = match self.silinecek_poz.clone() {
            Some(poz) => poz,
            None => return,
        };
        let mut sil = false;
        let mut iptal = false;
        egui::Window::new("Poz Sil")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("⚠  Bu poz kalıcı olarak silinecek.").color(tema::UYARI));
                ui.add_space(4.0);
                ui.label(RichText::new(&poz.poz_no).monospace().strong().color(tema::METIN));
                ui.label(RichText::new(metni_kisalt(&poz.tanim, 90)).color(tema::METIN_IKINCIL));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if tema::tehlike_buton(ui, "🗑 Sil").clicked() { sil = true; }
                    if ui.button("Vazgeç").clicked() { iptal = true; }
                });
            });
        if iptal {
            self.silinecek_poz = None;
        }
        if sil {
            if let Some(ref db) = self.db {
                match db.poz_sil(poz.kitap_id, &poz.poz_no) {
                    Ok(()) => {
                        if self.secili_poz.as_ref().map(|p| p.poz_no == poz.poz_no && p.kitap_id == poz.kitap_id).unwrap_or(false) {
                            self.secili_poz = None;
                        }
                        self.silinecek_poz = None;
                        self.basarili_mesaj = format!("{} silindi.", poz.poz_no);
                        self.hata_mesaji.clear();
                        self.kitaplari_yenile();
                        self.kategorileri_yukle();
                        self.pozlar_tablosu_yenile();
                    }
                    Err(e) => self.hata_mesaji = format!("{}", e),
                }
            }
        }
    }

    fn render_pozlar_tablosu(&mut self, ui: &mut Ui) {
        if self.secili_kitap.as_ref().map(|k| k.id) != self.pozlar_yuklu_kitap_id {
            self.pozlar_tablosu_yenile();
        }

        tema::bolum_basligi(ui, "🔎", "Pozlar");
        ui.add_space(6.0);
        tema::kart(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Kitap").color(tema::METIN_IKINCIL).size(12.0));
                let km = self.secili_kitap.as_ref().map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil)).unwrap_or_else(|| "Kitap seçin".into());
                egui::ComboBox::from_id_salt("pozlar_kitap_secici").selected_text(&km).width(340.0).show_ui(ui, |ui| {
                    for k in self.kitaplar.clone() {
                        if ui.selectable_label(self.secili_kitap.as_ref().map(|sk| sk.id == k.id).unwrap_or(false), format!("{} ({}/{})", k.ad, k.ay, k.yil)).clicked() {
                            self.secili_kitap = Some(k);
                            self.pozlar_tablosu_yenile();
                        }
                    }
                });
            });
            if self.secili_kitap.is_some() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍").size(13.0));
                    if ui.add_sized(Vec2::new(340.0, 26.0), TextEdit::singleline(&mut self.pozlar_arama_metni).hint_text("poz no, açıklama, birim veya kategori")).changed() {
                        self.pozlar_tablosu_yenile();
                    }
                    if !self.pozlar_arama_metni.is_empty() && ui.button("Temizle").clicked() {
                        self.pozlar_arama_metni.clear();
                        self.pozlar_tablosu_yenile();
                    }
                    tema::rozet(ui, &format!("{} poz", self.pozlar_tablosu.len()), tema::METIN_IKINCIL);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if tema::birincil_buton(ui, "＋ Poz Ekle").clicked() {
                            self.poz_formunu_yeni_icin_ac();
                        }
                    });
                });
            }
        });
        ui.add_space(8.0);

        if self.secili_kitap.is_none() {
            tema::bildirim_seridi(ui, "Poz listesini görmek için bir kitap seçin.", tema::UYARI_KOYU, tema::UYARI, tema::UYARI);
            return;
        }

        if self.pozlar_tablosu.is_empty() {
            ui.label(RichText::new("Sonuç bulunamadı.").color(tema::METIN_SOLUK));
            return;
        }

        let pozlar = self.pozlar_tablosu.clone();
        ScrollArea::vertical().max_height(ui.available_height()).auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("pozlar_grid").num_columns(7).min_col_width(60.0).spacing(egui::vec2(12.0, 8.0)).striped(true).show(ui, |ui: &mut egui::Ui| {
                ui.label(RichText::new("Poz No").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Açıklama").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Birim").strong().size(12.0));
                ui.label(RichText::new("B.Fiyat").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Kategori").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Kitap").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("İşlem").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.end_row();

                for poz in pozlar {
                    let fiyat = poz.fiyat.map(|f| format!("{} TL", para_formatla(f))).unwrap_or_else(|| "Formül".into());
                    let fiyat_renk = if poz.fiyat.is_some() { tema::BASARI } else { tema::UYARI };
                    let aciklama = metni_kisalt(&poz.tanim, 85);
                    ui.label(RichText::new(&poz.poz_no).monospace().size(11.5).color(tema::METIN));
                    ui.label(RichText::new(aciklama).size(11.5).color(tema::METIN_IKINCIL)).on_hover_text(&poz.tanim);
                    ui.label(RichText::new(&poz.birim).size(11.0).color(tema::METIN_IKINCIL));
                    ui.label(RichText::new(fiyat).size(11.5).color(fiyat_renk));
                    ui.label(RichText::new(&poz.kategori).size(10.5).color(tema::METIN_SOLUK));
                    ui.label(RichText::new(format!("{}/{}", poz.ay, poz.yil)).size(10.5).color(tema::METIN_SOLUK));
                    ui.horizontal(|ui| {
                        if ui.button("✏ Düzenle").clicked() {
                            self.poz_formunu_duzenleme_icin_ac(poz.clone());
                        }
                        if ui.add(egui::Button::new(RichText::new("🗑").color(tema::TEHLIKE)).stroke(egui::Stroke::new(1.0, tema::KENAR))).clicked() {
                            self.silinecek_poz = Some(poz.clone());
                        }
                    });
                    ui.end_row();
                }
            });
        });
    }

    // ==================== PDF YUKLE ====================
    fn render_pdf_yukle(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "📄", "PDF Birim Fiyat Listesi Yükle");
        ui.add_space(6.0);
        if self.kitaplar.is_empty() {
            tema::bildirim_seridi(ui, "⚠  Önce Kitap Yöneticisi'nden bir kitap ekleyin.", tema::TEHLIKE_KOYU, tema::TEHLIKE, tema::TEHLIKE);
            return;
        }
        tema::kart(ui, |ui| {
            ui.label(RichText::new("PDF'i hangi kitaba yükleyeceğinizi seçin.").color(tema::METIN_IKINCIL).size(12.0));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hedef Kitap").color(tema::METIN_IKINCIL).size(12.0));
                let km = self.secili_kitap.as_ref().map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil)).unwrap_or_else(|| "Kitap seçin".into());
                egui::ComboBox::from_id_salt("pdf_kitap_secici").selected_text(&km).width(340.0).show_ui(ui, |ui| {
                    for k in &self.kitaplar.clone() { if ui.selectable_label(false, format!("{} ({}/{})", k.ad, k.ay, k.yil)).clicked() { self.secili_kitap = Some(k.clone()); } }
                });
            });
            ui.add_space(10.0);
            if self.pdf_yukleniyor {
                ui.horizontal(|ui| { ui.spinner(); ui.label(RichText::new("PDF işleniyor…").color(tema::METIN_IKINCIL)); });
            } else if tema::birincil_buton(ui, "📂 PDF Dosyası Seç ve Yükle").clicked() {
                self.pdf_sec_ve_yukle();
            }
            if !self.pdf_durumu.is_empty() { ui.add_space(6.0); ui.label(RichText::new(&self.pdf_durumu).color(tema::BASARI)); }
        });
        let alt = PathBuf::from("20206-05-BF.pdf");
        if alt.exists() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hızlı yükleme:").color(tema::METIN_SOLUK).size(12.0));
                if ui.button("📄 20206-05-BF.pdf").clicked() { self.pdf_yukle(alt); }
            });
        }
    }

    // ==================== YARDIMCI ====================
    fn toplam_tutar(&self) -> f64 {
        if self.is_gruplari.is_empty() {
            return self.metraj_kalemleri.iter().map(|k| k.tutar).sum();
        }
        let secili = self.secili_grup_id.as_deref();
        self.is_gruplari
            .iter()
            .map(|g| grup_canli_toplam(g, secili, &self.metraj_kalemleri))
            .sum()
    }

    // Aktif (seçili) grubun kalemlerini düzenleme tamponundan (metraj_kalemleri) ağaca geri yazar.
    fn aktif_grubu_senkronize(&mut self) {
        if let Some(id) = self.secili_grup_id.clone() {
            if let Some(g) = grup_bul_mut(&mut self.is_gruplari, &id) {
                g.kalemler = self.metraj_kalemleri.clone();
            }
        }
    }

    // Bir grubu aktif yapar: önceki aktif grubu kaydeder, yeni grubun kalemlerini tampona yükler.
    fn grup_sec(&mut self, id: String) {
        if self.secili_grup_id.as_deref() == Some(id.as_str()) {
            return;
        }
        self.aktif_grubu_senkronize();
        let kalemler = grup_bul_ref(&self.is_gruplari, &id)
            .map(|g| g.kalemler.clone())
            .unwrap_or_default();
        self.secili_grup_id = Some(id);
        self.metraj_kalemleri = kalemler;
        self.secili_poz = None;
    }

    // ==================== GERİ AL / YİNELE ====================
    fn mevcut_anlik(&self) -> Anlik {
        Anlik {
            is_gruplari: self.is_gruplari.clone(),
            metraj_kalemleri: self.metraj_kalemleri.clone(),
            secili_grup_id: self.secili_grup_id.clone(),
            metraj_adi: self.metraj_adi.clone(),
        }
    }
    // Değiştiren bir işlemden HEMEN ÖNCE çağrılır: mevcut durumu geri-al yığınına koyar.
    fn anlik_goruntu_al(&mut self) {
        self.aktif_grubu_senkronize();
        let a = self.mevcut_anlik();
        self.geri_al_yigini.push(a);
        if self.geri_al_yigini.len() > 50 {
            self.geri_al_yigini.remove(0);
        }
        self.yinele_yigini.clear();
    }
    fn anlik_uygula(&mut self, a: Anlik) {
        self.is_gruplari = a.is_gruplari;
        self.metraj_kalemleri = a.metraj_kalemleri;
        self.secili_grup_id = a.secili_grup_id;
        self.metraj_adi = a.metraj_adi;
        self.secili_poz = None;
        self.degisiklik_var = true;
    }
    fn geri_al(&mut self) {
        if let Some(a) = self.geri_al_yigini.pop() {
            self.aktif_grubu_senkronize();
            let mevcut = self.mevcut_anlik();
            self.yinele_yigini.push(mevcut);
            self.anlik_uygula(a);
            self.basarili_mesaj = "↩ Geri alındı.".into();
            self.hata_mesaji.clear();
        }
    }
    fn yinele(&mut self) {
        if let Some(a) = self.yinele_yigini.pop() {
            self.aktif_grubu_senkronize();
            let mevcut = self.mevcut_anlik();
            self.geri_al_yigini.push(mevcut);
            self.anlik_uygula(a);
            self.basarili_mesaj = "↪ Yinelendi.".into();
            self.hata_mesaji.clear();
        }
    }

    // ==================== OTOMATİK KAYIT ====================
    fn autosave_kontrol(&mut self, ctx: &egui::Context) {
        if !self.degisiklik_var {
            return;
        }
        let now = ctx.input(|i| i.time);
        if self.son_autosave == 0.0 {
            self.son_autosave = now; // ilk işaretleme; hemen kaydetme
            return;
        }
        if now - self.son_autosave < 30.0 {
            return;
        }
        self.son_autosave = now;
        let yol = self.autosave_yolu.clone();
        let m = self.proje_olustur();
        let _ = metraj_json_kaydet(&m, &yol);
    }

    // Hiyerarşik is_gruplari yapısını düzleştirip (eski sürümler için) kalemler ile birlikte döndürür.
    fn kayit_yapisi_hazirla(&mut self) -> (Vec<IsGrubu>, Vec<MetrajKalemi>) {
        self.aktif_grubu_senkronize();
        if self.is_gruplari.is_empty() {
            (vec![], self.metraj_kalemleri.clone())
        } else {
            let mut flat = Vec::new();
            for g in &self.is_gruplari {
                flat.extend(g.tum_kalemler_duz());
            }
            (self.is_gruplari.clone(), flat)
        }
    }
    fn kitaplari_yenile(&mut self) { if let Some(ref db) = self.db { if let Ok(k) = db.kitaplari_listele() { self.kitaplar = k; } } }
    fn metraj_kalemlerini_tekillestir(&mut self) -> usize {
        let mut birlesen = 0;
        let mut tekil: Vec<MetrajKalemi> = Vec::with_capacity(self.metraj_kalemleri.len());
        for kalem in self.metraj_kalemleri.drain(..) {
            if let Some(mevcut) = tekil.iter_mut().find(|m| m.poz_no == kalem.poz_no) {
                mevcut.miktar += kalem.miktar;
                mevcut.detaylar.extend(kalem.detaylar);
                mevcut.tutar_guncelle();
                birlesen += 1;
            } else {
                tekil.push(kalem);
            }
        }
        tekil.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
        self.metraj_kalemleri = tekil;
        birlesen
    }
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
    fn akilli_ara(&mut self) {
        self.poz_arama_metni.clear();
        self.aciklama_arama_metni.clear();
        self.kategori_pozlar.clear();
        let sorgu = self.akilli_arama_metni.trim();
        if sorgu.is_empty() {
            self.arama_sonuclari.clear();
            return;
        }
        if let Some(ref db) = self.db {
            let kid = self.secili_kitap.as_ref().map(|k| k.id);
            let mut sonuc: Vec<Poz> = Vec::new();
            let poz_gibi = sorgu.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
            if poz_gibi {
                if let Ok(mut pozlar) = db.poz_no_ara(sorgu, kid) {
                    sonuc.append(&mut pozlar);
                }
            }
            if (!poz_gibi || sonuc.len() < 20) && sorgu.split_whitespace().all(|t| !t.is_empty()) {
                if let Ok(pozlar) = db.tam_metin_ara(sorgu, kid) {
                    for poz in pozlar {
                        if !sonuc.iter().any(|p| p.poz_no == poz.poz_no && p.kitap_id == poz.kitap_id) {
                            sonuc.push(poz);
                        }
                    }
                }
            }
            sonuc.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
            sonuc.truncate(100);
            self.arama_sonuclari = sonuc;
        }
    }
    fn poz_no_ara(&mut self) { if self.poz_arama_metni.is_empty() { self.arama_sonuclari.clear(); return; } if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(s) = db.poz_no_ara(&self.poz_arama_metni, kid) { self.arama_sonuclari = s; } } }
    fn aciklama_ara(&mut self) { if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(s) = db.tam_metin_ara(&self.aciklama_arama_metni, kid) { self.arama_sonuclari = s; } } }
    fn poz_sorgula(&mut self) {
        let poz_no = self.yeni_poz_no.trim().to_string();
        if poz_no.is_empty() { self.secili_poz = None; return; }
        if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id);
            match db.poz_getir(&poz_no, kid) {
                Ok(Some(p)) => {
                    self.secili_poz = Some(p);
                    self.yeni_poz_no = poz_no;
                }
                Ok(None) => {
                    if let Ok(s) = db.poz_no_ara(&poz_no, kid) {
                        if s.len() == 1 {
                            self.secili_poz = Some(s[0].clone());
                            self.yeni_poz_no = s[0].poz_no.clone();
                        } else {
                            self.secili_poz = None;
                            self.arama_sonuclari = s;
                        }
                    }
                }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }
    fn kalem_ekle(&mut self) {
        let poz = match self.secili_poz.clone() {
            Some(p) => p,
            None => { self.hata_mesaji = "Once bir poz secin.".into(); return; }
        };
        // Gruplar varsa kalem mutlaka bir aktif gruba eklenir.
        if !self.is_gruplari.is_empty() && self.secili_grup_id.is_none() {
            self.hata_mesaji = "Önce soldaki ağaçtan bir iş grubu seçin.".into();
            self.basarili_mesaj.clear();
            return;
        }
        // metraj_kalemleri aktif grubun düzenleme tamponudur; aynı poz tekrar eklenmez.
        if self.metraj_kalemleri.iter().any(|k| k.poz_no == poz.poz_no) {
            self.basarili_mesaj = format!("{} zaten listede var. Miktarını düzenlemek için satıra tıklayın.", poz.poz_no);
            self.hata_mesaji.clear();
            return;
        }
        self.anlik_goruntu_al();
        let kalem = MetrajKalemi::yeni(&poz, 0.0);
        self.metraj_kalemleri.push(kalem);
        self.metraj_kalemleri.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
        self.aktif_grubu_senkronize();
        self.degisiklik_var = true;
        self.basarili_mesaj = format!("{} eklendi.", poz.poz_no);
        self.hata_mesaji.clear();
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
    // Mevcut durumdan kaydedilebilir proje nesnesi oluşturur (oranlar dahil).
    fn proje_olustur(&mut self) -> KayitliMetraj {
        let (is_gruplari, kalemler) = self.kayit_yapisi_hazirla();
        KayitliMetraj {
            ad: self.metraj_adi.clone(),
            kalemler,
            is_gruplari,
            tarih: krono_tarih(),
            genel_gider_kar_orani: self.genel_gider_kar_orani,
            kdv_orani: self.kdv_orani,
        }
    }
    fn metraj_kaydet(&mut self) {
        // Ileri donuk uyumluluk: hem hiyerarsik is_gruplari hem de duzlestirilmis kalemler yazilir
        let m = self.proje_olustur();
        if let Some(ref y) = self.mevcut_dosya_yolu { match metraj_json_kaydet(&m, y) { Ok(()) => { self.degisiklik_var = false; self.basarili_mesaj = format!("Kaydedildi: {}", y.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } }
        else if let Some(d) = rfd::FileDialog::new().add_filter("Metrajmatik Projesi", &["mrj"]).set_file_name(&format!("{}.mrj", self.metraj_adi)).save_file() { match metraj_json_kaydet(&m, &d) { Ok(()) => { self.mevcut_dosya_yolu = Some(d.clone()); self.degisiklik_var = false; self.basarili_mesaj = format!("Kaydedildi: {}", d.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } }
    }
    fn metraj_yukle_diyalog(&mut self) {
        if let Some(d) = rfd::FileDialog::new()
            .add_filter("Metrajmatik Projesi", &["mrj", "json"])
            .pick_file()
        {
            self.metraj_dosyadan_yukle(&d, true);
        }
    }
    // Bir dosyadan projeyi yükler. `dosya_olarak` true ise yol "mevcut dosya" olur (kurtarmada false).
    fn metraj_dosyadan_yukle(&mut self, d: &std::path::Path, dosya_olarak: bool) {
        match metraj_json_yukle(d) {
            Ok(m) => {
                let KayitliMetraj { ad, kalemler, is_gruplari, genel_gider_kar_orani, kdv_orani, .. } = m;
                self.genel_gider_kar_orani = genel_gider_kar_orani;
                self.kdv_orani = kdv_orani;
                self.geri_al_yigini.clear();
                self.yinele_yigini.clear();
                self.secili_grup_id = None;
                self.secili_poz = None;
                let mut birlesen = 0;

                if is_gruplari.is_empty() {
                    // Eski flat proje: kalemleri tekilleştir ve otomatik gruba aktar
                    self.is_gruplari = vec![];
                    self.metraj_kalemleri = kalemler;
                    birlesen = self.metraj_kalemlerini_tekillestir();
                    if !self.metraj_kalemleri.is_empty() {
                        self.is_gruplari = vec![
                            IsGrubu {
                                id: "otomatik_insaat".into(),
                                ad: "İnşaat".into(),
                                alt_gruplar: vec![
                                    IsGrubu {
                                        id: "otomatik_kaba_insaat".into(),
                                        ad: "Kaba İnşaat".into(),
                                        alt_gruplar: vec![],
                                        kalemler: std::mem::take(&mut self.metraj_kalemleri),
                                    },
                                ],
                                kalemler: vec![],
                            },
                        ];
                    }
                } else {
                    // Hiyerarşik proje: kalemler grupların içinde, tampon boş başlar
                    self.is_gruplari = is_gruplari;
                    self.metraj_kalemleri = vec![];
                }

                // İlk yaprak grubu aktif yap ve kalemlerini tampona yükle
                if let Some(id) = ilk_yaprak_grup_id(&self.is_gruplari) {
                    self.grup_sec(id);
                }

                self.metraj_adi = ad;
                if dosya_olarak {
                    self.mevcut_dosya_yolu = Some(d.to_path_buf());
                    self.degisiklik_var = birlesen > 0;
                    self.basarili_mesaj = if birlesen > 0 {
                        format!("Açıldı: {} ({} yinelenen poz birleştirildi)", d.display(), birlesen)
                    } else {
                        format!("Açıldı: {}", d.display())
                    };
                } else {
                    // Kurtarma: kaydedilmemiş sayılır
                    self.degisiklik_var = true;
                    self.basarili_mesaj = "Otomatik kayıttan kurtarıldı. Lütfen 'Kaydet' ile kalıcı hale getirin.".into();
                }
            }
            Err(e) => self.hata_mesaji = format!("{}", e),
        }
    }
    fn metraj_excel_diyalog(&mut self) {
        let m = self.proje_olustur();
        if let Some(d) = rfd::FileDialog::new().add_filter("Excel", &["xlsx"]).set_file_name(&format!("{}.xlsx", self.metraj_adi)).save_file() { match metraj_excel_aktar(&m, &d) { Ok(()) => { self.basarili_mesaj = format!("Excel: {}", d.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } }
    }

    fn fiyatlari_guncelle(&mut self) {
        let hedef_kitap = match self.fiyat_guncelle_hedef.clone() {
            Some(k) => k,
            None => { self.hata_mesaji = "Lutfen hedef kitap secin!".into(); return; }
        };
        // Aktif grubun tampondaki kalemlerini ağaca yaz ki güncelleme tüm gruplara uygulansın
        self.anlik_goruntu_al();
        if let Some(ref db) = self.db {
            let mut guncellenen = 0;
            let mut bulunamayan = 0;
            // Kitap bazlı sayaç: (eski_kitap_adi, guncellenen, bulunamayan)
            let mut kitap_bazli: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();

            // Tek bir kalemi güncelleyen kapanış (closure)
            let mut kalem_guncelle = |kalem: &mut MetrajKalemi| {
                let eski_kitap = kalem.kitap_adi.clone();
                if let Ok(Some(poz)) = db.poz_getir(&kalem.poz_no, Some(hedef_kitap.id)) {
                    if let Some(yeni_fiyat) = poz.fiyat {
                        kalem.birim_fiyat = yeni_fiyat;
                        kalem.kitap_adi = format!("{} ({}/{})", hedef_kitap.ad, hedef_kitap.ay, hedef_kitap.yil);
                        kalem.tutar_guncelle();
                        guncellenen += 1;
                        let entry = kitap_bazli.entry(eski_kitap).or_insert((0, 0));
                        entry.0 += 1;
                        return;
                    }
                }
                bulunamayan += 1;
                let entry = kitap_bazli.entry(eski_kitap).or_insert((0, 0));
                entry.1 += 1;
            };

            // Ağaçtaki tüm grupları gezerek her kalemi güncelle
            fn agaci_gez(gruplar: &mut [IsGrubu], f: &mut dyn FnMut(&mut MetrajKalemi)) {
                for g in gruplar.iter_mut() {
                    for kalem in g.kalemler.iter_mut() { f(kalem); }
                    agaci_gez(&mut g.alt_gruplar, f);
                }
            }

            if self.is_gruplari.is_empty() {
                for kalem in self.metraj_kalemleri.iter_mut() { kalem_guncelle(&mut *kalem); }
            } else {
                agaci_gez(&mut self.is_gruplari, &mut kalem_guncelle);
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
        // Aktif grubun tampondaki kalemlerini güncellenmiş ağaçtan tazele
        if let Some(id) = self.secili_grup_id.clone() {
            if let Some(g) = grup_bul_ref(&self.is_gruplari, &id) {
                self.metraj_kalemleri = g.kalemler.clone();
            }
        }
    }
}

// ==================== İŞ GRUBU AĞAÇ YARDIMCILARI ====================
// Ağaçta verilen id'ye sahip grubu bulup değiştirilebilir referans döndürür.
fn grup_bul_mut<'a>(gruplar: &'a mut [IsGrubu], hedef_id: &str) -> Option<&'a mut IsGrubu> {
    for g in gruplar.iter_mut() {
        if g.id == hedef_id {
            return Some(g);
        }
        if let Some(bulunan) = grup_bul_mut(&mut g.alt_gruplar, hedef_id) {
            return Some(bulunan);
        }
    }
    None
}

// Ağaçta verilen id'ye sahip grubu bulup salt-okunur referans döndürür.
fn grup_bul_ref<'a>(gruplar: &'a [IsGrubu], hedef_id: &str) -> Option<&'a IsGrubu> {
    for g in gruplar.iter() {
        if g.id == hedef_id {
            return Some(g);
        }
        if let Some(bulunan) = grup_bul_ref(&g.alt_gruplar, hedef_id) {
            return Some(bulunan);
        }
    }
    None
}

// Ağaçtan verilen id'ye sahip grubu (ve alt ağacını) siler.
fn grup_sil(gruplar: &mut Vec<IsGrubu>, hedef_id: &str) -> bool {
    if let Some(pos) = gruplar.iter().position(|g| g.id == hedef_id) {
        gruplar.remove(pos);
        return true;
    }
    for g in gruplar.iter_mut() {
        if grup_sil(&mut g.alt_gruplar, hedef_id) {
            return true;
        }
    }
    false
}

// Ağaçtaki ilk yaprak (alt grubu olmayan) grubun id'sini döndürür.
fn ilk_yaprak_grup_id(gruplar: &[IsGrubu]) -> Option<String> {
    for g in gruplar {
        if g.alt_gruplar.is_empty() {
            return Some(g.id.clone());
        }
        if let Some(id) = ilk_yaprak_grup_id(&g.alt_gruplar) {
            return Some(id);
        }
    }
    None
}

// Bir grubun canlı toplamı: aktif grubun kalemleri düzenleme tamponundan (aktif_kalemler) okunur.
fn grup_canli_toplam(grup: &IsGrubu, secili_id: Option<&str>, aktif_kalemler: &[MetrajKalemi]) -> f64 {
    let kalemler_toplam: f64 = if secili_id == Some(grup.id.as_str()) {
        aktif_kalemler.iter().map(|k| k.tutar).sum()
    } else {
        grup.kalemler.iter().map(|k| k.tutar).sum()
    };
    let alt_toplam: f64 = grup
        .alt_gruplar
        .iter()
        .map(|g| grup_canli_toplam(g, secili_id, aktif_kalemler))
        .sum();
    kalemler_toplam + alt_toplam
}

// İş grupları ağacını çizer; tıklanan grubun id'sini secilen_out'a yazar.
fn is_grubu_agac_ciz(
    ui: &mut Ui,
    gruplar: &[IsGrubu],
    secili_id: Option<&str>,
    aktif_kalemler: &[MetrajKalemi],
    secilen_out: &mut Option<String>,
) {
    for g in gruplar {
        let secili = secili_id == Some(g.id.as_str());
        let toplam = grup_canli_toplam(g, secili_id, aktif_kalemler);
        let yaprak = g.alt_gruplar.is_empty();
        let ikon = if yaprak { "📄" } else { "📁" };
        let ad_rengi = if secili { Color32::WHITE } else { tema::METIN };
        let tutar_rengi = if toplam > 0.0 { tema::BASARI } else { tema::METIN_SOLUK };

        let mut job = egui::text::LayoutJob::default();
        job.append(&format!("{} ", ikon), 0.0, egui::TextFormat { font_id: egui::FontId::proportional(13.5), color: ad_rengi, ..Default::default() });
        job.append(&g.ad, 0.0, egui::TextFormat { font_id: egui::FontId::proportional(13.5), color: ad_rengi, ..Default::default() });
        job.append(&format!("   {} TL", para_formatla(toplam)), 0.0, egui::TextFormat { font_id: egui::FontId::proportional(11.5), color: tutar_rengi, ..Default::default() });

        if ui.add(egui::SelectableLabel::new(secili, job)).clicked() {
            *secilen_out = Some(g.id.clone());
        }
        if !yaprak {
            ui.indent(g.id.clone(), |ui| {
                is_grubu_agac_ciz(ui, &g.alt_gruplar, secili_id, aktif_kalemler, secilen_out);
            });
        }
    }
}

// ==================== MİKTAR DETAY (BOYUT) YARDIMCILARI ====================
fn opt_str(o: Option<f64>) -> String {
    o.map(|v| format!("{}", v).replace('.', ",")).unwrap_or_default()
}

// Bir popup satırının boyutlarından miktarı hesaplar (hiç boyut yoksa None).
fn satir_miktar(s: &PopupDetaySatiri) -> Option<f64> {
    let a = sayi_oku(&s.adet);
    let e = sayi_oku(&s.en);
    let b = sayi_oku(&s.boy);
    let y = sayi_oku(&s.yukseklik);
    if a.is_none() && e.is_none() && b.is_none() && y.is_none() {
        return None;
    }
    Some(a.unwrap_or(1.0) * e.unwrap_or(1.0) * b.unwrap_or(1.0) * y.unwrap_or(1.0))
}

fn satir_to_detay(s: &PopupDetaySatiri) -> Option<MiktarDetay> {
    let m = satir_miktar(s)?;
    Some(MiktarDetay {
        aciklama: s.aciklama.clone(),
        miktar: m,
        adet: sayi_oku(&s.adet),
        en: sayi_oku(&s.en),
        boy: sayi_oku(&s.boy),
        yukseklik: sayi_oku(&s.yukseklik),
    })
}

fn detay_to_satir(d: &MiktarDetay) -> PopupDetaySatiri {
    if d.boyutlu_mu() {
        PopupDetaySatiri {
            aciklama: d.aciklama.clone(),
            adet: opt_str(d.adet),
            en: opt_str(d.en),
            boy: opt_str(d.boy),
            yukseklik: opt_str(d.yukseklik),
        }
    } else {
        // Eski/elle girilmiş detay: miktarı "Adet" sütununa koy
        PopupDetaySatiri {
            aciklama: d.aciklama.clone(),
            adet: if d.miktar != 0.0 { opt_str(Some(d.miktar)) } else { String::new() },
            ..Default::default()
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

fn para_formatla(deger: f64) -> String {
    let isaret = if deger.is_sign_negative() { "-" } else { "" };
    let yuvarlanmis = format!("{:.2}", deger.abs());
    let mut parcalar = yuvarlanmis.split('.');
    let tam = parcalar.next().unwrap_or("0");
    let kurus = parcalar.next().unwrap_or("00");
    let mut gruplu_ters = String::new();
    for (idx, ch) in tam.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            gruplu_ters.push('.');
        }
        gruplu_ters.push(ch);
    }
    let gruplu: String = gruplu_ters.chars().rev().collect();
    format!("{}{},{}", isaret, gruplu, kurus)
}

fn sayi_oku(metin: &str) -> Option<f64> {
    let mut temiz = metin.trim().replace(' ', "");
    if temiz.contains(',') {
        temiz = temiz.replace('.', "").replace(',', ".");
    }
    if temiz.is_empty() {
        return None;
    }
    temiz.parse::<f64>().ok()
}
