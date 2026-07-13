//! Uygulama kabuğu: durum (`MetrajApp`), başlangıç değerleri ve `eframe::App::update`
//! akışı (kısayollar, modallar, menü/durum çubuğu, sekme dağıtımı).
//!
//! Ekran çizimleri ve UI-dışı iş mantığı alt modüllere ayrılmıştır:
//! - [`gorunum_metraj`]: Metraj sekmesi (arama, iş grupları, kalem tablosu, miktar popup)
//! - [`gorunum_diger`]: Kitap yöneticisi, İcmal, Pozlar, PDF yükleme
//! - [`islemler`]: arama, dosya, geri-al/yinele, otomatik kayıt, fiyat güncelleme

use eframe::egui;
use egui::{Color32, RichText, TextEdit};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::bicim::{metni_kisalt, para_formatla};
use crate::database::Veritabani;
use crate::is_grubu::ilk_yaprak_grup_id;
use crate::models::{HesapTuru, IsGrubu, Kitap, MetrajKalemi, Poz};
use crate::tema;

mod analiz_ui;
mod gorunum_diger;
mod gorunum_metraj;
mod islemler;

#[derive(Debug, Clone, PartialEq)]
enum Sekme { MetrajTablosu, Icmal, Pozlar, KitapYoneticisi, PdfYukle }

/// Miktar popup'ında bir detay satırının düzenlenebilir (metin) hali.
#[derive(Default, Clone)]
pub(crate) struct PopupDetaySatiri {
    aciklama: String,
    adet: String,
    en: String,
    boy: String,
    yukseklik: String,
    cikan: bool,
}

/// Analiz popup'ında düzenlenebilir bir girdi satırı (katsayı/tür metin olarak tutulur).
#[derive(Default, Clone)]
pub(crate) struct AnalizGirdiSatiri {
    girdi_no: String,
    tanim: String,
    birim: String,
    birim_fiyat: f64,
    miktar_metni: String,
    tur: String,
}

/// Geri al/yinele için projenin düzenlenebilir durumunun anlık görüntüsü.
#[derive(Clone)]
pub(crate) struct Anlik {
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
    popup_imalat_cinsi: String,

    // Analiz popup
    analiz_popup_acik: bool,
    analiz_poz: Option<Poz>,
    analiz_girdileri: Vec<AnalizGirdiSatiri>,
    analiz_kar_orani: f64,
    analiz_rayic_arama: String,
    analiz_rayic_sonuc: Vec<Poz>,
    analizli_pozlar: HashSet<String>, // Pozlar sekmesinde analizli poz rozeti için

    // Hiyerarşik İş Grupları alanları
    is_gruplari: Vec<crate::models::IsGrubu>,
    secili_grup_id: Option<String>,
    yeni_grup_adi: String,

    // İcmal / yaklaşık maliyet oranları
    hesap_turu: HesapTuru,
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

/// Uygulama veri dizini: `%APPDATA%\Metrajmatik` (yoksa çalışma dizini). Oluşturulur.
fn veri_dizini() -> PathBuf {
    let taban = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let dizin = taban.join("Metrajmatik");
    let _ = std::fs::create_dir_all(&dizin);
    dizin
}

/// Dosyayı veri dizinindeki yolu ile döndürür. Yeni konumda yoksa ama eski (çalışma
/// dizini) konumda varsa, veri kaybını önlemek için bir kereye mahsus taşır
/// (kopyalar; SQLite WAL/SHM yan dosyaları dahil). Kopyalama, taşıma bozulursa eski
/// veri elde kalsın diyedir.
fn veri_yolu(dosya_adi: &str) -> PathBuf {
    let dizin = veri_dizini();
    let yeni = dizin.join(dosya_adi);
    let eski = PathBuf::from(dosya_adi);
    if !yeni.exists() && eski.exists() {
        for ek in ["", "-wal", "-shm"] {
            let kaynak = PathBuf::from(format!("{}{}", dosya_adi, ek));
            if kaynak.exists() {
                let _ = std::fs::copy(&kaynak, dizin.join(format!("{}{}", dosya_adi, ek)));
            }
        }
    }
    yeni
}

impl Default for MetrajApp {
    fn default() -> Self {
        let db_yolu = veri_yolu("metrajmatik_veriler.db");
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
        let autosave_yolu = veri_yolu("metrajmatik_autosave.mrj");
        let kurtarma_mevcut = autosave_yolu.exists();
        Self {
            db, poz_sayisi, kitaplar, secili_kitap: None,
            metraj_kalemleri: vec![], metraj_adi: "Isimsiz Metraj".into(),
            mevcut_dosya_yolu: None, degisiklik_var: false,
            poz_arama_metni: String::new(), akilli_arama_metni: String::new(), arama_sonuclari: vec![], secili_poz: None,
            aciklama_arama_metni: String::new(), yeni_poz_no: String::new(),
            yeni_kitap_adi: String::new(), yeni_kitap_yil: 2026, yeni_kitap_ay: 5,
            duzenlenen_kitap: None, duzenleme_adi: String::new(),
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
            popup_imalat_cinsi: String::new(),

            // Analiz popup
            analiz_popup_acik: false,
            analiz_poz: None,
            analiz_girdileri: vec![],
            analiz_kar_orani: 25.0,
            analiz_rayic_arama: String::new(),
            analiz_rayic_sonuc: vec![],
            analizli_pozlar: HashSet::new(),

            // Hiyerarşik İş Grupları
            is_gruplari: baslangic_gruplari,
            secili_grup_id: baslangic_secili,
            yeni_grup_adi: String::new(),

            // İcmal oranları (varsayılan): yeni proje Kamu → KDV hariç, kâr %0
            // (kurum birim fiyatları kâr+genel gideri zaten içerir).
            hesap_turu: HesapTuru::Kamu,
            genel_gider_kar_orani: 0.0,
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
                    ui.label("Kurum Adı:");
                    ui.add(TextEdit::singleline(&mut self.duzenleme_adi).desired_width(320.0));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if tema::basari_buton(ui, "✓ Kaydet").clicked() {
                            if let Some(ref db) = self.db {
                                let kitap_id = self.duzenlenen_kitap.as_ref().unwrap().id;
                                let _ = db.kitap_guncelle(kitap_id, &self.duzenleme_adi);
                                self.basarili_mesaj = format!("'{}' güncellendi.", self.duzenleme_adi);
                                self.duzenlenen_kitap = None;
                                self.kitaplari_yenile();
                                if let Some(ref mut sk) = self.secili_kitap {
                                    if sk.id == kitap_id { sk.ad = self.duzenleme_adi.clone(); }
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

        // Birim fiyat analizi popup'ı
        self.render_analiz_popup(ctx);

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
