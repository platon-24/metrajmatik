//! Yaklaşık maliyet özeti hesabı — TEK kaynak.
//!
//! Önceden bu hesap hem `app.rs` (İcmal ekranı) hem `export.rs` (Excel çıktısı)
//! içinde birebir tekrar ediyordu. Buraya toplandı; kâr/KDV kuralları tek yerde.
//!
//! Mevzuat: **Kamu** yaklaşık maliyeti **KDV hariç** hesaplanır; bu nedenle Kamu
//! türünde KDV oranı ne olursa olsun 0 uygulanır (bkz. [`HesapTuru`]).

use crate::models::HesapTuru;

/// Ara toplamdan (işçilik + malzeme) genel gider + kâr ve (varsa) KDV uygulanmış özet.
#[derive(Debug, Clone, Copy)]
pub struct MaliyetOzeti {
    pub ara_toplam: f64,
    pub kar: f64,
    pub kdv_matrahi: f64,
    pub kdv: f64,
    pub genel_toplam: f64,
}

impl MaliyetOzeti {
    /// Ara toplam + (% genel gider & kâr) + (% KDV) zincirini hesaplar.
    /// Kamu türünde KDV **hariçtir** (oran verilse bile 0 sayılır).
    pub fn hesapla(ara_toplam: f64, genel_gider_kar_orani: f64, kdv_orani: f64, hesap_turu: HesapTuru) -> Self {
        let kar = ara_toplam * genel_gider_kar_orani / 100.0;
        let kdv_matrahi = ara_toplam + kar;
        let etkin_kdv_orani = if hesap_turu.kamu_mu() { 0.0 } else { kdv_orani };
        let kdv = kdv_matrahi * etkin_kdv_orani / 100.0;
        let genel_toplam = kdv_matrahi + kdv;
        Self { ara_toplam, kar, kdv_matrahi, kdv, genel_toplam }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ozel_kip_kdv_uygular() {
        let o = MaliyetOzeti::hesapla(100.0, 25.0, 20.0, HesapTuru::Ozel);
        assert_eq!(o.kar, 25.0);
        assert_eq!(o.kdv_matrahi, 125.0);
        assert_eq!(o.kdv, 25.0);
        assert_eq!(o.genel_toplam, 150.0);
    }

    #[test]
    fn kamu_kip_kdv_haric() {
        // Kamu: KDV uygulanmaz; oran verilse bile 0 sayılır.
        let o = MaliyetOzeti::hesapla(100.0, 0.0, 20.0, HesapTuru::Kamu);
        assert_eq!(o.kar, 0.0);
        assert_eq!(o.kdv, 0.0);
        assert_eq!(o.genel_toplam, 100.0);
    }

    #[test]
    fn kamu_kip_analiz_karini_uygular_kdv_haric() {
        // Kamu ama analiz pozları için kâr girilmiş: kâr uygulanır, KDV yine hariç.
        let o = MaliyetOzeti::hesapla(100.0, 25.0, 20.0, HesapTuru::Kamu);
        assert_eq!(o.kar, 25.0);
        assert_eq!(o.kdv, 0.0);
        assert_eq!(o.genel_toplam, 125.0);
    }
}
