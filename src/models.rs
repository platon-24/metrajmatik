use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

static KALEM_KIMLIK_SAYACI: AtomicU64 = AtomicU64::new(1);

/// Proje içindeki bir metraj kalemini poz numarasından bağımsız olarak tanımlar.
/// Aynı poz farklı iş gruplarında birden fazla kez kullanılabildiği için hakediş
/// bağlantıları bu kalıcı kimlik üzerinden kurulur.
pub fn yeni_kalem_id() -> String {
    let zaman = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let sayac = KALEM_KIMLIK_SAYACI.fetch_add(1, Ordering::Relaxed);
    format!("k_{zaman:x}_{sayac:x}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poz {
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub fiyat: Option<f64>,
    pub kategori: String,
    pub kitap_id: i64,
    pub kitap_adi: String,
    pub yil: u32,
    pub ay: u32,
}

/// Bir kurum (kitap). yil/ay artık kurumun EN SON dönemini (görüntü) taşır;
/// dönemler ayrı bir kavramdır (bkz. [`Donem`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kitap {
    pub id: i64,
    pub ad: String,
    pub yil: u32,
    pub ay: u32,
    pub poz_sayisi: u32,
    pub tarih: String,
}

/// Bir kurumun sahip olduğu bir fiyat dönemi (yıl/ay) ve o dönemdeki poz sayısı.
#[derive(Debug, Clone)]
pub struct Donem {
    pub yil: u32,
    pub ay: u32,
    pub poz_sayisi: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiktarDetay {
    pub aciklama: String,
    pub miktar: f64, // hesaplanmış sonuç (boyutlar varsa onların çarpımı, yoksa elle girilen)
    #[serde(default)]
    pub adet: Option<f64>,
    #[serde(default)]
    pub en: Option<f64>,
    #[serde(default)]
    pub boy: Option<f64>,
    #[serde(default)]
    pub yukseklik: Option<f64>,
    #[serde(default)]
    pub cikan: bool, // true ise miktar düşülür (çıkan: boşluk / pencere / kapı vb.)
}

impl MiktarDetay {
    /// Boyutlardan (işaretli) miktarı hesaplar. Hiç boyut girilmemişse elle girilen
    /// `miktar` korunur. `cikan` ise sonuç negatiftir (metrajdan düşülür).
    pub fn hesaplanan_miktar(&self) -> f64 {
        let buyukluk = if self.adet.is_none()
            && self.en.is_none()
            && self.boy.is_none()
            && self.yukseklik.is_none()
        {
            self.miktar
        } else {
            self.adet.unwrap_or(1.0)
                * self.en.unwrap_or(1.0)
                * self.boy.unwrap_or(1.0)
                * self.yukseklik.unwrap_or(1.0)
        };
        if self.cikan {
            -buyukluk.abs()
        } else {
            buyukluk
        }
    }

    /// Boyut tabanlı mı yoksa elle girilmiş mi?
    pub fn boyutlu_mu(&self) -> bool {
        self.adet.is_some() || self.en.is_some() || self.boy.is_some() || self.yukseklik.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetrajKalemi {
    #[serde(default)]
    pub id: String,
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub birim_fiyat: f64,
    pub miktar: f64,
    pub tutar: f64,
    pub kitap_adi: String,
    pub detaylar: Vec<MiktarDetay>,
    #[serde(default)]
    pub imalat_cinsi: String, // metrajın neyi ölçtüğü (ör. "Zemin kat perde duvarları")
    #[serde(default)]
    pub kitap_id: i64, // analiz föyüne bağlanmak için kurum kimliği (0 = bilinmiyor)
}

impl MetrajKalemi {
    pub fn yeni(poz: &Poz, miktar: f64) -> Self {
        let birim_fiyat = poz.fiyat.unwrap_or(0.0);
        let tutar = birim_fiyat * miktar;
        MetrajKalemi {
            id: yeni_kalem_id(),
            poz_no: poz.poz_no.clone(),
            tanim: poz.tanim.clone(),
            birim: poz.birim.clone(),
            birim_fiyat,
            miktar,
            tutar,
            kitap_adi: format!("{} ({}/{})", poz.kitap_adi, poz.ay, poz.yil),
            detaylar: Vec::new(),
            imalat_cinsi: String::new(),
            kitap_id: poz.kitap_id,
        }
    }

    pub fn detaylardan_miktar_hesapla(&mut self) {
        // Her detayın miktarını boyutlarından tazele, sonra topla
        for d in self.detaylar.iter_mut() {
            d.miktar = d.hesaplanan_miktar();
        }
        self.miktar = self.detaylar.iter().map(|d| d.miktar).sum();
        self.tutar_guncelle();
    }

    pub fn tutar_guncelle(&mut self) {
        self.tutar = crate::bicim::kurus_yuvarla(self.birim_fiyat * self.miktar);
    }
}

/// Bir birim fiyat analizinin tek girdisi (rayiç: işçilik / malzeme / makine kalemi).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalizGirdisi {
    pub girdi_no: String, // rayiç / girdi poz numarası
    pub tanim: String,
    pub birim: String,
    pub birim_fiyat: f64, // girdinin birim fiyatı (rayiçten alınan değer)
    pub miktar: f64,      // katsayı: 1 birim imalat için gereken girdi miktarı
    pub tur: String,      // "İşçilik" | "Malzeme" | "Makine"
}

impl AnalizGirdisi {
    pub fn tutar(&self) -> f64 {
        self.miktar * self.birim_fiyat
    }
}

/// Analiz girdilerinin (kâr + genel gider hariç) ara toplamı.
pub fn analiz_ara_toplam(girdiler: &[AnalizGirdisi]) -> f64 {
    girdiler.iter().map(|g| g.tutar()).sum()
}

/// Analizden çıkan birim fiyat = ara toplam × (1 + kâr/genel gider oranı).
/// Kamu yaklaşık maliyetinde %25'in uygulandığı yer BURASIDIR (hazır kurum birim
/// fiyatları değil — onlar bu oranı zaten içerir). Bkz. [[mevzuat]] / rapor H1.
pub fn analiz_birim_fiyat(girdiler: &[AnalizGirdisi], kar_orani: f64) -> f64 {
    crate::bicim::kurus_yuvarla(analiz_ara_toplam(girdiler) * (1.0 + kar_orani / 100.0))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsGrubu {
    pub id: String,
    pub ad: String,
    pub alt_gruplar: Vec<IsGrubu>,
    pub kalemler: Vec<MetrajKalemi>,
}

impl IsGrubu {
    pub fn yeni(ad: &str) -> Self {
        let id = format!(
            "g_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        IsGrubu {
            id,
            ad: ad.to_string(),
            alt_gruplar: Vec::new(),
            kalemler: Vec::new(),
        }
    }

    pub fn toplam_tutar(&self) -> f64 {
        let kalemler_toplam: f64 = self.kalemler.iter().map(|k| k.tutar).sum();
        let alt_gruplar_toplam: f64 = self.alt_gruplar.iter().map(|g| g.toplam_tutar()).sum();
        kalemler_toplam + alt_gruplar_toplam
    }

    // Geriye dönük uyumluluk için altındaki tüm kalemleri düzleştirip döndürür
    pub fn tum_kalemler_duz(&self) -> Vec<MetrajKalemi> {
        let mut sonuc = self.kalemler.clone();
        for alt in &self.alt_gruplar {
            sonuc.extend(alt.tum_kalemler_duz());
        }
        sonuc
    }
}

/// Yaklaşık maliyet hesap türü.
/// - **Kamu**: Kamu ihalesi. Yaklaşık maliyet **KDV hariç** hesaplanır. Kâr + genel
///   gider yalnızca analizle (rayiçten) bulunan özel pozlara uygulanır; kurum birim
///   fiyatları bunu zaten içerdiğinden varsayılan oran %0'dır.
/// - **Ozel**: Özel sektör. KDV dahil; kâr + genel gider kullanıcı denetiminde.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HesapTuru {
    #[default]
    Kamu,
    Ozel,
}

impl HesapTuru {
    pub fn kamu_mu(self) -> bool {
        matches!(self, HesapTuru::Kamu)
    }
}

/// Bir hakedişte tek bir iş kaleminin yeşil defter (kümülatif yapılan) miktarı.
/// `detaylar` doluysa kümülatif miktar onların toplamıdır (ataşman/ölçü kırılımı).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HakedisSatiri {
    #[serde(default)]
    pub kalem_id: String,
    pub poz_no: String,
    pub kumulatif_miktar: f64, // bu hakedişe kadar YAPILAN toplam (yeşil defter)
    #[serde(default)]
    pub detaylar: Vec<MiktarDetay>, // yeşil defter / ataşman ölçü kırılımı
}

impl HakedisSatiri {
    /// Ölçü kırılımı (detaylar) varsa kümülatif miktarı onlardan tazeler.
    pub fn detaylardan_tazele(&mut self) {
        if !self.detaylar.is_empty() {
            for d in self.detaylar.iter_mut() {
                d.miktar = d.hesaplanan_miktar();
            }
            self.kumulatif_miktar = self.detaylar.iter().map(|d| d.miktar).sum();
        }
    }
}

/// Bir hakediş (progress payment): kümülatif imalat miktarları + kesintiler +
/// fiyat farkı (Yİ-ÜFE) + KDV/tevkifat. Sözleşme (keşif) kalemleri projeden gelir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hakedis {
    pub no: u32,
    pub tarih: String,
    pub tur: String, // "İlk" | "Ara" | "Kesin"
    #[serde(default)]
    pub satirlar: Vec<HakedisSatiri>,
    #[serde(default = "hakedis_damga_orani")]
    pub damga_orani: f64, // binde (‰) — damga vergisi
    #[serde(default)]
    pub teminat_orani: f64, // % — kesin teminat kesintisi
    #[serde(default)]
    pub sgk_orani: f64, // % — SGK
    #[serde(default)]
    pub avans_mahsup: f64, // TL — avans mahsubu
    #[serde(default)]
    pub fiyat_farki: f64, // TL — elle fiyat farkı (ff_uygula=false ise kullanılır)
    // Fiyat farkı (Yİ-ÜFE, otomatik): F = An × B × (güncel/temel − 1)
    #[serde(default)]
    pub ff_uygula: bool,
    #[serde(default = "hakedis_ff_b")]
    pub ff_b: f64, // B katsayısı (genelde 0,90)
    #[serde(default)]
    pub ff_temel_endeks: f64, // Po — ihale/teklif ayı Yİ-ÜFE
    #[serde(default)]
    pub ff_guncel_endeks: f64, // Pn — uygulama ayı Yİ-ÜFE
    // KDV / tevkifat (bilgi amaçlı)
    #[serde(default = "varsayilan_kdv_orani")]
    pub kdv_orani: f64,
    #[serde(default)]
    pub tevkifat_orani: f64, // 0,4 = 4/10 tevkifat
    /// Yeni fiyat farkı modeli. Eski ff_* alanları dosya uyumluluğu için korunur.
    #[serde(default)]
    pub fiyat_farki_ayari: FiyatFarkiAyari,
}

impl Hakedis {
    pub fn yeni(no: u32, tur: &str, tarih: String) -> Self {
        Hakedis {
            no,
            tarih,
            tur: tur.to_string(),
            satirlar: Vec::new(),
            damga_orani: 9.48,
            teminat_orani: 0.0,
            sgk_orani: 0.0,
            avans_mahsup: 0.0,
            fiyat_farki: 0.0,
            ff_uygula: false,
            ff_b: 0.90,
            ff_temel_endeks: 0.0,
            ff_guncel_endeks: 0.0,
            kdv_orani: 20.0,
            tevkifat_orani: 0.0,
            fiyat_farki_ayari: FiyatFarkiAyari::default(),
        }
    }
    /// Verilen keşif kaleminin bu hakedişteki kümülatif miktarı. Eski proje
    /// dosyalarında kimlik bulunmadığı için poz numarası yalnızca geriye dönük
    /// uyumluluk amacıyla son çare olarak kullanılır.
    pub fn kumulatif(&self, kalem_id: &str, poz_no: &str) -> f64 {
        if !kalem_id.is_empty() {
            if let Some(satir) = self.satirlar.iter().find(|s| s.kalem_id == kalem_id) {
                return satir.kumulatif_miktar;
            }
        }
        self.satirlar
            .iter()
            .find(|s| s.kalem_id.is_empty() && s.poz_no == poz_no)
            .map(|s| s.kumulatif_miktar)
            .unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FiyatFarkiYontemi {
    #[default]
    Yok,
    Manuel,
    TekEndeks,
    YapimAgirlikli,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiyatFarkiBileseni {
    pub kod: String,
    pub ad: String,
    #[serde(default)]
    pub katsayi: f64,
    #[serde(default)]
    pub temel_endeks: f64,
    #[serde(default)]
    pub guncel_endeks: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiyatFarkiAyari {
    #[serde(default)]
    pub yontem: FiyatFarkiYontemi,
    #[serde(default = "hakedis_ff_b")]
    pub b: f64,
    /// Tek endeks hesabında Po'nun ait olduğu ay (`YYYY-AA`).
    #[serde(default)]
    pub temel_ayi: String,
    #[serde(default)]
    pub uygulama_ayi: String,
    #[serde(default = "varsayilan_fiyat_farki_bilesenleri")]
    pub bilesenler: Vec<FiyatFarkiBileseni>,
}

impl Default for FiyatFarkiAyari {
    fn default() -> Self {
        Self {
            yontem: FiyatFarkiYontemi::Yok,
            b: 0.90,
            temel_ayi: String::new(),
            uygulama_ayi: String::new(),
            bilesenler: varsayilan_fiyat_farki_bilesenleri(),
        }
    }
}

impl FiyatFarkiAyari {
    pub fn normalize(&mut self) {
        if self.bilesenler.len() != 7 {
            self.bilesenler = varsayilan_fiyat_farki_bilesenleri();
        }
    }
}

fn varsayilan_fiyat_farki_bilesenleri() -> Vec<FiyatFarkiBileseni> {
    [
        ("a", "İşçilik"),
        ("b1", "Metalik olmayan mineral ürünler"),
        ("b2", "Demir ve çelik ürünleri"),
        ("b3", "Katı / sıvı yakıtlar"),
        ("b4", "Ağaç ve mantar ürünleri"),
        ("b5", "Diğer malzemeler"),
        ("c", "Makine ve ekipman amortismanı"),
    ]
    .into_iter()
    .map(|(kod, ad)| FiyatFarkiBileseni {
        kod: kod.into(),
        ad: ad.into(),
        katsayi: 0.0,
        temel_endeks: 0.0,
        guncel_endeks: 0.0,
    })
    .collect()
}

fn hakedis_damga_orani() -> f64 {
    9.48
}
fn hakedis_ff_b() -> f64 {
    0.90
}

/// Taşınabilir veri paketi: bir kurumun tüm pozları + dönem fiyatları (.mvp dosyası).
/// Kurum kitaplarını paylaşmak/dağıtmak için (veri paketi iş modeli).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaketPoz {
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub kategori: String,
    pub fiyatlar: Vec<(u32, u32, Option<f64>)>, // (yıl, ay, fiyat)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeriPaketi {
    pub kurum: String,
    pub pozlar: Vec<PaketPoz>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KayitliMetraj {
    pub ad: String,
    #[serde(default)]
    pub kalemler: Vec<MetrajKalemi>, // Eski flat projeler için
    #[serde(default)]
    pub is_gruplari: Vec<IsGrubu>, // Yeni hiyerarşik yapı için
    pub tarih: String,
    #[serde(default = "varsayilan_kar_orani")]
    pub genel_gider_kar_orani: f64, // % müteahhit kârı + genel gider
    #[serde(default = "varsayilan_kdv_orani")]
    pub kdv_orani: f64, // % KDV
    // Eski dosyalar bu alan OLMADAN kaydedildiği için varsayılanı ÖZEL'dir: böylece
    // eski projelerin KDV dahil toplamı açılışta sessizce değişmez. Yeni projeler
    // MetrajApp::default içinde KAMU olarak başlar.
    #[serde(default = "varsayilan_hesap_turu")]
    pub hesap_turu: HesapTuru,
    #[serde(default)]
    pub hakedisler: Vec<Hakedis>,
    #[serde(default)]
    pub is_programi: IsProgrami,
    #[serde(default)]
    pub proje_bilgi: ProjeBilgi,
    #[serde(default)]
    pub proje_asamasi: ProjeAsamasi,
    #[serde(default)]
    pub sozlesme_ayarlari: SozlesmeAyarlari,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjeAsamasi {
    #[default]
    Metraj,
    Hakedis,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenzilatYontemi {
    #[default]
    ManuelOran,
    SozlesmeBedelinden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SozlesmeAyarlari {
    /// Dönüşüm anındaki metraj toplamı; sözleşme bazını sonradan değişmeye karşı dondurur.
    #[serde(default)]
    pub kesif_bedeli: f64,
    #[serde(default)]
    pub sozlesme_bedeli: f64,
    #[serde(default)]
    pub tenzilat_yontemi: TenzilatYontemi,
    #[serde(default)]
    pub manuel_tenzilat_orani: f64,
    #[serde(default)]
    pub donusum_tarihi: String,
}

impl Default for SozlesmeAyarlari {
    fn default() -> Self {
        Self {
            kesif_bedeli: 0.0,
            sozlesme_bedeli: 0.0,
            tenzilat_yontemi: TenzilatYontemi::ManuelOran,
            manuel_tenzilat_orani: 0.0,
            donusum_tarihi: String::new(),
        }
    }
}

impl SozlesmeAyarlari {
    /// Oran hesabı altı ondalık hanede tutulur; parasal yuvarlama daha sonra yapılır.
    pub fn tenzilat_orani(&self) -> f64 {
        let oran = match self.tenzilat_yontemi {
            TenzilatYontemi::ManuelOran => self.manuel_tenzilat_orani,
            TenzilatYontemi::SozlesmeBedelinden if self.kesif_bedeli > 0.0 => {
                (1.0 - self.sozlesme_bedeli / self.kesif_bedeli) * 100.0
            }
            TenzilatYontemi::SozlesmeBedelinden => 0.0,
        };
        (oran * 1_000_000.0).round() / 1_000_000.0
    }

    pub fn hesaplanan_sozlesme_bedeli(&self) -> f64 {
        match self.tenzilat_yontemi {
            TenzilatYontemi::SozlesmeBedelinden => self.sozlesme_bedeli,
            TenzilatYontemi::ManuelOran => {
                self.kesif_bedeli * (1.0 - self.tenzilat_orani() / 100.0)
            }
        }
    }
}

/// Resmî çıktıların (yaklaşık maliyet cetveli, hakediş, teklif) başlığında yer alan
/// idari proje künyesi. Kamu ihale dokümanlarının standart üst bilgi alanları.
/// Tüm alanlar serde-default (eski `.mrj` dosyaları boş künye ile açılır).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjeBilgi {
    #[serde(default)]
    pub idare_adi: String, // İdarenin (işveren) adı
    #[serde(default)]
    pub is_adi: String, // İşin adı
    #[serde(default)]
    pub is_yeri: String, // İşin yeri (il/ilçe)
    #[serde(default)]
    pub ihale_kayit_no: String, // İhale Kayıt Numarası (İKN)
    #[serde(default)]
    pub is_turu: String, // Yapım / Hizmet / Mal
    #[serde(default)]
    pub yuklenici: String, // Yüklenici (hakediş/teklif aşamasında)
    #[serde(default)]
    pub sozlesme_no: String, // Sözleşme no
    #[serde(default)]
    pub sozlesme_tarihi: String, // Sözleşme tarihi
}

impl ProjeBilgi {
    /// Künyede en az bir alan doldurulmuş mu?
    pub fn dolu_mu(&self) -> bool {
        !(self.idare_adi.is_empty()
            && self.is_adi.is_empty()
            && self.is_yeri.is_empty()
            && self.ihale_kayit_no.is_empty()
            && self.is_turu.is_empty()
            && self.yuklenici.is_empty()
            && self.sozlesme_no.is_empty()
            && self.sozlesme_tarihi.is_empty())
    }
}

fn varsayilan_kar_orani() -> f64 {
    25.0
}
fn varsayilan_kdv_orani() -> f64 {
    20.0
}
fn varsayilan_hesap_turu() -> HesapTuru {
    HesapTuru::Ozel
}

impl KayitliMetraj {
    /// Eski `.mrj` dosyalarındaki kimliksiz kalemleri yükseltir ve hakediş
    /// satırlarını aynı pozun sıra bazlı eşleşmesiyle keşif kalemlerine bağlar.
    pub fn kimlikleri_tamamla(&mut self) {
        fn grup_kimliklerini_tamamla(gruplar: &mut [IsGrubu], gorulen: &mut HashSet<String>) {
            for grup in gruplar {
                for kalem in &mut grup.kalemler {
                    if kalem.id.is_empty() || !gorulen.insert(kalem.id.clone()) {
                        kalem.id = yeni_kalem_id();
                        gorulen.insert(kalem.id.clone());
                    }
                }
                grup_kimliklerini_tamamla(&mut grup.alt_gruplar, gorulen);
            }
        }

        let mut gorulen = HashSet::new();
        if self.is_gruplari.is_empty() {
            for kalem in &mut self.kalemler {
                if kalem.id.is_empty() || !gorulen.insert(kalem.id.clone()) {
                    kalem.id = yeni_kalem_id();
                    gorulen.insert(kalem.id.clone());
                }
            }
        } else {
            grup_kimliklerini_tamamla(&mut self.is_gruplari, &mut gorulen);
            self.kalemler = self
                .is_gruplari
                .iter()
                .flat_map(IsGrubu::tum_kalemler_duz)
                .collect();
        }

        let kesif: Vec<(String, String)> = self
            .kalemler
            .iter()
            .map(|k| (k.id.clone(), k.poz_no.clone()))
            .collect();
        for hakedis in &mut self.hakedisler {
            let mut kullanilan = HashSet::new();
            for satir in &mut hakedis.satirlar {
                if !satir.kalem_id.is_empty()
                    && kesif.iter().any(|(id, _)| id == &satir.kalem_id)
                    && kullanilan.insert(satir.kalem_id.clone())
                {
                    continue;
                }
                satir.kalem_id = kesif
                    .iter()
                    .find(|(id, poz)| poz == &satir.poz_no && !kullanilan.contains(id))
                    .map(|(id, _)| id.clone())
                    .unwrap_or_default();
                if !satir.kalem_id.is_empty() {
                    kullanilan.insert(satir.kalem_id.clone());
                }
            }
        }
    }

    pub fn toplam_tutar(&self) -> f64 {
        if !self.is_gruplari.is_empty() {
            self.is_gruplari.iter().map(|g| g.toplam_tutar()).sum()
        } else {
            self.kalemler.iter().map(|k| k.tutar).sum()
        }
    }
}

/// Pursantajlı iş programı: sözleşme süresini aylara böler ve her aya bir
/// ilerleme yüzdesi (pursantaj) atar. Toplam bedel × aylık % = o ayın imalat
/// tutarı; kümülatif toplam ise ilerleme (S) eğrisini verir. OSKA/AMP'de
/// "iş programı / pursantaj cetveli" olarak bilinen çıktının temelidir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsProgrami {
    pub baslangic_yil: u32,
    pub baslangic_ay: u32,
    pub sure_ay: u32,
    /// Her ayın yüzdesi (toplamı 100 olmalı). Boşsa eşit dağıtılır.
    #[serde(default)]
    pub dagilim: Vec<f64>,
}

impl Default for IsProgrami {
    fn default() -> Self {
        Self {
            baslangic_yil: 2026,
            baslangic_ay: 1,
            sure_ay: 6,
            dagilim: Vec::new(),
        }
    }
}

impl IsProgrami {
    /// Süreyi eşit yüzdelere böler (kullanıcı sonradan elle düzenleyebilir).
    pub fn esit_dagit(&mut self) {
        let n = self.sure_ay.max(1) as usize;
        self.dagilim = vec![100.0 / n as f64; n];
    }

    /// Dağılım uzunluğu süre ile uyumlu değilse eşit dağıtıma döner.
    pub fn normalize(&mut self) {
        let n = self.sure_ay.max(1) as usize;
        if self.dagilim.len() != n {
            self.esit_dagit();
        }
    }

    pub fn toplam_yuzde(&self) -> f64 {
        self.dagilim.iter().sum()
    }

    /// i. ayın (0 tabanlı) takvim yıl/ay etiketi.
    pub fn ay_etiketi(&self, i: usize) -> (u32, u32) {
        let toplam = (self.baslangic_ay.max(1) as usize - 1) + i;
        let yil = self.baslangic_yil + (toplam / 12) as u32;
        let ay = (toplam % 12) as u32 + 1;
        (yil, ay)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn kalem(poz: &str, tutar: f64) -> MetrajKalemi {
        MetrajKalemi {
            id: yeni_kalem_id(),
            poz_no: poz.into(),
            tanim: "test".into(),
            birim: "m3".into(),
            birim_fiyat: tutar,
            miktar: 1.0,
            tutar,
            kitap_adi: "K".into(),
            detaylar: vec![],
            imalat_cinsi: String::new(),
            kitap_id: 0,
        }
    }

    fn ornek_agac() -> Vec<IsGrubu> {
        vec![IsGrubu {
            id: "g1".into(),
            ad: "İnşaat".into(),
            kalemler: vec![kalem("A", 100.0)],
            alt_gruplar: vec![IsGrubu {
                id: "g2".into(),
                ad: "Kaba İnşaat".into(),
                kalemler: vec![kalem("B", 50.0), kalem("C", 25.0)],
                alt_gruplar: vec![],
            }],
        }]
    }

    #[test]
    fn grup_toplami_alt_gruplari_da_kapsar() {
        let agac = ornek_agac();
        assert_eq!(agac[0].toplam_tutar(), 175.0);
    }

    #[test]
    fn duzlestirme_tum_kalemleri_dondurur() {
        let agac = ornek_agac();
        let duz = agac[0].tum_kalemler_duz();
        assert_eq!(duz.len(), 3);
        let toplam: f64 = duz.iter().map(|k| k.tutar).sum();
        assert_eq!(toplam, 175.0);
    }

    #[test]
    fn gruplu_proje_toplami_gruplardan_hesaplanir() {
        let m = KayitliMetraj {
            ad: "T".into(),
            kalemler: vec![],
            is_gruplari: ornek_agac(),
            tarih: "2026-01-01".into(),
            genel_gider_kar_orani: 25.0,
            kdv_orani: 20.0,
            hesap_turu: HesapTuru::Kamu,
            hakedisler: vec![],
            is_programi: IsProgrami::default(),
            proje_bilgi: ProjeBilgi::default(),
            proje_asamasi: ProjeAsamasi::Metraj,
            sozlesme_ayarlari: SozlesmeAyarlari::default(),
        };
        assert_eq!(m.toplam_tutar(), 175.0);
    }

    #[test]
    fn is_programi_esit_dagitir_ve_takvim_hesaplar() {
        let mut p = IsProgrami {
            baslangic_yil: 2026,
            baslangic_ay: 11,
            sure_ay: 4,
            dagilim: vec![],
        };
        p.normalize(); // boş dağılımı eşitler
        assert_eq!(p.dagilim.len(), 4);
        assert!((p.toplam_yuzde() - 100.0).abs() < 1e-9);
        // Kasım 2026'dan başlayıp yıl atlar: Kas, Ara, Oca(2027), Şub(2027)
        assert_eq!(p.ay_etiketi(0), (2026, 11));
        assert_eq!(p.ay_etiketi(1), (2026, 12));
        assert_eq!(p.ay_etiketi(2), (2027, 1));
        assert_eq!(p.ay_etiketi(3), (2027, 2));
    }

    #[test]
    fn is_programi_sure_degisince_yeniden_esitlenir() {
        let mut p = IsProgrami::default();
        p.esit_dagit(); // 6 ay
        p.dagilim[0] = 90.0; // elle boz
        p.sure_ay = 3;
        p.normalize(); // uzunluk uyumsuz → eşitle
        assert_eq!(p.dagilim.len(), 3);
        assert!((p.dagilim[0] - 100.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn sozlesme_bedelinden_tenzilat_alti_hanede_hesaplanir() {
        let ayar = SozlesmeAyarlari {
            kesif_bedeli: 1_000_000.0,
            sozlesme_bedeli: 876_543.22,
            tenzilat_yontemi: TenzilatYontemi::SozlesmeBedelinden,
            ..SozlesmeAyarlari::default()
        };
        assert_eq!(ayar.tenzilat_orani(), 12.345678);
    }

    #[test]
    fn eski_flat_json_is_gruplari_olmadan_okunur() {
        // is_gruplari alanı olmayan eski proje dosyası serde(default) ile açılabilmeli
        let eski = r#"{"ad":"Eski","kalemler":[{"poz_no":"A","tanim":"x","birim":"m","birim_fiyat":10.0,"miktar":2.0,"tutar":20.0,"kitap_adi":"K","detaylar":[]}],"tarih":"2025-01-01"}"#;
        let m: KayitliMetraj = serde_json::from_str(eski).expect("eski format okunmalı");
        assert!(m.is_gruplari.is_empty());
        assert_eq!(m.kalemler.len(), 1);
        assert_eq!(m.toplam_tutar(), 20.0);
        // Oranlar varsayılana düşmeli
        assert_eq!(m.genel_gider_kar_orani, 25.0);
        assert_eq!(m.kdv_orani, 20.0);
        // Eski dosya hesap türü alanı içermez → ÖZEL'e düşmeli (KDV dahil davranışı korunur)
        assert_eq!(m.hesap_turu, HesapTuru::Ozel);
    }

    #[test]
    fn eski_projede_ayni_pozlu_kalemler_ve_hakedisler_ayri_kimlik_alir() {
        let mut a = kalem("P", 100.0);
        let mut b = kalem("P", 200.0);
        a.id.clear();
        b.id.clear();
        let mut h = Hakedis::yeni(1, "İlk", "2026-01-01".into());
        h.satirlar = vec![
            HakedisSatiri {
                kalem_id: String::new(),
                poz_no: "P".into(),
                kumulatif_miktar: 1.0,
                detaylar: vec![],
            },
            HakedisSatiri {
                kalem_id: String::new(),
                poz_no: "P".into(),
                kumulatif_miktar: 2.0,
                detaylar: vec![],
            },
        ];
        let mut proje = KayitliMetraj {
            ad: "Eski".into(),
            kalemler: vec![a, b],
            is_gruplari: vec![],
            tarih: "2026-01-01".into(),
            genel_gider_kar_orani: 0.0,
            kdv_orani: 20.0,
            hesap_turu: HesapTuru::Kamu,
            hakedisler: vec![h],
            is_programi: IsProgrami::default(),
            proje_bilgi: ProjeBilgi::default(),
            proje_asamasi: ProjeAsamasi::Metraj,
            sozlesme_ayarlari: SozlesmeAyarlari::default(),
        };

        proje.kimlikleri_tamamla();
        assert!(!proje.kalemler[0].id.is_empty());
        assert_ne!(proje.kalemler[0].id, proje.kalemler[1].id);
        assert_eq!(
            proje.hakedisler[0].satirlar[0].kalem_id,
            proje.kalemler[0].id
        );
        assert_eq!(
            proje.hakedisler[0].satirlar[1].kalem_id,
            proje.kalemler[1].id
        );
    }

    #[test]
    fn boyutlardan_miktar_carpilir() {
        let d = MiktarDetay {
            aciklama: "kiriş".into(),
            miktar: 0.0,
            adet: Some(4.0),
            en: Some(0.30),
            boy: Some(5.0),
            yukseklik: Some(0.40),
            cikan: false,
        };
        assert!((d.hesaplanan_miktar() - 2.4).abs() < 1e-9); // 4 * 0.30 * 5 * 0.40
        assert!(d.boyutlu_mu());
    }

    #[test]
    fn eksik_boyutlar_bir_sayilir() {
        // sadece adet ve boy verilmiş; en/yükseklik 1 sayılır
        let d = MiktarDetay {
            aciklama: "".into(),
            miktar: 0.0,
            adet: Some(3.0),
            en: None,
            boy: Some(2.5),
            yukseklik: None,
            cikan: false,
        };
        assert!((d.hesaplanan_miktar() - 7.5).abs() < 1e-9);
    }

    #[test]
    fn boyutsuz_detay_elle_girilen_miktari_korur() {
        let d = MiktarDetay {
            aciklama: "hazır".into(),
            miktar: 12.0,
            adet: None,
            en: None,
            boy: None,
            yukseklik: None,
            cikan: false,
        };
        assert_eq!(d.hesaplanan_miktar(), 12.0);
        assert!(!d.boyutlu_mu());
    }

    #[test]
    fn cikan_detay_metrajdan_dusulur() {
        let mut k = MetrajKalemi {
            id: yeni_kalem_id(),
            poz_no: "P".into(),
            tanim: "duvar".into(),
            birim: "m2".into(),
            birim_fiyat: 10.0,
            miktar: 0.0,
            tutar: 0.0,
            kitap_adi: "K".into(),
            detaylar: vec![
                // Brüt duvar: 10 x 3 = 30 m²
                MiktarDetay {
                    aciklama: "duvar".into(),
                    miktar: 0.0,
                    adet: Some(1.0),
                    en: Some(10.0),
                    boy: Some(3.0),
                    yukseklik: None,
                    cikan: false,
                },
                // 2 pencere boşluğu: 2 x 1.5 x 1.0 = 3 m² düşülür
                MiktarDetay {
                    aciklama: "pencere".into(),
                    miktar: 0.0,
                    adet: Some(2.0),
                    en: Some(1.5),
                    boy: Some(1.0),
                    yukseklik: None,
                    cikan: true,
                },
            ],
            imalat_cinsi: String::new(),
            kitap_id: 0,
        };
        k.detaylardan_miktar_hesapla();
        assert_eq!(k.miktar, 27.0); // 30 - 3
        assert_eq!(k.tutar, 270.0);
    }

    #[test]
    fn analiz_birim_fiyati_kar_uygular() {
        let g = vec![
            AnalizGirdisi {
                girdi_no: "10.100.1001".into(),
                tanim: "Düz işçi".into(),
                birim: "saat".into(),
                birim_fiyat: 100.0,
                miktar: 2.0,
                tur: "İşçilik".into(),
            },
            AnalizGirdisi {
                girdi_no: "10.130.1001".into(),
                tanim: "Çimento".into(),
                birim: "kg".into(),
                birim_fiyat: 5.0,
                miktar: 50.0,
                tur: "Malzeme".into(),
            },
        ];
        assert_eq!(analiz_ara_toplam(&g), 450.0); // 200 + 250
        assert_eq!(analiz_birim_fiyat(&g, 25.0), 562.5); // 450 * 1.25
    }

    #[test]
    fn kalem_detaylardan_toplam_miktar_ve_tutar() {
        let mut k = MetrajKalemi {
            id: yeni_kalem_id(),
            poz_no: "P".into(),
            tanim: "t".into(),
            birim: "m3".into(),
            birim_fiyat: 100.0,
            miktar: 0.0,
            tutar: 0.0,
            kitap_adi: "K".into(),
            detaylar: vec![
                MiktarDetay {
                    aciklama: "a".into(),
                    miktar: 0.0,
                    adet: Some(2.0),
                    en: None,
                    boy: None,
                    yukseklik: None,
                    cikan: false,
                },
                MiktarDetay {
                    aciklama: "b".into(),
                    miktar: 0.0,
                    adet: Some(1.0),
                    en: Some(3.0),
                    boy: Some(0.5),
                    yukseklik: None,
                    cikan: false,
                },
            ],
            imalat_cinsi: String::new(),
            kitap_id: 0,
        };
        k.detaylardan_miktar_hesapla();
        assert_eq!(k.miktar, 3.5); // 2 + 1.5
        assert_eq!(k.tutar, 350.0);
    }
}
