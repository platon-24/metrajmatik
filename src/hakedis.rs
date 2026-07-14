//! Hakediş (progress payment) hesabı. Sözleşme (keşif) kalemleri + bir hakedişin
//! kümülatif (yeşil defter) miktarları + önceki hakediş → bu hakediş tutarı,
//! kesintiler ve net ödeme. Yuvarlama tek yerden: [`crate::bicim::kurus_yuvarla`].

use crate::bicim::kurus_yuvarla;
use crate::models::{Hakedis, MetrajKalemi};

/// Bir iş kaleminin bir hakedişteki satır hesabı.
pub struct HakedisPozHesap {
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub birim_fiyat: f64,
    pub sozlesme_miktar: f64,
    pub onceki_kumulatif: f64,
    pub kumulatif: f64,
    pub bu_hakedis_miktar: f64,
    pub bu_hakedis_tutar: f64,
    pub kumulatif_tutar: f64,
}

/// Hakediş icmali: brütler, fiyat farkı, kesintiler ve net ödeme.
pub struct HakedisIcmal {
    pub kumulatif_brut: f64,
    pub onceki_brut: f64,
    pub bu_hakedis_brut: f64,
    pub fiyat_farki: f64,
    pub tahakkuk: f64, // bu hakediş brüt + fiyat farkı
    pub damga: f64,
    pub teminat: f64,
    pub sgk: f64,
    pub avans_mahsup: f64,
    pub kesinti_toplam: f64,
    pub net_odeme: f64,
    pub kdv: f64,      // tahakkuk × KDV oranı (bilgi)
    pub tevkifat: f64, // KDV × tevkifat oranı (bilgi)
}

/// Keşif kalemleri + hakediş + önceki hakediş → poz bazında satır hesapları.
pub fn poz_hesaplari(kesif: &[MetrajKalemi], hakedis: &Hakedis, onceki: Option<&Hakedis>) -> Vec<HakedisPozHesap> {
    kesif.iter().map(|k| {
        let kumulatif = hakedis.kumulatif(&k.poz_no);
        let onceki_kumulatif = onceki.map(|h| h.kumulatif(&k.poz_no)).unwrap_or(0.0);
        let bu_miktar = kumulatif - onceki_kumulatif;
        HakedisPozHesap {
            poz_no: k.poz_no.clone(),
            tanim: k.tanim.clone(),
            birim: k.birim.clone(),
            birim_fiyat: k.birim_fiyat,
            sozlesme_miktar: k.miktar,
            onceki_kumulatif,
            kumulatif,
            bu_hakedis_miktar: bu_miktar,
            bu_hakedis_tutar: kurus_yuvarla(k.birim_fiyat * bu_miktar),
            kumulatif_tutar: kurus_yuvarla(k.birim_fiyat * kumulatif),
        }
    }).collect()
}

/// Poz hesaplarından + hakediş oranlarından icmal (kesintiler + net ödeme).
pub fn icmal(hesaplar: &[HakedisPozHesap], hakedis: &Hakedis) -> HakedisIcmal {
    let kumulatif_brut = kurus_yuvarla(hesaplar.iter().map(|h| h.kumulatif_tutar).sum());
    let bu_hakedis_brut = kurus_yuvarla(hesaplar.iter().map(|h| h.bu_hakedis_tutar).sum());
    let onceki_brut = kurus_yuvarla(kumulatif_brut - bu_hakedis_brut);
    // Fiyat farkı: Yİ-ÜFE otomatik (F = An × B × (güncel/temel − 1)) ya da elle.
    let fiyat_farki = if hakedis.ff_uygula && hakedis.ff_temel_endeks > 0.0 {
        kurus_yuvarla(bu_hakedis_brut * hakedis.ff_b * (hakedis.ff_guncel_endeks / hakedis.ff_temel_endeks - 1.0))
    } else {
        hakedis.fiyat_farki
    };
    let tahakkuk = kurus_yuvarla(bu_hakedis_brut + fiyat_farki);
    let damga = kurus_yuvarla(tahakkuk * hakedis.damga_orani / 1000.0);
    let teminat = kurus_yuvarla(bu_hakedis_brut * hakedis.teminat_orani / 100.0);
    let sgk = kurus_yuvarla(bu_hakedis_brut * hakedis.sgk_orani / 100.0);
    let avans_mahsup = kurus_yuvarla(hakedis.avans_mahsup);
    let kesinti_toplam = kurus_yuvarla(damga + teminat + sgk + avans_mahsup);
    let net_odeme = kurus_yuvarla(tahakkuk - kesinti_toplam);
    let kdv = kurus_yuvarla(tahakkuk * hakedis.kdv_orani / 100.0);
    let tevkifat = kurus_yuvarla(kdv * hakedis.tevkifat_orani);
    HakedisIcmal {
        kumulatif_brut, onceki_brut, bu_hakedis_brut,
        fiyat_farki, tahakkuk,
        damga, teminat, sgk, avans_mahsup, kesinti_toplam, net_odeme, kdv, tevkifat,
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::models::{Hakedis, HakedisSatiri, MetrajKalemi};

    fn kalem(poz: &str, bf: f64, sozlesme: f64) -> MetrajKalemi {
        MetrajKalemi {
            poz_no: poz.into(), tanim: "t".into(), birim: "m3".into(),
            birim_fiyat: bf, miktar: sozlesme, tutar: bf * sozlesme, kitap_adi: "K".into(),
            detaylar: vec![], imalat_cinsi: String::new(), kitap_id: 0,
        }
    }
    fn hakedis(no: u32, kumler: &[(&str, f64)]) -> Hakedis {
        let mut h = Hakedis::yeni(no, "Ara", "2026-01-01".into());
        h.satirlar = kumler.iter().map(|(p, m)| HakedisSatiri { poz_no: p.to_string(), kumulatif_miktar: *m, detaylar: vec![] }).collect();
        h
    }

    #[test]
    fn bu_donem_miktar_tutar_ve_onceki_brut() {
        let kesif = vec![kalem("A", 100.0, 10.0), kalem("B", 50.0, 20.0)];
        let h1 = hakedis(1, &[("A", 4.0), ("B", 5.0)]);
        let h2 = hakedis(2, &[("A", 7.0), ("B", 12.0)]);
        let hesaplar = poz_hesaplari(&kesif, &h2, Some(&h1));
        assert_eq!(hesaplar[0].bu_hakedis_miktar, 3.0); // 7-4
        assert_eq!(hesaplar[0].bu_hakedis_tutar, 300.0);
        assert_eq!(hesaplar[1].bu_hakedis_tutar, 350.0); // 50×(12-5)
        let ic = icmal(&hesaplar, &h2);
        assert_eq!(ic.bu_hakedis_brut, 650.0);
        assert_eq!(ic.kumulatif_brut, 1300.0); // 100×7 + 50×12
        assert_eq!(ic.onceki_brut, 650.0);
    }

    #[test]
    fn kesintiler_ve_net_odeme() {
        let kesif = vec![kalem("A", 1000.0, 100.0)];
        let mut h = hakedis(1, &[("A", 10.0)]); // bu hakediş brüt = 10.000
        h.teminat_orani = 5.0;
        h.avans_mahsup = 1000.0;
        let hesaplar = poz_hesaplari(&kesif, &h, None);
        let ic = icmal(&hesaplar, &h);
        assert_eq!(ic.bu_hakedis_brut, 10000.0);
        assert_eq!(ic.damga, 94.80); // 10000 × 9.48/1000
        assert_eq!(ic.teminat, 500.0);
        assert_eq!(ic.kesinti_toplam, 1594.80); // 94.80 + 500 + 1000
        assert_eq!(ic.net_odeme, 8405.20); // 10000 - 1594.80
    }

    #[test]
    fn fiyat_farki_yi_ufe_otomatik() {
        let kesif = vec![kalem("A", 1000.0, 100.0)];
        let mut h = hakedis(1, &[("A", 10.0)]); // bu hakediş brüt = 10.000
        h.ff_uygula = true;
        h.ff_b = 0.90;
        h.ff_temel_endeks = 100.0;
        h.ff_guncel_endeks = 120.0;
        // F = 10000 × 0.90 × (120/100 − 1) = 1800
        let ic = icmal(&poz_hesaplari(&kesif, &h, None), &h);
        assert_eq!(ic.fiyat_farki, 1800.0);
        assert_eq!(ic.tahakkuk, 11800.0);
    }
}
