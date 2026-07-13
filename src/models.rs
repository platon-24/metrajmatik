use serde::{Deserialize, Serialize};

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
        let buyukluk = if self.adet.is_none() && self.en.is_none() && self.boy.is_none() && self.yukseklik.is_none() {
            self.miktar
        } else {
            self.adet.unwrap_or(1.0)
                * self.en.unwrap_or(1.0)
                * self.boy.unwrap_or(1.0)
                * self.yukseklik.unwrap_or(1.0)
        };
        if self.cikan { -buyukluk.abs() } else { buyukluk }
    }

    /// Boyut tabanlı mı yoksa elle girilmiş mi?
    pub fn boyutlu_mu(&self) -> bool {
        self.adet.is_some() || self.en.is_some() || self.boy.is_some() || self.yukseklik.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetrajKalemi {
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
    pub girdi_no: String,   // rayiç / girdi poz numarası
    pub tanim: String,
    pub birim: String,
    pub birim_fiyat: f64,   // girdinin birim fiyatı (rayiçten alınan değer)
    pub miktar: f64,        // katsayı: 1 birim imalat için gereken girdi miktarı
    pub tur: String,        // "İşçilik" | "Malzeme" | "Makine"
}

impl AnalizGirdisi {
    pub fn tutar(&self) -> f64 { self.miktar * self.birim_fiyat }
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
        let id = format!("g_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HesapTuru { Kamu, Ozel }

impl Default for HesapTuru {
    fn default() -> Self { HesapTuru::Kamu }
}

impl HesapTuru {
    pub fn kamu_mu(self) -> bool { matches!(self, HesapTuru::Kamu) }
}

/// Bir hakedişte tek bir iş kaleminin yeşil defter (kümülatif yapılan) miktarı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HakedisSatiri {
    pub poz_no: String,
    pub kumulatif_miktar: f64, // bu hakedişe kadar YAPILAN toplam (yeşil defter)
}

/// Bir hakediş (progress payment): numarası, dönemi, kümülatif imalat miktarları ve
/// kesinti oranları. Sözleşme (keşif) kalemleri projeden gelir; hakediş yalnızca o
/// pozların kümülatif yapılan miktarını + kesintileri tutar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hakedis {
    pub no: u32,
    pub tarih: String,
    pub tur: String, // "İlk" | "Ara" | "Kesin"
    #[serde(default)]
    pub satirlar: Vec<HakedisSatiri>,
    #[serde(default = "hakedis_damga_orani")]
    pub damga_orani: f64, // binde (‰) — damga vergisi (tahakkuk üzerinden)
    #[serde(default)]
    pub teminat_orani: f64, // % — kesin teminat kesintisi
    #[serde(default)]
    pub sgk_orani: f64, // % — SGK kesintisi (varsa)
    #[serde(default)]
    pub avans_mahsup: f64, // TL — bu hakedişte mahsup edilecek avans
    #[serde(default)]
    pub fiyat_farki: f64, // TL — +/- fiyat farkı (Yİ-ÜFE; şimdilik elle)
}

impl Hakedis {
    /// Verilen pozun bu hakedişteki kümülatif yapılan miktarı (yoksa 0).
    pub fn kumulatif(&self, poz_no: &str) -> f64 {
        self.satirlar.iter().find(|s| s.poz_no == poz_no).map(|s| s.kumulatif_miktar).unwrap_or(0.0)
    }
}

fn hakedis_damga_orani() -> f64 { 9.48 } // binde 9.48 (hakediş damga vergisi)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KayitliMetraj {
    pub ad: String,
    #[serde(default)]
    pub kalemler: Vec<MetrajKalemi>, // Eski flat projeler için
    #[serde(default)]
    pub is_gruplari: Vec<IsGrubu>,   // Yeni hiyerarşik yapı için
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
}

fn varsayilan_kar_orani() -> f64 { 25.0 }
fn varsayilan_kdv_orani() -> f64 { 20.0 }
fn varsayilan_hesap_turu() -> HesapTuru { HesapTuru::Ozel }

impl KayitliMetraj {
    pub fn toplam_tutar(&self) -> f64 {
        if !self.is_gruplari.is_empty() {
            self.is_gruplari.iter().map(|g| g.toplam_tutar()).sum()
        } else {
            self.kalemler.iter().map(|k| k.tutar).sum()
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn kalem(poz: &str, tutar: f64) -> MetrajKalemi {
        MetrajKalemi {
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
        let m = KayitliMetraj { ad: "T".into(), kalemler: vec![], is_gruplari: ornek_agac(), tarih: "2026-01-01".into(), genel_gider_kar_orani: 25.0, kdv_orani: 20.0, hesap_turu: HesapTuru::Kamu, hakedisler: vec![] };
        assert_eq!(m.toplam_tutar(), 175.0);
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
    fn boyutlardan_miktar_carpilir() {
        let d = MiktarDetay { aciklama: "kiriş".into(), miktar: 0.0, adet: Some(4.0), en: Some(0.30), boy: Some(5.0), yukseklik: Some(0.40), cikan: false };
        assert!((d.hesaplanan_miktar() - 2.4).abs() < 1e-9); // 4 * 0.30 * 5 * 0.40
        assert!(d.boyutlu_mu());
    }

    #[test]
    fn eksik_boyutlar_bir_sayilir() {
        // sadece adet ve boy verilmiş; en/yükseklik 1 sayılır
        let d = MiktarDetay { aciklama: "".into(), miktar: 0.0, adet: Some(3.0), en: None, boy: Some(2.5), yukseklik: None, cikan: false };
        assert!((d.hesaplanan_miktar() - 7.5).abs() < 1e-9);
    }

    #[test]
    fn boyutsuz_detay_elle_girilen_miktari_korur() {
        let d = MiktarDetay { aciklama: "hazır".into(), miktar: 12.0, adet: None, en: None, boy: None, yukseklik: None, cikan: false };
        assert_eq!(d.hesaplanan_miktar(), 12.0);
        assert!(!d.boyutlu_mu());
    }

    #[test]
    fn cikan_detay_metrajdan_dusulur() {
        let mut k = MetrajKalemi {
            poz_no: "P".into(), tanim: "duvar".into(), birim: "m2".into(),
            birim_fiyat: 10.0, miktar: 0.0, tutar: 0.0, kitap_adi: "K".into(),
            detaylar: vec![
                // Brüt duvar: 10 x 3 = 30 m²
                MiktarDetay { aciklama: "duvar".into(), miktar: 0.0, adet: Some(1.0), en: Some(10.0), boy: Some(3.0), yukseklik: None, cikan: false },
                // 2 pencere boşluğu: 2 x 1.5 x 1.0 = 3 m² düşülür
                MiktarDetay { aciklama: "pencere".into(), miktar: 0.0, adet: Some(2.0), en: Some(1.5), boy: Some(1.0), yukseklik: None, cikan: true },
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
            AnalizGirdisi { girdi_no: "10.100.1001".into(), tanim: "Düz işçi".into(), birim: "saat".into(), birim_fiyat: 100.0, miktar: 2.0, tur: "İşçilik".into() },
            AnalizGirdisi { girdi_no: "10.130.1001".into(), tanim: "Çimento".into(), birim: "kg".into(), birim_fiyat: 5.0, miktar: 50.0, tur: "Malzeme".into() },
        ];
        assert_eq!(analiz_ara_toplam(&g), 450.0); // 200 + 250
        assert_eq!(analiz_birim_fiyat(&g, 25.0), 562.5); // 450 * 1.25
    }

    #[test]
    fn kalem_detaylardan_toplam_miktar_ve_tutar() {
        let mut k = MetrajKalemi {
            poz_no: "P".into(), tanim: "t".into(), birim: "m3".into(),
            birim_fiyat: 100.0, miktar: 0.0, tutar: 0.0, kitap_adi: "K".into(),
            detaylar: vec![
                MiktarDetay { aciklama: "a".into(), miktar: 0.0, adet: Some(2.0), en: None, boy: None, yukseklik: None, cikan: false },
                MiktarDetay { aciklama: "b".into(), miktar: 0.0, adet: Some(1.0), en: Some(3.0), boy: Some(0.5), yukseklik: None, cikan: false },
            ],
            imalat_cinsi: String::new(),
            kitap_id: 0,
        };
        k.detaylardan_miktar_hesapla();
        assert_eq!(k.miktar, 3.5); // 2 + 1.5
        assert_eq!(k.tutar, 350.0);
    }
}