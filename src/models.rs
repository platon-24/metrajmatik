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
pub struct MetrajKalemi {
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub birim_fiyat: f64,
    pub miktar: f64,
    pub tutar: f64,
    pub kitap_adi: String,
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
        }
    }

    pub fn tutar_guncelle(&mut self) {
        self.tutar = self.birim_fiyat * self.miktar;
    }
}

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