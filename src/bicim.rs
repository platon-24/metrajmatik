//! Biçimlendirme ve ayrıştırma yardımcıları: para, tarih, metin kısaltma, sayı okuma.
//! Uygulama genelinde TEK kaynaktan kullanılır. (Önceden `krono_tarih` hem
//! `app.rs` hem `database.rs` içinde; biçimlendirme fonksiyonları `app.rs` içinde
//! serbest fonksiyonlardı.)

/// UNIX zamanından basit "YYYY-AA-GG" üretir.
///
/// NOT: Bu hesap yaklaşıktır (artık yıl yok, ay = 30 gün varsayımı). Davranış eski
/// `krono_tarih` ile birebir korunmuştur; gerçek takvime taşınması ayrı bir iş.
pub fn krono_tarih() -> String {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let d = s / 86400;
    let y = 1970 + d / 365;
    let r = d % 365;
    format!("{:04}-{:02}-{:02}", y, r / 30 + 1, r % 30 + 1)
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
}
