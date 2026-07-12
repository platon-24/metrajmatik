//! Yaklaşık maliyet özeti hesabı — TEK kaynak.
//!
//! Önceden bu hesap hem `app.rs` (İcmal ekranı) hem `export.rs` (Excel çıktısı)
//! içinde birebir tekrar ediyordu. Buraya toplandı; ileride kâr/KDV kuralları
//! değişirse tek yerde düzeltilir.

/// Ara toplamdan (işçilik + malzeme) genel gider + kâr ve KDV uygulanmış özet.
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
    pub fn hesapla(ara_toplam: f64, genel_gider_kar_orani: f64, kdv_orani: f64) -> Self {
        let kar = ara_toplam * genel_gider_kar_orani / 100.0;
        let kdv_matrahi = ara_toplam + kar;
        let kdv = kdv_matrahi * kdv_orani / 100.0;
        let genel_toplam = kdv_matrahi + kdv;
        Self { ara_toplam, kar, kdv_matrahi, kdv, genel_toplam }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ozet_zinciri() {
        let o = MaliyetOzeti::hesapla(100.0, 25.0, 20.0);
        assert_eq!(o.kar, 25.0);
        assert_eq!(o.kdv_matrahi, 125.0);
        assert_eq!(o.kdv, 25.0);
        assert_eq!(o.genel_toplam, 150.0);
    }

    #[test]
    fn sifir_oranlar_ara_toplami_korur() {
        let o = MaliyetOzeti::hesapla(842.50, 0.0, 0.0);
        assert_eq!(o.genel_toplam, 842.50);
    }
}
