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
use std::path::{Path, PathBuf};

use crate::bicim::{metni_kisalt, para_formatla};
use crate::database::Veritabani;
use crate::is_grubu::ilk_yaprak_grup_id;
use crate::models::{
    Donem, Hakedis, HesapTuru, IsGrubu, IsProgrami, Kitap, MetrajKalemi, Poz, ProjeAsamasi,
    ProjeBilgi, SozlesmeAyarlari,
};
use crate::tema;

mod analiz_ui;
mod gorunum_diger;
mod gorunum_metraj;
mod hakedis_ui;
mod is_programi_ui;
mod islemler;
mod proje_ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sekme {
    Proje,
    MetrajTablosu,
    Icmal,
    Hakedis,
    IsProgrami,
    Pozlar,
    KitapYoneticisi,
    PdfYukle,
}

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
    bekleyen_acma_yolu: Option<PathBuf>,
    bekleyen_geri_yukleme_yolu: Option<PathBuf>,
    kapanis_onayi: bool,
    kapanisa_izin_ver: bool,
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
    silinecek_kitap: Option<Kitap>,
    fiyat_guncelle_hedef: Option<Kitap>,
    // Rayiç güncelleme modalı
    fiyat_guncelle_acik: bool,
    fiyat_guncelle_endeks_mod: bool, // false = kuruma göre, true = endekse (Yİ-ÜFE) göre
    fiyat_guncelle_en_son: bool,     // kurum kipinde: en son fiyat mı, seçilen dönem mi
    fiyat_guncelle_yil: u32,
    fiyat_guncelle_ay: u32,
    fiyat_endeks_temel: f64,
    fiyat_endeks_guncel: f64,
    cift_tiklama_ekle: bool,
    pdf_durumu: String,
    pdf_yukleniyor: bool,
    import_profili: String, // PDF ayrıştırma profili: "Otomatik" | "Çevre ve Şehircilik" | "Genel"
    aktif_sekme: Sekme,
    hata_mesaji: String,
    basarili_mesaj: String,
    son_hata_mesaji: String,
    son_basarili_mesaj: String,
    hata_gosterim_baslangici: f64,
    basari_gosterim_baslangici: f64,
    silinecek_grup_id: Option<String>,
    metraj_temizleme_onayi: bool,
    kategoriler: Vec<String>,
    secili_kategori: String,
    kategori_pozlar: Vec<Poz>,
    pozlar_arama_metni: String,
    pozlar_tablosu: Vec<Poz>,
    pozlar_yuklu_kitap_id: Option<i64>,
    pozlar_donem: Option<(u32, u32)>, // Pozlar sekmesinde seçili dönem (None = en son)
    pozlar_donemler: Vec<Donem>,      // Seçili kurumun dönemleri (dönem seçici için)
    poz_form_acik: bool,
    poz_form_duzenleme: bool,
    poz_form_eski_poz_no: String,
    poz_form_poz_no: String,
    poz_form_tanim: String,
    poz_form_birim: String,
    poz_form_fiyat: String,
    poz_form_kategori: String,
    poz_form_yil: u32,
    poz_form_ay: u32,
    poz_form_teklifler: String, // fiyat araştırması: boşlukla ayrılmış teklifler
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

    // Nakliye popup (taşıma bedeli = birim fiyat × miktar × mesafe)
    nakliye_popup_acik: bool,
    nakliye_poz: Option<Poz>,
    nakliye_miktar: String,
    nakliye_mesafe: String,

    // Hiyerarşik İş Grupları alanları
    is_gruplari: Vec<crate::models::IsGrubu>,
    secili_grup_id: Option<String>,
    yeni_grup_adi: String,

    // İcmal / yaklaşık maliyet oranları
    hesap_turu: HesapTuru,
    genel_gider_kar_orani: f64,
    kdv_orani: f64,

    // Hakediş
    hakedisler: Vec<Hakedis>,
    secili_hakedis: Option<usize>,
    hakedis_detay_acik: bool,           // yeşil defter ölçü kırılımı popup'ı
    hakedis_detay_satir: Option<usize>, // seçili hakedişte hangi satır

    // İş programı (pursantajlı zaman planı)
    is_programi: IsProgrami,

    // Proje künyesi (idare, iş adı, İKN — resmî çıktı başlıkları)
    proje_bilgi: ProjeBilgi,
    proje_asamasi: ProjeAsamasi,
    sozlesme_ayarlari: SozlesmeAyarlari,
    hakedise_donusum_onayi: bool,

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
    let taban = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
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

const AUTOSAVE_DOSYA_ADI: &str = "metrajmatik_autosave.mrj";

/// Eski sürümlerin autosave bırakabildiği çalışma ve uygulama dizinleri.
fn eski_autosave_yollari() -> Vec<PathBuf> {
    let mut yollari = vec![PathBuf::from(AUTOSAVE_DOSYA_ADI)];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dizin) = exe.parent() {
            let exe_yolu = dizin.join(AUTOSAVE_DOSYA_ADI);
            if !yollari.contains(&exe_yolu) {
                yollari.push(exe_yolu);
            }
        }
    }
    yollari
}

/// Eski autosave'i yeni konuma yalnız bir kez taşır ve kaynak kopyaları tüketir.
/// Kaynağı yerinde bırakan normal veri göçü autosave için uygun değildir; kullanıcı
/// yeni dosyayı sildiğinde eski kopya sonraki açılışta yeniden canlanır.
fn eski_autosave_kopyalarini_tuket(yeni: &Path, eskiler: &[PathBuf]) -> std::io::Result<()> {
    for eski in eskiler {
        if eski == yeni || !eski.exists() {
            continue;
        }
        if yeni.exists() {
            std::fs::remove_file(eski)?;
            continue;
        }
        if std::fs::rename(eski, yeni).is_err() {
            std::fs::copy(eski, yeni)?;
            std::fs::remove_file(eski)?;
        }
    }
    Ok(())
}

fn autosave_veri_yolu() -> PathBuf {
    let yeni = veri_dizini().join(AUTOSAVE_DOSYA_ADI);
    if let Err(e) = eski_autosave_kopyalarini_tuket(&yeni, &eski_autosave_yollari()) {
        log::warn!("Eski otomatik kayıt temizlenemedi: {}", e);
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
            Err(e) => {
                log::error!("{}", e);
                (None, 0, vec![])
            }
        };
        // Yeni/boş proje için veritabanındaki varsayılan iş grupları şablonunu yükle
        let baslangic_gruplari = db
            .as_ref()
            .and_then(|vt| vt.varsayilan_gruplari_getir().ok())
            .unwrap_or_default();
        let baslangic_secili = ilk_yaprak_grup_id(&baslangic_gruplari);
        let autosave_yolu = autosave_veri_yolu();
        let kurtarma_mevcut = autosave_yolu.exists();
        Self {
            db,
            poz_sayisi,
            kitaplar,
            secili_kitap: None,
            metraj_kalemleri: vec![],
            metraj_adi: "Isimsiz Metraj".into(),
            mevcut_dosya_yolu: None,
            degisiklik_var: false,
            bekleyen_acma_yolu: None,
            bekleyen_geri_yukleme_yolu: None,
            kapanis_onayi: false,
            kapanisa_izin_ver: false,
            poz_arama_metni: String::new(),
            akilli_arama_metni: String::new(),
            arama_sonuclari: vec![],
            secili_poz: None,
            aciklama_arama_metni: String::new(),
            yeni_poz_no: String::new(),
            yeni_kitap_adi: String::new(),
            yeni_kitap_yil: 2026,
            yeni_kitap_ay: 5,
            duzenlenen_kitap: None,
            duzenleme_adi: String::new(),
            silinecek_kitap: None,
            fiyat_guncelle_hedef: None,
            fiyat_guncelle_acik: false,
            fiyat_guncelle_endeks_mod: false,
            fiyat_guncelle_en_son: true,
            fiyat_guncelle_yil: 2026,
            fiyat_guncelle_ay: 5,
            fiyat_endeks_temel: 100.0,
            fiyat_endeks_guncel: 100.0,
            cift_tiklama_ekle: false,
            pdf_durumu: String::new(),
            pdf_yukleniyor: false,
            import_profili: "Otomatik".into(),
            aktif_sekme: Sekme::Proje,
            hata_mesaji: String::new(),
            basarili_mesaj: String::new(),
            son_hata_mesaji: String::new(),
            son_basarili_mesaj: String::new(),
            hata_gosterim_baslangici: 0.0,
            basari_gosterim_baslangici: 0.0,
            silinecek_grup_id: None,
            metraj_temizleme_onayi: false,
            kategoriler: vec![],
            secili_kategori: "TÜMÜ".into(),
            kategori_pozlar: vec![],
            pozlar_arama_metni: String::new(),
            pozlar_tablosu: vec![],
            pozlar_yuklu_kitap_id: None,
            pozlar_donem: None,
            pozlar_donemler: vec![],
            poz_form_acik: false,
            poz_form_duzenleme: false,
            poz_form_eski_poz_no: String::new(),
            poz_form_poz_no: String::new(),
            poz_form_tanim: String::new(),
            poz_form_birim: String::new(),
            poz_form_fiyat: String::new(),
            poz_form_kategori: String::new(),
            poz_form_yil: 2026,
            poz_form_ay: 1,
            poz_form_teklifler: String::new(),
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

            nakliye_popup_acik: false,
            nakliye_poz: None,
            nakliye_miktar: String::new(),
            nakliye_mesafe: String::new(),

            // Hiyerarşik İş Grupları
            is_gruplari: baslangic_gruplari,
            secili_grup_id: baslangic_secili,
            yeni_grup_adi: String::new(),

            // İcmal oranları (varsayılan): yeni proje Kamu → KDV hariç, kâr %0
            // (kurum birim fiyatları kâr+genel gideri zaten içerir).
            hesap_turu: HesapTuru::Kamu,
            genel_gider_kar_orani: 0.0,
            kdv_orani: 20.0,

            // Hakediş
            hakedisler: vec![],
            secili_hakedis: None,
            hakedis_detay_acik: false,
            hakedis_detay_satir: None,

            // İş programı
            is_programi: IsProgrami::default(),

            // Proje künyesi
            proje_bilgi: ProjeBilgi::default(),
            proje_asamasi: ProjeAsamasi::Metraj,
            sozlesme_ayarlari: SozlesmeAyarlari::default(),
            hakedise_donusum_onayi: false,

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

impl MetrajApp {
    fn sekme_ac(&mut self, sekme: Sekme) {
        self.aktif_sekme = sekme;
        if matches!(
            sekme,
            Sekme::MetrajTablosu | Sekme::Pozlar | Sekme::KitapYoneticisi
        ) {
            self.kitaplari_yenile();
        }
        if sekme == Sekme::MetrajTablosu {
            self.kategorileri_yukle();
        }
        if sekme == Sekme::Pozlar {
            self.pozlar_tablosu_yenile();
        }
    }

    fn bildirimleri_guncelle(&mut self, ctx: &egui::Context) {
        let simdi = ctx.input(|i| i.time);
        let yeni_hata =
            self.hata_mesaji != self.son_hata_mesaji && !self.hata_mesaji.trim().is_empty();
        let yeni_basari = self.basarili_mesaj != self.son_basarili_mesaj
            && !self.basarili_mesaj.trim().is_empty();

        if yeni_hata {
            self.hata_gosterim_baslangici = simdi;
            self.basarili_mesaj.clear();
        } else if yeni_basari {
            self.basari_gosterim_baslangici = simdi;
            self.hata_mesaji.clear();
        }

        if !self.hata_mesaji.is_empty() && simdi - self.hata_gosterim_baslangici > 9.0 {
            self.hata_mesaji.clear();
        }
        if !self.basarili_mesaj.is_empty() && simdi - self.basari_gosterim_baslangici > 4.5 {
            self.basarili_mesaj.clear();
        }

        self.son_hata_mesaji.clone_from(&self.hata_mesaji);
        self.son_basarili_mesaj.clone_from(&self.basarili_mesaj);
    }

    fn render_bildirimler(&mut self, ctx: &egui::Context) {
        let (mesaj, hata) = if !self.hata_mesaji.is_empty() {
            (self.hata_mesaji.clone(), true)
        } else if !self.basarili_mesaj.is_empty() {
            (self.basarili_mesaj.clone(), false)
        } else {
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        let mut kapat = false;
        egui::Area::new(egui::Id::new("bildirim_toast"))
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 72.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_max_width(420.0);
                egui::Frame::default()
                    .fill(if hata {
                        tema::TEHLIKE_KOYU
                    } else {
                        tema::BASARI_KOYU
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if hata { tema::TEHLIKE } else { tema::BASARI },
                    ))
                    .corner_radius(egui::CornerRadius::same(tema::KOSE))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 5],
                        blur: 18,
                        spread: 0,
                        color: Color32::from_black_alpha(130),
                    })
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.label(
                                RichText::new(if hata { "⚠" } else { "✓" })
                                    .size(17.0)
                                    .color(if hata { tema::TEHLIKE } else { tema::BASARI }),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(mesaj).color(tema::METIN).size(13.0),
                                )
                                .wrap(),
                            );
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("✕").color(tema::METIN_IKINCIL),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE),
                                )
                                .on_hover_text("Bildirimi kapat")
                                .clicked()
                            {
                                kapat = true;
                            }
                        });
                    });
            });
        if kapat {
            self.hata_mesaji.clear();
            self.basarili_mesaj.clear();
        }
    }
}

fn gezinti_butonu(
    ui: &mut egui::Ui,
    aktif: bool,
    ikon: &str,
    baslik: &str,
    aciklama: &str,
    kilitli: bool,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let zemin = if aktif {
            tema::VURGU_SOLUK
        } else if response.hovered() {
            tema::YUZEY_3
        } else {
            Color32::TRANSPARENT
        };
        let kenar = if aktif {
            tema::VURGU
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(tema::KOSE_KUCUK),
            zemin,
            egui::Stroke::new(1.0, kenar),
            egui::StrokeKind::Inside,
        );
        let yazi = if aktif {
            Color32::WHITE
        } else {
            tema::METIN_IKINCIL
        };
        ui.painter().text(
            egui::pos2(rect.left() + 20.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            ikon,
            egui::FontId::new(18.0, tema::ikon_fontu()),
            if aktif { tema::VURGU_HOVER } else { yazi },
        );
        ui.painter().text(
            egui::pos2(rect.left() + 40.0, rect.top() + 18.0),
            egui::Align2::LEFT_CENTER,
            baslik,
            egui::FontId::proportional(12.5),
            yazi,
        );
        ui.painter().text(
            egui::pos2(rect.left() + 40.0, rect.top() + 35.0),
            egui::Align2::LEFT_CENTER,
            aciklama,
            egui::FontId::proportional(10.5),
            tema::METIN_SOLUK,
        );
        if kilitli {
            ui.painter().text(
                egui::pos2(rect.right() - 15.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                tema::ikon::KILIT,
                egui::FontId::new(13.0, tema::ikon_fontu()),
                tema::UYARI,
            );
        }
    }
    response.on_hover_text(aciklama).clicked()
}

fn mobil_gezinti_butonu(ui: &mut egui::Ui, aktif: bool, ikon: &str, baslik: &str) -> bool {
    let genislik = if baslik.len() > 8 { 118.0 } else { 98.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(genislik, 34.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(tema::KOSE_KUCUK),
            if aktif { tema::VURGU } else { tema::YUZEY_2 },
            egui::Stroke::new(1.0, if aktif { tema::VURGU } else { tema::KENAR }),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            egui::pos2(rect.left() + 18.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            ikon,
            egui::FontId::new(15.0, tema::ikon_fontu()),
            if aktif {
                Color32::WHITE
            } else {
                tema::METIN_IKINCIL
            },
        );
        ui.painter().text(
            egui::pos2(rect.left() + 34.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            baslik,
            egui::FontId::proportional(12.0),
            if aktif {
                Color32::WHITE
            } else {
                tema::METIN_IKINCIL
            },
        );
    }
    response.clicked()
}

impl eframe::App for MetrajApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.bildirimleri_guncelle(ctx);
        if !self.kapanisa_izin_ver
            && self.degisiklik_var
            && ctx.input(|i| i.viewport().close_requested())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.kapanis_onayi = true;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.metraj_kaydet();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            self.metraj_yukle_diyalog();
        }
        // Geri al / yinele (Ctrl+Z, Ctrl+Y veya Ctrl+Shift+Z)
        if self.proje_asamasi == ProjeAsamasi::Metraj
            && ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z))
        {
            self.geri_al();
        }
        if self.proje_asamasi == ProjeAsamasi::Metraj
            && ctx.input(|i| {
                i.modifiers.ctrl
                    && (i.key_pressed(egui::Key::Y)
                        || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))
            })
        {
            self.yinele();
        }
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
                                self.basarili_mesaj =
                                    format!("'{}' güncellendi.", self.duzenleme_adi);
                                self.duzenlenen_kitap = None;
                                self.kitaplari_yenile();
                                if let Some(ref mut sk) = self.secili_kitap {
                                    if sk.id == kitap_id {
                                        sk.ad = self.duzenleme_adi.clone();
                                    }
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
        self.render_kitap_sil_onay_popup(ctx);
        self.render_metraj_onaylari(ctx);
        self.render_veritabani_geri_yukleme_onayi(ctx);

        // Miktar detay popup'ı
        self.render_miktar_popup(ctx);

        // Birim fiyat analizi popup'ı
        self.render_analiz_popup(ctx);

        // Nakliye popup'ı
        self.render_nakliye_popup(ctx);

        // Hakediş yeşil defter kırılımı popup'ı
        self.render_hakedis_detay_popup(ctx);

        // Rayiç/fiyat güncelleme modalı
        self.render_fiyat_guncelle_popup(ctx);

        // Veri kaybı onayları diğer pencerelerin üstünde kalmalı.
        self.render_kaydetme_onaylari(ctx);

        let genis_pencere = ctx.screen_rect().width() >= 980.0;
        let proje_adi = if self.metraj_adi.trim().is_empty() {
            "Adsız proje".to_owned()
        } else {
            self.metraj_adi.clone()
        };
        let asama_etiketi = if self.proje_asamasi == ProjeAsamasi::Metraj {
            "METRAJ AŞAMASI"
        } else {
            "HAKEDİŞ AŞAMASI"
        };
        let dosya_durumu = if self.degisiklik_var {
            "Kaydedilmemiş değişiklik"
        } else if self.mevcut_dosya_yolu.is_some() {
            "Tüm değişiklikler kayıtlı"
        } else {
            "Yeni proje"
        };

        egui::TopBottomPanel::top("workspace_header")
            .exact_height(68.0)
            .frame(
                egui::Frame::default()
                    .fill(tema::YUZEY)
                    .stroke(egui::Stroke::new(1.0, tema::KENAR_YUMUSAK))
                    .inner_margin(egui::Margin::symmetric(18, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    egui::Frame::default()
                        .fill(tema::AKSAN)
                        .corner_radius(egui::CornerRadius::same(9))
                        .inner_margin(egui::Margin::symmetric(11, 8))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("M")
                                    .size(19.0)
                                    .strong()
                                    .color(tema::ARKA_PLAN),
                            );
                        });
                    ui.add_space(3.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("METRAJMATİK").size(14.5).strong());
                        ui.label(
                            RichText::new("Keşiften hakedişe tek çalışma alanı")
                                .size(10.5)
                                .color(tema::METIN_SOLUK),
                        );
                    });
                    if genis_pencere {
                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&proje_adi).size(14.0).strong());
                            ui.label(
                                RichText::new(asama_etiketi)
                                    .size(10.0)
                                    .strong()
                                    .color(tema::AKSAN),
                            );
                        });
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if tema::birincil_buton(ui, "Kaydet").clicked() {
                            self.metraj_kaydet();
                        }
                        if genis_pencere {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(dosya_durumu).size(11.5).color(
                                    if self.degisiklik_var {
                                        tema::UYARI
                                    } else {
                                        tema::BASARI
                                    },
                                ));
                                ui.label(
                                    RichText::new(format!(
                                        "{} kalem  ·  {} TL",
                                        self.metraj_kalemleri.len(),
                                        para_formatla(self.toplam_tutar())
                                    ))
                                    .size(10.5)
                                    .color(tema::METIN_SOLUK),
                                );
                            });
                        }
                    });
                });
            });

        if !genis_pencere {
            egui::TopBottomPanel::top("mobile_navigation")
                .frame(
                    egui::Frame::default()
                        .fill(tema::YUZEY)
                        .inner_margin(egui::Margin::symmetric(10, 7)),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (sekme, ikon, ad) in [
                                (Sekme::Proje, tema::ikon::PROJE, "Proje"),
                                (Sekme::MetrajTablosu, tema::ikon::METRAJ, "Metraj"),
                                (Sekme::Icmal, tema::ikon::ICMAL, "İcmal"),
                                (Sekme::Hakedis, tema::ikon::HAKEDIS, "Hakediş"),
                                (Sekme::IsProgrami, tema::ikon::IS_PROGRAMI, "Program"),
                                (Sekme::Pozlar, tema::ikon::POZLAR, "Pozlar"),
                                (Sekme::KitapYoneticisi, tema::ikon::KITAPLAR, "Kitaplar"),
                                (Sekme::PdfYukle, tema::ikon::PDF_AKTAR, "Aktar"),
                            ] {
                                if mobil_gezinti_butonu(ui, self.aktif_sekme == sekme, ikon, ad) {
                                    self.sekme_ac(sekme);
                                }
                            }
                        });
                    });
                });
        }

        // ÖNEMLİ: Alt durum çubuğu CentralPanel'den ÖNCE eklenmeli; aksi halde merkez
        // içerik pencerenin en altına kadar uzar ve durum çubuğu içeriğin üzerine biner.
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::default()
                    .fill(tema::YUZEY)
                    .inner_margin(egui::Margin::symmetric(12, 5)),
            )
            .show(ctx, |ui| {
                let dar = ui.available_width() < 900.0;
                ui.horizontal(|ui| {
                    let durum = if self.mevcut_dosya_yolu.is_some() {
                        if self.degisiklik_var {
                            ("● Kaydedilmedi", tema::UYARI)
                        } else {
                            ("✓ Kayıtlı", tema::BASARI)
                        }
                    } else {
                        ("○ Yeni proje", tema::METIN_SOLUK)
                    };
                    tema::rozet(ui, durum.0, durum.1);
                    if !dar {
                        if let Some(ref k) = self.secili_kitap {
                            tema::rozet(
                                ui,
                                &format!("Kitap  ·  {}", metni_kisalt(&k.ad, 30)),
                                tema::METIN_IKINCIL,
                            );
                        }
                        tema::rozet(ui, &format!("{} poz", self.poz_sayisi), tema::METIN_IKINCIL);
                        tema::rozet(
                            ui,
                            &format!("{} kalem", self.metraj_kalemleri.len()),
                            tema::METIN_IKINCIL,
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} TL", para_formatla(self.toplam_tutar())))
                                .color(tema::BASARI)
                                .strong()
                                .size(14.0),
                        );
                        ui.label(
                            RichText::new("Genel Toplam:")
                                .color(tema::METIN_SOLUK)
                                .size(12.0),
                        );
                    });
                });
            });

        if genis_pencere {
            egui::SidePanel::left("primary_navigation")
                .exact_width(224.0)
                .resizable(false)
                .frame(
                    egui::Frame::default()
                        .fill(tema::YUZEY)
                        .stroke(egui::Stroke::new(1.0, tema::KENAR_YUMUSAK))
                        .inner_margin(egui::Margin::symmetric(12, 16)),
                )
                .show(ctx, |ui| {
                    for (grup, ogeler) in [
                        (
                            "ÇALIŞMA",
                            vec![
                                (
                                    Sekme::Proje,
                                    tema::ikon::PROJE,
                                    "Proje Merkezi",
                                    "Künye ve genel durum",
                                    false,
                                ),
                                (
                                    Sekme::MetrajTablosu,
                                    tema::ikon::METRAJ,
                                    "Metraj",
                                    "Poz, grup ve miktarlar",
                                    false,
                                ),
                                (
                                    Sekme::Icmal,
                                    tema::ikon::ICMAL,
                                    "İcmal",
                                    "Maliyet ve genel toplam",
                                    false,
                                ),
                            ],
                        ),
                        (
                            "SÖZLEŞME",
                            vec![
                                (
                                    Sekme::Hakedis,
                                    tema::ikon::HAKEDIS,
                                    "Hakediş",
                                    "Sözleşme ve ödemeler",
                                    false,
                                ),
                                (
                                    Sekme::IsProgrami,
                                    tema::ikon::IS_PROGRAMI,
                                    "İş Programı",
                                    "Pursantaj ve zaman planı",
                                    self.proje_asamasi == ProjeAsamasi::Metraj,
                                ),
                            ],
                        ),
                        (
                            "KÜTÜPHANE",
                            vec![
                                (
                                    Sekme::Pozlar,
                                    tema::ikon::POZLAR,
                                    "Poz Kütüphanesi",
                                    "Birim fiyat verileri",
                                    false,
                                ),
                                (
                                    Sekme::KitapYoneticisi,
                                    tema::ikon::KITAPLAR,
                                    "Fiyat Kitapları",
                                    "Kurum ve dönemler",
                                    false,
                                ),
                                (
                                    Sekme::PdfYukle,
                                    tema::ikon::PDF_AKTAR,
                                    "PDF'den Aktar",
                                    "Yeni veri kaynağı",
                                    false,
                                ),
                            ],
                        ),
                    ] {
                        ui.label(
                            RichText::new(grup)
                                .size(10.0)
                                .strong()
                                .color(tema::METIN_SOLUK),
                        );
                        ui.add_space(4.0);
                        for (sekme, ikon, ad, aciklama, kilitli) in ogeler {
                            if gezinti_butonu(
                                ui,
                                self.aktif_sekme == sekme,
                                ikon,
                                ad,
                                aciklama,
                                kilitli,
                            ) {
                                self.sekme_ac(sekme);
                            }
                        }
                        ui.add_space(12.0);
                    }
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        ui.label(
                            RichText::new(if self.proje_asamasi == ProjeAsamasi::Metraj {
                                "SONRAKİ AŞAMA  ·  HAKEDİŞ"
                            } else {
                                "AKTİF AŞAMA  ·  HAKEDİŞ"
                            })
                            .size(10.0)
                            .strong()
                            .color(tema::AKSAN),
                        );
                    });
                });
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(tema::ARKA_PLAN)
                    .inner_margin(egui::Margin::same(if genis_pencere { 18 } else { 10 })),
            )
            .show(ctx, |ui| {
            // Kurtarma şeridi: otomatik kayıt dosyası varsa
            if self.kurtarma_mevcut {
                let mut kurtar = false;
                let mut autosave_sil = false;
                egui::Frame::default()
                    .fill(tema::UYARI_KOYU)
                    .stroke(egui::Stroke::new(1.0, tema::UYARI))
                    .corner_radius(egui::CornerRadius::same(tema::KOSE_KUCUK))
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("⟲  Önceki oturumdan kurtarılabilir otomatik kayıt bulundu.").color(tema::UYARI));
                            if tema::birincil_buton(ui, "Kurtar").clicked() { kurtar = true; }
                            if tema::tehlike_buton(ui, "🗑 Otomatik Kaydı Sil")
                                .on_hover_text("Bu kurtarma dosyasını kalıcı olarak siler")
                                .clicked()
                            {
                                autosave_sil = true;
                            }
                        });
                    });
                ui.add_space(6.0);
                if kurtar {
                    let yol = self.autosave_yolu.clone();
                    self.metraj_dosyadan_yukle(&yol, false);
                    self.kurtarma_mevcut = false;
                }
                if autosave_sil {
                    self.autosave_dosyasini_sil(true);
                }
            }
            match self.aktif_sekme {
                Sekme::MetrajTablosu => self.render_metraj_tablosu(ui),
                Sekme::Icmal => self.render_icmal(ui),
                Sekme::Proje => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.render_proje(ui));
                }
                Sekme::Hakedis => self.render_hakedis(ui),
                Sekme::IsProgrami => self.render_is_programi(ui),
                Sekme::Pozlar => self.render_pozlar_tablosu(ui),
                Sekme::KitapYoneticisi => self.render_kitap_yoneticisi(ui),
                Sekme::PdfYukle => self.render_pdf_yukle(ui),
            }
        });
        self.render_bildirimler(ctx);
    }
}

#[cfg(test)]
mod testler {
    use super::eski_autosave_kopyalarini_tuket;
    use std::path::PathBuf;

    fn gecici_dizin() -> PathBuf {
        let benzersiz = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("sistem saati")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "metrajmatik-autosave-test-{}-{}",
            std::process::id(),
            benzersiz
        ))
    }

    #[test]
    fn eski_autosave_bir_kez_tasinir_ve_tekrar_canlanmaz() {
        let dizin = gecici_dizin();
        std::fs::create_dir_all(&dizin).expect("geçici dizin");
        let yeni = dizin.join("appdata-autosave.mrj");
        let eski = dizin.join("eski-autosave.mrj");

        std::fs::write(&eski, "eski kayıt").expect("eski kayıt");
        eski_autosave_kopyalarini_tuket(&yeni, std::slice::from_ref(&eski)).expect("ilk taşıma");
        assert_eq!(std::fs::read_to_string(&yeni).unwrap(), "eski kayıt");
        assert!(!eski.exists());

        // Eski konumda yeniden bir artık bulunsa bile yeni kayıt değiştirilmez;
        // artık kaynak tüketilir ve sonraki silmede tekrar geri gelemez.
        std::fs::write(&eski, "bayat kayıt").expect("bayat kayıt");
        eski_autosave_kopyalarini_tuket(&yeni, std::slice::from_ref(&eski))
            .expect("artık temizliği");
        assert_eq!(std::fs::read_to_string(&yeni).unwrap(), "eski kayıt");
        assert!(!eski.exists());

        std::fs::remove_dir_all(dizin).expect("geçici dizin temizliği");
    }
}
