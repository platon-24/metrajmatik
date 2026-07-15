//! Hakediş (progress payment) hesabı. Sözleşme (keşif) kalemleri + bir hakedişin
//! kümülatif (yeşil defter) miktarları + önceki hakediş → bu hakediş tutarı,
//! kesintiler ve net ödeme. Yuvarlama tek yerden: [`crate::bicim::kurus_yuvarla`].

use crate::bicim::kurus_yuvarla;
use crate::models::{FiyatFarkiYontemi, Hakedis, MetrajKalemi};

/// Bir iş kaleminin bir hakedişteki satır hesabı.
pub struct HakedisPozHesap {
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub birim_fiyat: f64,
    pub kesif_birim_fiyat: f64,
    pub sozlesme_miktar: f64,
    pub onceki_kumulatif: f64,
    pub kumulatif: f64,
    pub bu_hakedis_miktar: f64,
    pub bu_hakedis_tutar: f64,
    pub kumulatif_tutar: f64,
}

/// Hakediş icmali: brütler, fiyat farkı, kesintiler ve net ödeme.
pub struct HakedisIcmal {
    pub bu_hakedis_ham: f64,
    pub tenzilat_tutari: f64,
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
    pub net_odeme: f64,      // KDV hariç tahakkuk − kesintiler
    pub kdv: f64,            // tahakkuk × KDV oranı (bilgi)
    pub tevkifat: f64,       // KDV × tevkifat oranı (bilgi)
    pub odenecek_tutar: f64, // KDV hariç net + KDV − KDV tevkifatı
}

/// Keşif kalemleri + hakediş + önceki hakediş → poz bazında satır hesapları.
pub fn poz_hesaplari(
    kesif: &[MetrajKalemi],
    hakedis: &Hakedis,
    onceki: Option<&Hakedis>,
    tenzilat_orani: f64,
) -> Vec<HakedisPozHesap> {
    let carpani = 1.0 - tenzilat_orani / 100.0;
    kesif
        .iter()
        .map(|k| {
            let kumulatif = hakedis.kumulatif(&k.id, &k.poz_no);
            let onceki_kumulatif = onceki.map(|h| h.kumulatif(&k.id, &k.poz_no)).unwrap_or(0.0);
            let bu_miktar = kumulatif - onceki_kumulatif;
            let sozlesme_birim_fiyati = k.birim_fiyat * carpani;
            HakedisPozHesap {
                poz_no: k.poz_no.clone(),
                tanim: k.tanim.clone(),
                birim: k.birim.clone(),
                birim_fiyat: sozlesme_birim_fiyati,
                kesif_birim_fiyat: k.birim_fiyat,
                sozlesme_miktar: k.miktar,
                onceki_kumulatif,
                kumulatif,
                bu_hakedis_miktar: bu_miktar,
                bu_hakedis_tutar: kurus_yuvarla(sozlesme_birim_fiyati * bu_miktar),
                kumulatif_tutar: kurus_yuvarla(sozlesme_birim_fiyati * kumulatif),
            }
        })
        .collect()
}

/// Poz hesaplarından + hakediş oranlarından icmal (kesintiler + net ödeme).
pub fn icmal(hesaplar: &[HakedisPozHesap], hakedis: &Hakedis) -> HakedisIcmal {
    let kumulatif_brut = kurus_yuvarla(hesaplar.iter().map(|h| h.kumulatif_tutar).sum());
    let bu_hakedis_brut = kurus_yuvarla(hesaplar.iter().map(|h| h.bu_hakedis_tutar).sum());
    let onceki_brut = kurus_yuvarla(kumulatif_brut - bu_hakedis_brut);
    let bu_hakedis_ham = kurus_yuvarla(
        hesaplar
            .iter()
            .map(|h| h.kesif_birim_fiyat * h.bu_hakedis_miktar)
            .sum(),
    );
    let tenzilat_tutari = kurus_yuvarla(bu_hakedis_ham - bu_hakedis_brut);
    let fiyat_farki = fiyat_farki_hesapla(bu_hakedis_brut, hakedis);
    let tahakkuk = kurus_yuvarla(bu_hakedis_brut + fiyat_farki);
    let damga = kurus_yuvarla(tahakkuk * hakedis.damga_orani / 1000.0);
    let teminat = kurus_yuvarla(bu_hakedis_brut * hakedis.teminat_orani / 100.0);
    let sgk = kurus_yuvarla(bu_hakedis_brut * hakedis.sgk_orani / 100.0);
    let avans_mahsup = kurus_yuvarla(hakedis.avans_mahsup);
    let kesinti_toplam = kurus_yuvarla(damga + teminat + sgk + avans_mahsup);
    let net_odeme = kurus_yuvarla(tahakkuk - kesinti_toplam);
    let kdv = kurus_yuvarla(tahakkuk * hakedis.kdv_orani / 100.0);
    let tevkifat = kurus_yuvarla(kdv * hakedis.tevkifat_orani);
    let odenecek_tutar = kurus_yuvarla(net_odeme + kdv - tevkifat);
    HakedisIcmal {
        bu_hakedis_ham,
        tenzilat_tutari,
        kumulatif_brut,
        onceki_brut,
        bu_hakedis_brut,
        fiyat_farki,
        tahakkuk,
        damga,
        teminat,
        sgk,
        avans_mahsup,
        kesinti_toplam,
        net_odeme,
        kdv,
        tevkifat,
        odenecek_tutar,
    }
}

/// F = An × B × (Pn − 1). Ağırlıklı yöntemde Pn, sözleşmedeki yapım işi
/// bileşenlerinin katsayı × (güncel/temel) toplamıdır.
fn fiyat_farki_hesapla(an: f64, hakedis: &Hakedis) -> f64 {
    let ayar = &hakedis.fiyat_farki_ayari;
    match ayar.yontem {
        FiyatFarkiYontemi::Manuel => kurus_yuvarla(hakedis.fiyat_farki),
        FiyatFarkiYontemi::TekEndeks => {
            let Some(b) = ayar.bilesenler.first() else {
                return 0.0;
            };
            if b.temel_endeks <= 0.0 || b.guncel_endeks <= 0.0 {
                return 0.0;
            }
            kurus_yuvarla(an * ayar.b * (b.guncel_endeks / b.temel_endeks - 1.0))
        }
        FiyatFarkiYontemi::YapimAgirlikli => {
            let katsayi_toplami: f64 = ayar.bilesenler.iter().map(|b| b.katsayi).sum();
            if (katsayi_toplami - 1.0).abs() > 0.000001 {
                return 0.0;
            }
            if ayar
                .bilesenler
                .iter()
                .any(|b| b.katsayi > 0.0 && (b.temel_endeks <= 0.0 || b.guncel_endeks <= 0.0))
            {
                return 0.0;
            }
            let pn: f64 = ayar
                .bilesenler
                .iter()
                .filter(|b| b.katsayi != 0.0)
                .map(|b| b.katsayi * b.guncel_endeks / b.temel_endeks)
                .sum();
            kurus_yuvarla(an * ayar.b * (pn - 1.0))
        }
        FiyatFarkiYontemi::Yok if hakedis.ff_uygula && hakedis.ff_temel_endeks > 0.0 => {
            // Eski proje dosyalarının tek-endeks hesabını aynen aç.
            kurus_yuvarla(
                an * hakedis.ff_b * (hakedis.ff_guncel_endeks / hakedis.ff_temel_endeks - 1.0),
            )
        }
        FiyatFarkiYontemi::Yok if hakedis.fiyat_farki != 0.0 => kurus_yuvarla(hakedis.fiyat_farki),
        FiyatFarkiYontemi::Yok => 0.0,
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::models::{Hakedis, HakedisSatiri, MetrajKalemi};

    fn kalem(poz: &str, bf: f64, sozlesme: f64) -> MetrajKalemi {
        MetrajKalemi {
            id: crate::models::yeni_kalem_id(),
            poz_no: poz.into(),
            tanim: "t".into(),
            birim: "m3".into(),
            birim_fiyat: bf,
            miktar: sozlesme,
            tutar: bf * sozlesme,
            kitap_adi: "K".into(),
            detaylar: vec![],
            imalat_cinsi: String::new(),
            kitap_id: 0,
        }
    }
    fn hakedis(no: u32, kumler: &[(&str, f64)]) -> Hakedis {
        let mut h = Hakedis::yeni(no, "Ara", "2026-01-01".into());
        h.satirlar = kumler
            .iter()
            .map(|(p, m)| HakedisSatiri {
                kalem_id: String::new(),
                poz_no: p.to_string(),
                kumulatif_miktar: *m,
                detaylar: vec![],
            })
            .collect();
        h
    }

    #[test]
    fn bu_donem_miktar_tutar_ve_onceki_brut() {
        let kesif = vec![kalem("A", 100.0, 10.0), kalem("B", 50.0, 20.0)];
        let h1 = hakedis(1, &[("A", 4.0), ("B", 5.0)]);
        let h2 = hakedis(2, &[("A", 7.0), ("B", 12.0)]);
        let hesaplar = poz_hesaplari(&kesif, &h2, Some(&h1), 0.0);
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
        let hesaplar = poz_hesaplari(&kesif, &h, None, 0.0);
        let ic = icmal(&hesaplar, &h);
        assert_eq!(ic.bu_hakedis_brut, 10000.0);
        assert_eq!(ic.damga, 94.80); // 10000 × 9.48/1000
        assert_eq!(ic.teminat, 500.0);
        assert_eq!(ic.kesinti_toplam, 1594.80); // 94.80 + 500 + 1000
        assert_eq!(ic.net_odeme, 8405.20); // 10000 - 1594.80
        assert_eq!(ic.odenecek_tutar, 10405.20); // KDV hariç net + %20 KDV
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
        let ic = icmal(&poz_hesaplari(&kesif, &h, None, 0.0), &h);
        assert_eq!(ic.fiyat_farki, 1800.0);
        assert_eq!(ic.tahakkuk, 11800.0);
    }

    #[test]
    fn ayni_pozlu_farkli_kalemler_kimlikle_ayri_hesaplanir() {
        let mut a = kalem("15.100.1001", 100.0, 10.0);
        let mut b = kalem("15.100.1001", 250.0, 20.0);
        a.id = "kalem-a".into();
        b.id = "kalem-b".into();
        let kesif = vec![a, b];
        let mut h = Hakedis::yeni(1, "İlk", "2026-01-01".into());
        h.satirlar = vec![
            HakedisSatiri {
                kalem_id: "kalem-a".into(),
                poz_no: "15.100.1001".into(),
                kumulatif_miktar: 2.0,
                detaylar: vec![],
            },
            HakedisSatiri {
                kalem_id: "kalem-b".into(),
                poz_no: "15.100.1001".into(),
                kumulatif_miktar: 3.0,
                detaylar: vec![],
            },
        ];

        let hesaplar = poz_hesaplari(&kesif, &h, None, 0.0);
        assert_eq!(hesaplar[0].bu_hakedis_tutar, 200.0);
        assert_eq!(hesaplar[1].bu_hakedis_tutar, 750.0);
    }

    #[test]
    fn odenecek_tutar_kdv_ve_tevkifati_acikca_hesaplar() {
        let kesif = vec![kalem("A", 1000.0, 100.0)];
        let mut h = hakedis(1, &[("A", 10.0)]);
        h.damga_orani = 0.0;
        h.kdv_orani = 20.0;
        h.tevkifat_orani = 0.40;
        let ic = icmal(&poz_hesaplari(&kesif, &h, None, 0.0), &h);
        assert_eq!(ic.net_odeme, 10000.0);
        assert_eq!(ic.kdv, 2000.0);
        assert_eq!(ic.tevkifat, 800.0);
        assert_eq!(ic.odenecek_tutar, 11200.0);
    }

    #[test]
    fn tenzilat_ham_tutari_sozlesme_tutarina_indirir() {
        let kesif = vec![kalem("A", 1000.0, 100.0)];
        let h = hakedis(1, &[("A", 10.0)]);
        let ic = icmal(&poz_hesaplari(&kesif, &h, None, 12.345678), &h);
        assert_eq!(ic.bu_hakedis_ham, 10000.0);
        assert_eq!(ic.tenzilat_tutari, 1234.57);
        assert_eq!(ic.bu_hakedis_brut, 8765.43);
    }

    #[test]
    fn yapim_fiyat_farki_agirlikli_pn_ile_hesaplanir() {
        let kesif = vec![kalem("A", 1000.0, 100.0)];
        let mut h = hakedis(1, &[("A", 10.0)]);
        h.fiyat_farki_ayari.yontem = FiyatFarkiYontemi::YapimAgirlikli;
        h.fiyat_farki_ayari.b = 0.90;
        h.fiyat_farki_ayari.bilesenler[0].katsayi = 0.40;
        h.fiyat_farki_ayari.bilesenler[0].temel_endeks = 100.0;
        h.fiyat_farki_ayari.bilesenler[0].guncel_endeks = 120.0;
        h.fiyat_farki_ayari.bilesenler[1].katsayi = 0.60;
        h.fiyat_farki_ayari.bilesenler[1].temel_endeks = 200.0;
        h.fiyat_farki_ayari.bilesenler[1].guncel_endeks = 220.0;
        // Pn = 0,4×1,2 + 0,6×1,1 = 1,14; F = 10.000×0,90×0,14 = 1.260
        let ic = icmal(&poz_hesaplari(&kesif, &h, None, 0.0), &h);
        assert_eq!(ic.fiyat_farki, 1260.0);
    }
}
