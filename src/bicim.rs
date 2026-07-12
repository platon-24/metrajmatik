//! Biçimlendirme ve ayrıştırma yardımcıları: para, tarih, metin kısaltma, sayı okuma.
//! Uygulama genelinde TEK kaynaktan kullanılır. (Önceden `krono_tarih` hem
//! `app.rs` hem `database.rs` içinde; biçimlendirme fonksiyonları `app.rs` içinde
//! serbest fonksiyonlardı.)

/// Bugünün tarihini "YYYY-AA-GG" olarak Türkiye saatiyle üretir.
///
/// Gregoryen takvim; artık yıllar doğru hesaplanır. Türkiye 2016'dan beri kalıcı
/// olarak UTC+3'tür (yaz saati yok), bu yüzden sabit +3 saat ofseti uygulanır.
pub fn krono_tarih() -> String {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let yerel = s as i64 + 3 * 3600; // Türkiye saati (UTC+3)
    let gun = yerel.div_euclid(86400); // 1970-01-01 = 0
    let (y, a, g) = takvim_gununden(gun);
    format!("{:04}-{:02}-{:02}", y, a, g)
}

/// Gün sayısını (1970-01-01 = 0) Gregoryen (yıl, ay, gün)'e çevirir.
/// Howard Hinnant'ın `civil_from_days` algoritması (public domain, artık yıl doğru).
fn takvim_gununden(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let g = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let a = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if a <= 2 { y + 1 } else { y }, a as u32, g as u32)
}

/// Metni en fazla `en_fazla` karaktere kısaltır, taşarsa sonuna "..." ekler.
pub fn metni_kisalt(metin: &str, en_fazla: usize) -> String {
    if metin.chars().count() <= en_fazla {
        return metin.to_string();
    }
    let govde: String = metin.chars().take(en_fazla.saturating_sub(3)).collect();
    format!("{}...", govde)
}

/// f64 parayı "1.234.567,89" (Türk biçimi, binlik nokta / ondalık virgül) yazdırır.
pub fn para_formatla(deger: f64) -> String {
    let isaret = if deger.is_sign_negative() { "-" } else { "" };
    let yuvarlanmis = format!("{:.2}", deger.abs());
    let mut parcalar = yuvarlanmis.split('.');
    let tam = parcalar.next().unwrap_or("0");
    let kurus = parcalar.next().unwrap_or("00");
    let mut gruplu_ters = String::new();
    for (idx, ch) in tam.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            gruplu_ters.push('.');
        }
        gruplu_ters.push(ch);
    }
    let gruplu: String = gruplu_ters.chars().rev().collect();
    format!("{}{},{}", isaret, gruplu, kurus)
}

/// Kullanıcı girdisini ("1.234,56" veya "1234.56") f64'e çevirir (boşsa None).
pub fn sayi_oku(metin: &str) -> Option<f64> {
    let mut temiz = metin.trim().replace(' ', "");
    if temiz.contains(',') {
        temiz = temiz.replace('.', "").replace(',', ".");
    }
    if temiz.is_empty() {
        return None;
    }
    temiz.parse::<f64>().ok()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn para_turk_bicimi() {
        assert_eq!(para_formatla(1234567.89), "1.234.567,89");
        assert_eq!(para_formatla(0.0), "0,00");
        assert_eq!(para_formatla(-12.5), "-12,50");
    }

    #[test]
    fn sayi_okuma_iki_bicim() {
        assert_eq!(sayi_oku("1.234,56"), Some(1234.56));
        assert_eq!(sayi_oku("1234.56"), Some(1234.56));
        assert_eq!(sayi_oku("  "), None);
    }

    #[test]
    fn metin_kisaltma() {
        assert_eq!(metni_kisalt("kısa", 10), "kısa");
        assert_eq!(metni_kisalt("çok uzun bir metin", 8), "çok u...");
    }

    #[test]
    fn takvim_bilinen_gunler() {
        assert_eq!(takvim_gununden(0), (1970, 1, 1));
        assert_eq!(takvim_gununden(10957), (2000, 1, 1));
        assert_eq!(takvim_gununden(-1), (1969, 12, 31)); // negatif gün de doğru
    }

    #[test]
    fn takvim_artik_yil() {
        // 2020 artık yıl: 29 Şubat gerçek bir gün, 1 Mart'a doğru geçmeli
        assert_eq!(takvim_gununden(18321), (2020, 2, 29));
        assert_eq!(takvim_gununden(18322), (2020, 3, 1));
    }
}
