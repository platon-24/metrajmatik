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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kitap {
    pub id: i64,
    pub ad: String,
    pub yil: u32,
    pub ay: u32,
    pub poz_sayisi: u32,
    pub tarih: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiktarDetay {
    pub aciklama: String,
    pub miktar: f64,
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
        }
    }

    pub fn detaylardan_miktar_hesapla(&mut self) {
        self.miktar = self.detaylar.iter().map(|d| d.miktar).sum();
        self.tutar_guncelle();
    }

    pub fn tutar_guncelle(&mut self) {
        self.tutar = self.birim_fiyat * self.miktar;
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KayitliMetraj {
    pub ad: String,
    #[serde(default)]
    pub kalemler: Vec<MetrajKalemi>, // Eski flat projeler için
    #[serde(default)]
    pub is_gruplari: Vec<IsGrubu>,   // Yeni hiyerarşik yapı için
    pub tarih: String,
}

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
        let m = KayitliMetraj { ad: "T".into(), kalemler: vec![], is_gruplari: ornek_agac(), tarih: "2026-01-01".into() };
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
    }
}