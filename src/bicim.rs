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

/// Parayı kuruşa (2 ondalık) yuvarlar — TEK yuvarlama kuralı. Tüm tutar/kâr/KDV
/// hesapları buradan geçer ki f64 kayması (drift) birikmesin ve denetimde tutsun.
pub fn kurus_yuvarla(deger: f64) -> f64 {
    (deger * 100.0).round() / 100.0
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

/// Fiyat araştırması: boşluk / `;` / satır ile ayrılmış tekliflerin ortalaması.
/// Virgül ondalık ayırıcı olduğundan virgülle BÖLÜNMEZ ("1.200,50 1350,00").
pub fn teklif_ortalamasi(metin: &str) -> Option<f64> {
    let sayilar: Vec<f64> = metin.split([' ', ';', '\t', '\n', '\r']).filter_map(sayi_oku).collect();
    if sayilar.is_empty() {
        return None;
    }
    Some(kurus_yuvarla(sayilar.iter().sum::<f64>() / sayilar.len() as f64))
}

/// Bir tam sayıyı Türkçe okunuşuna çevirir ("bin iki yüz otuz dört"). 0 → "sıfır".
/// Türkçe kuralları: "yüz" (bir yüz değil), "bin" (bir bin değil — yalnız binler
/// basamağında ve grup tam 1 iken); "bir milyon" ise "bir" ile yazılır.
fn tam_sayi_yaziya(n: i64) -> String {
    if n == 0 {
        return "sıfır".to_string();
    }
    if n < 0 {
        return format!("eksi {}", tam_sayi_yaziya(-n));
    }
    const BIRLER: [&str; 10] = ["", "bir", "iki", "üç", "dört", "beş", "altı", "yedi", "sekiz", "dokuz"];
    const ONLAR: [&str; 10] = ["", "on", "yirmi", "otuz", "kırk", "elli", "altmış", "yetmiş", "seksen", "doksan"];
    const BASAMAKLAR: [&str; 5] = ["", "bin", "milyon", "milyar", "trilyon"];

    // Sayıyı 3'lü gruplara böl (en düşük basamak önce).
    let mut gruplar: Vec<usize> = Vec::new();
    let mut x = n;
    while x > 0 {
        gruplar.push((x % 1000) as usize);
        x /= 1000;
    }

    let mut parcalar: Vec<String> = Vec::new();
    for (i, &grup) in gruplar.iter().enumerate().rev() {
        if grup == 0 {
            continue;
        }
        let (yuz, on, bir) = (grup / 100, (grup % 100) / 10, grup % 10);
        let mut p = String::new();
        if yuz > 0 {
            if yuz > 1 {
                p.push_str(BIRLER[yuz]);
                p.push(' ');
            }
            p.push_str("yüz");
        }
        if on > 0 {
            if !p.is_empty() {
                p.push(' ');
            }
            p.push_str(ONLAR[on]);
        }
        if bir > 0 && !(i == 1 && grup == 1) {
            // Binler basamağında tam "1" ise "bir" yazma → "bin".
            if !p.is_empty() {
                p.push(' ');
            }
            p.push_str(BIRLER[bir]);
        }
        if i > 0 {
            if !p.is_empty() {
                p.push(' ');
            }
            p.push_str(BASAMAKLAR[i]);
        }
        parcalar.push(p);
    }
    parcalar.join(" ").trim().to_string()
}

/// Para tutarını "yazı ile" Türkçe okunuşuna çevirir: teklif mektubu / hakediş
/// gibi resmî belgelerde rakamın yanına yazılır. Örn: `1234.5` → "bin iki yüz
/// otuz dört TL elli Kr". Kuruş 0 ise yalnız TL kısmı yazılır.
pub fn sayi_yaziya(deger: f64) -> String {
    let yuvarlanmis = kurus_yuvarla(deger.abs());
    let tam = yuvarlanmis.trunc() as i64;
    let kurus = ((yuvarlanmis - tam as f64) * 100.0).round() as i64;
    let isaret = if deger < 0.0 { "eksi " } else { "" };
    let mut s = format!("{}{} TL", isaret, tam_sayi_yaziya(tam));
    if kurus > 0 {
        s.push(' ');
        s.push_str(&tam_sayi_yaziya(kurus));
        s.push_str(" Kr");
    }
    s
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
    fn kurus_yuvarlama_driftu_giderir() {
        assert_eq!(kurus_yuvarla(0.1 + 0.2), 0.3); // 0.30000000000000004 → 0.30
        assert_eq!(kurus_yuvarla(2.348), 2.35);
        assert_eq!(kurus_yuvarla(2.344), 2.34);
        assert_eq!(kurus_yuvarla(842.5), 842.5);
    }

    #[test]
    fn teklif_ortalamasi_virguldeki_ondaligi_korur() {
        // 3 teklif ortalaması; virgül ondalık ayırıcı → bölme yok
        assert_eq!(teklif_ortalamasi("1200,00 1350,00 1180,00"), Some(1243.33));
        assert_eq!(teklif_ortalamasi("100;200;300"), Some(200.0));
        assert_eq!(teklif_ortalamasi(""), None);
    }

    #[test]
    fn metin_kisaltma() {
        assert_eq!(metni_kisalt("kısa", 10), "kısa");
        assert_eq!(metni_kisalt("çok uzun bir metin", 8), "çok u...");
    }

    #[test]
    fn sayi_yaziya_temel() {
        assert_eq!(tam_sayi_yaziya(0), "sıfır");
        assert_eq!(tam_sayi_yaziya(1), "bir");
        assert_eq!(tam_sayi_yaziya(100), "yüz"); // "bir yüz" değil
        assert_eq!(tam_sayi_yaziya(200), "iki yüz");
        assert_eq!(tam_sayi_yaziya(1000), "bin"); // "bir bin" değil
        assert_eq!(tam_sayi_yaziya(2000), "iki bin");
        assert_eq!(tam_sayi_yaziya(1234), "bin iki yüz otuz dört");
        assert_eq!(tam_sayi_yaziya(21000), "yirmi bir bin");
        assert_eq!(tam_sayi_yaziya(1_000_000), "bir milyon"); // "bir" ile
        assert_eq!(tam_sayi_yaziya(1_234_567), "bir milyon iki yüz otuz dört bin beş yüz altmış yedi");
    }

    #[test]
    fn sayi_yaziya_para() {
        assert_eq!(sayi_yaziya(1234.56), "bin iki yüz otuz dört TL elli altı Kr");
        assert_eq!(sayi_yaziya(1000.0), "bin TL"); // kuruş 0 → yalnız TL
        assert_eq!(sayi_yaziya(0.0), "sıfır TL");
        assert_eq!(sayi_yaziya(0.5), "sıfır TL elli Kr");
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
