use serde::{Deserialize, Serialize};

/// Bir birim fiyat pozunu temsil eder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poz {
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub fiyat: Option<f64>, // None ise formül pozudur
    pub kategori: String,
}

/// Metraj tablosundaki bir kalemi temsil eder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetrajKalemi {
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub birim_fiyat: f64,
    pub miktar: f64,
    pub tutar: f64,
}

impl MetrajKalemi {
    pub fn yeni(poz: &Poz, miktar: f64) -> Option<Self> {
        poz.fiyat.map(|bf| {
            let tutar = bf * miktar;
            MetrajKalemi {
                poz_no: poz.poz_no.clone(),
                tanim: poz.tanim.clone(),
                birim: poz.birim.clone(),
                birim_fiyat: bf,
                miktar,
                tutar,
            }
        })
    }

    pub fn tutar_guncelle(&mut self) {
        self.tutar = self.birim_fiyat * self.miktar;
    }
}

/// Kayıtlı metraj dosyası
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KayitliMetraj {
    pub ad: String,
    pub kalemler: Vec<MetrajKalemi>,
    pub tarih: String,
}

impl KayitliMetraj {
    pub fn toplam_tutar(&self) -> f64 {
        self.kalemler.iter().map(|k| k.tutar).sum()
    }
}