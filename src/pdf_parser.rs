use regex::Regex;
use std::path::Path;

use crate::models::Poz;

/// PDF'ten ham metin çıkarır
pub fn pdf_metin_cikar(pdf_yolu: &Path) -> Result<String, String> {
    let bytes = std::fs::read(pdf_yolu).map_err(|e| format!("PDF okunamadı: {}", e))?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("PDF metin çıkarılamadı: {}", e))
}

/// Bilinen birimler listesi (tanım metninden ayırmak için)
const BIRIMLER: &[&str] = &[
    "100 m²", "100  m²", "1000 Ad", "1000 m²", "m³", "m²", "m ", "m",
    "Ton", "Kg", "Ad", "TL",
];

/// Bilinen kategori başlık anahtar kelimeleri
const KATEGORI_ANAHATLARI: &[(&str, &str)] = &[
    ("MALZEMELERİN YÜKLEME", "YÜKLEME, BOŞALTMA VE İSTİF"),
    ("KAZI ALANINDAKİ FUNDALIK", "FUNDALIK-AĞAÇ TEMİZLİĞİ"),
    ("YIKIM VE SÖKÜM", "YIKIM VE SÖKÜM İŞLERİ"),
    ("Yıkım İşleri", "YIKIM İŞLERİ"),
    ("Söküm İşleri", "SÖKÜM İŞLERİ"),
    ("KAZILARDA DERİNLİK ZAMMI", "DERİNLİK ZAMMI"),
    ("EL İLE YAPILAN SERBEST KAZILAR", "EL İLE SERBEST KAZI"),
    ("EL İLE YAPILAN DERİN KAZILAR", "EL İLE DERİN KAZI"),
    ("BİNA İNŞAATLARINDAKİ MAKİNALI KAZILAR", "MAKİNALI KAZI"),
    ("DOLGU İŞLERİ", "DOLGU İŞLERİ"),
    ("İKSA İŞLERİ", "İKSA İŞLERİ"),
    ("JET GROUT İŞLERİ", "JET GROUT"),
    ("FORE KAZIK İŞLERİ", "FORE KAZIK"),
    ("DİYAFRAM DUVAR", "DİYAFRAM DUVAR"),
    ("HAZIR BETONLAR", "HAZIR BETON - GRİ"),
    ("HAZIR BETONLAR", "HAZIR BETON - BEYAZ"),
    ("SİLİNDİR İLE SIKIŞTIRILMIŞ", "SİLİNDİR BETON YOL"),
    ("Kayar Kalıplı", "KAYAR KALIPLI BETON"),
    ("Beton Yol Yüzeyinin", "BETON YOL KÜR"),
    ("BETON PREFABRİK", "PREFABRİK İMALAT"),
    ("BETON ÇELİK ÇUBUKLARININ", "BETON ÇELİK ÇUBUK"),
    ("DEMİR İNŞAAT", "DEMİR İNŞAAT"),
    ("KALIP", "KALIP İŞLERİ"),
    ("KALIP VE İŞ İSKELELERİ", "KALIP VE İŞ İSKELELERİ"),
    ("GÜVENLİK AĞI", "GÜVENLİK AĞI"),
    ("ÇEŞİTLİ YAPI KİMYASALI", "YAPI KİMYASALI"),
    ("BETON/BETONARME BORU", "BETON BORU"),
    ("DRENAJ LEVHASI", "DRENAJ LEVHASI"),
    ("PVC ESASLI", "PVC DRENAJ BORUSU"),
    ("TAŞ İŞLERİ", "TAŞ İŞLERİ"),
    ("TUĞLA İŞLERİ", "TUĞLA İŞLERİ"),
    ("GAZBETON İŞLERİ", "GAZBETON İŞLERİ"),
];

/// PDF'ten ayrıştırılmış poz listesini çıkarır
pub fn pozlari_ayristir(metin: &str) -> Vec<Poz> {
    let poz_re = Regex::new(r"^(\d{2}\.\d{3}\.\d{4})\s*(.*)").unwrap();
    let fiyat_re = Regex::new(r"([\d]{1,3}(?:\.[\d]{3})*(?:,\d{2}))\s*$").unwrap();
    let birim_re = Regex::new(r"\b(m³|m²|Ton|Kg|Ad|m\b|100\s*m²|1000\s*Ad|1000\s*m²)\b").unwrap();
    let sayfa_no_re = Regex::new(r"^\s*\d+\s*$").unwrap();

    let mut pozlar: Vec<Poz> = Vec::new();
    let mut mevcut_kategori = "DİĞER".to_string();

    // Satırları birleştir ve normalize et
    let satirlar: Vec<&str> = metin.lines().collect();
    let mut i = 0;

    while i < satirlar.len() {
        let satir = satirlar[i].trim();

        // Boş satırları atla
        if satir.is_empty() {
            i += 1;
            continue;
        }

        // Sayfa numarası kontrolü (tek başına sayı)
        if sayfa_no_re.is_match(satir) && satir.len() <= 3 {
            i += 1;
            continue;
        }

        // Başlık satırı kontrolü - "2026 MAYIS" veya "Poz No Tanım Birim" gibi
        if satir.contains("2026 MAYIS")
            || satir.contains("Poz No")
            || satir.contains("TÜİK")
            || satir.contains("Endeksleriyle")
            || satir.contains("Güncel Fiyatlar")
            || satir == "TL"
            || satir == "(TL)"
        {
            i += 1;
            continue;
        }

        // Kategori başlığı kontrolü
        let mut yeni_kategori = None;
        for (anahtar, kategori_adi) in KATEGORI_ANAHATLARI {
            // Önce mevcut satırda ara, 1-2 satır ileriye de bak
            let kontrol_metni = if i + 1 < satirlar.len() {
                format!("{} {}", satir, satirlar[i + 1].trim())
            } else {
                satir.to_string()
            };

            if kontrol_metni.to_uppercase().contains(anahtar) {
                // Satırda poz numarası yoksa kategori başlığı olarak kabul et
                let poz_kontrol = poz_re.captures(satir);
                if poz_kontrol.is_none() {
                    yeni_kategori = Some(kategori_adi.to_string());
                    break;
                }
            }
        }

        if let Some(kat) = yeni_kategori {
            mevcut_kategori = kat;
            i += 1;
            continue;
        }

        // Poz numarası ara
        if let Some(caps) = poz_re.captures(satir) {
            let poz_no = caps[1].to_string();
            let kalan = caps[2].to_string();

            // Çok satırlı tanımı topla (fiyat bulunana kadar)
            let mut tanim_parcalari = vec![kalan];
            let mut fiyat: Option<f64> = None;
            let mut birim = String::new();
            let mut j = i + 1;

            while j < satirlar.len() && j < i + 20 {
                let sonraki = satirlar[j].trim();

                if sonraki.is_empty() || sayfa_no_re.is_match(sonraki) {
                    j += 1;
                    continue;
                }

                // Eğer sonraki satırda yeni poz numarası varsa dur
                if poz_re.is_match(sonraki) {
                    break;
                }

                // Fiyat ara: sonunda XX.XXX,XX veya XXX,XX formatı
                if let Some(f_caps) = fiyat_re.captures(sonraki) {
                    let fiyat_str = f_caps[1].to_string();
                    // Fiyat satırından birim'i ayıkla
                    let once = &sonraki[..sonraki.len() - fiyat_str.len()].trim();

                    // Birim ara
                    if let Some(b_caps) = birim_re.captures(once) {
                        birim = b_caps[1].to_string();
                    } else if birim.is_empty() {
                        birim = once.to_string();
                    }

                    // Eğer bu satır sadece birim+fiyat değilse, tanıma ekle
                    let once_trimmed = once.replace(&birim, "").trim().to_string();
                    if !once_trimmed.is_empty()
                        && !once_trimmed.starts_with("m³")
                        && !once_trimmed.starts_with("m²")
                        && !once_trimmed.starts_with("Ton")
                        && !once_trimmed.starts_with("Kg")
                        && !once_trimmed.starts_with("Ad")
                    {
                        tanim_parcalari.push(once_trimmed);
                    }

                    fiyat = parse_fiyat(&fiyat_str);
                    i = j;
                    break;
                }

                // Kategori başlığı kontrolü
                let mut baslik_mi = false;
                for (anahtar, _) in KATEGORI_ANAHATLARI {
                    if sonraki.to_uppercase().contains(anahtar) && !poz_re.is_match(sonraki) {
                        baslik_mi = true;
                        break;
                    }
                }
                if baslik_mi {
                    break;
                }

                tanim_parcalari.push(sonraki.to_string());
                j += 1;
            }

            // Fiyat bulunamadıysa satırın içinde ara
            if fiyat.is_none() {
                let birlestirilmis = tanim_parcalari.join(" ");
                if let Some(f_caps) = fiyat_re.captures(&birlestirilmis) {
                    let fiyat_str = f_caps[1].to_string();
                    fiyat = parse_fiyat(&fiyat_str);

                    // Birimi tanımdan ayır
                    let fiyat_pos = birlestirilmis.rfind(&fiyat_str).unwrap_or(0);
                    let tanim_oncu = birlestirilmis[..fiyat_pos].trim();

                    if let Some(b_caps) = birim_re.captures(tanim_oncu) {
                        birim = b_caps[1].to_string();
                    }

                    // Temiz tanım
                    tanim_parcalari = vec![tanim_oncu.replace(&birim, "").trim().to_string()];
                }
            }

            // Birim hala boşsa tanım içinde ara
            if birim.is_empty() {
                let birlestirilmis = tanim_parcalari.join(" ");
                if let Some(b_caps) = birim_re.captures(&birlestirilmis) {
                    birim = b_caps[1].to_string();
                }
            }

            let tanim = tanim_parcalari
                .join(" ")
                .replace("  ", " ")
                .trim()
                .to_string();

            // Çok kısa tanımları veya boş tanımları filtrele
            if !tanim.is_empty() || !poz_no.is_empty() {
                let final_birim = if birim.is_empty() {
                    "---".to_string()
                } else {
                    birim.trim().to_string()
                };

                pozlar.push(Poz {
                    poz_no,
                    tanim: temiz_tanim(&tanim),
                    birim: final_birim,
                    fiyat,
                    kategori: mevcut_kategori.clone(),
                });
            }
        }

        i += 1;
    }

    log::info!("Toplam {} poz ayrıştırıldı", pozlar.len());
    pozlar
}

fn parse_fiyat(s: &str) -> Option<f64> {
    let temiz: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    let ondalikli = temiz.replace(',', ".");
    ondalikli.parse::<f64>().ok()
}

fn temiz_tanim(s: &str) -> String {
    // Gereksiz boşlukları temizle
    let s = s.trim();
    // Başında ve sonunda gereksiz karakterleri temizle
    let s = s.trim_matches(|c: char| c == '-' || c == '.' || c == ',' || c.is_whitespace());
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fiyat_parse() {
        assert_eq!(parse_fiyat("280,21"), Some(280.21));
        assert_eq!(parse_fiyat("1.120,85"), Some(1120.85));
        assert_eq!(parse_fiyat("47.899,25"), Some(47899.25));
    }

    #[test]
    fn test_ornek_poz() {
        let metin = "15.100.1001 1 ton her cins çimento ve kirecin taşıtlara yükleme, boşaltma ve istifi (Fabrikadan alınan malzemeden yükleme bedeli düşülür.) Ton 280,21";
        let pozlar = pozlari_ayristir(metin);
        assert!(!pozlar.is_empty());
        let poz = &pozlar[0];
        assert_eq!(poz.poz_no, "15.100.1001");
        assert_eq!(poz.fiyat, Some(280.21));
    }
}