use regex::Regex;
use std::path::Path;

use crate::models::Poz;

pub fn pdf_metin_cikar(pdf_yolu: &Path) -> Result<String, String> {
    let bytes = std::fs::read(pdf_yolu).map_err(|e| format!("PDF okunamadı: {}", e))?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("PDF metin çıkarılamadı: {}", e))
}

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

pub fn pozlari_ayristir(metin: &str, kitap_id: i64, kitap_adi: &str, yil: u32, ay: u32) -> Vec<Poz> {
    let poz_re = Regex::new(r"^(\d{2}\.\d{3}\.\d{4})\s*(.*)").unwrap();
    let fiyat_sonda_re = Regex::new(r"([\d]{1,3}(?:\.[\d]{3})*(?:,\d{2}))\s*$").unwrap();
    let fiyat_re = Regex::new(r"([\d]{1,3}(?:\.[\d]{3})*(?:,\d{2}))").unwrap();
    // Birim tablolarda fiyatın hemen solundadır. Bu yüzden birimi satır sonundan
    // ayırmak, açıklamadaki "m" harflerini yanlış birim sanmaktan daha güvenlidir.
    let birim_sonda_re = Regex::new(r"(?i)(?:^|\s)(1000\s*ad|1000\s*m\s*[²2]|100\s*m\s*[²2]|m\s*[³3]|m\s*[²2]|ton|kg|ad|mt|m|[³²])\s*(?:₺|tl)?\s*$").unwrap();
    let sayfa_no_re = Regex::new(r"^\s*\d+\s*$").unwrap();

    let mut pozlar: Vec<Poz> = Vec::new();
    let mut mevcut_kategori = "DİĞER".to_string();
    let satirlar: Vec<&str> = metin.lines().collect();
    let mut i = 0;

    while i < satirlar.len() {
        let satir = satirlar[i].trim();
        if satir.is_empty() { i += 1; continue; }
        if sayfa_no_re.is_match(satir) && satir.len() <= 3 { i += 1; continue; }
        if satir.contains("2026 MAYIS") || satir.contains("Poz No") || satir.contains("TÜİK")
            || satir.contains("Endeksleriyle") || satir.contains("Güncel Fiyatlar")
            || satir == "TL" || satir == "(TL)"
        { i += 1; continue; }

        let mut yeni_kategori = None;
        for (anahtar, kategori_adi) in KATEGORI_ANAHATLARI {
            let kontrol_metni = if i + 1 < satirlar.len() {
                format!("{} {}", satir, satirlar[i + 1].trim())
            } else { satir.to_string() };
            if kontrol_metni.to_uppercase().contains(anahtar) && poz_re.captures(satir).is_none() {
                yeni_kategori = Some(kategori_adi.to_string());
                break;
            }
        }
        if let Some(kat) = yeni_kategori { mevcut_kategori = kat; i += 1; continue; }

        if let Some(caps) = poz_re.captures(satir) {
            let poz_no = caps[1].to_string();
            let kalan = caps[2].to_string();
            let mut tanim_parcalari = vec![kalan];
            let mut fiyat: Option<f64> = None;
            let mut birim = String::new();
            let mut j = i + 1;

            while j < satirlar.len() && j < i + 20 {
                let sonraki = satirlar[j].trim();
                if sonraki.is_empty() || sayfa_no_re.is_match(sonraki) { j += 1; continue; }
                if poz_re.is_match(sonraki) { break; }

                if let Some((satir_tanimi, satir_birimi, fiyat_str)) = fiyatli_satir_ayir(sonraki, &fiyat_sonda_re, &fiyat_re, &birim_sonda_re) {
                    if let Some(b) = satir_birimi.as_ref() { birim = b.clone(); }
                    if !satir_tanimi.is_empty() && satir_birimi.is_some() { tanim_parcalari.push(satir_tanimi); }
                    fiyat = parse_fiyat(&fiyat_str);
                    i = j;
                    break;
                }

                let mut baslik_mi = false;
                for (anahtar, _) in KATEGORI_ANAHATLARI {
                    if sonraki.to_uppercase().contains(anahtar) && !poz_re.is_match(sonraki) { baslik_mi = true; break; }
                }
                if baslik_mi { break; }
                tanim_parcalari.push(sonraki.to_string());
                j += 1;
            }

            if fiyat.is_none() {
                let bl = tanim_parcalari.join(" ");
                if let Some((satir_tanimi, satir_birimi, fiyat_str)) = fiyatli_satir_ayir(&bl, &fiyat_sonda_re, &fiyat_re, &birim_sonda_re) {
                    fiyat = parse_fiyat(&fiyat_str);
                    if let Some(b) = satir_birimi { birim = b; }
                    tanim_parcalari = vec![satir_tanimi];
                }
            }
            if birim.is_empty() {
                let bl = tanim_parcalari.join(" ");
                let (satir_tanimi, satir_birimi) = tanim_ve_birim_ayir(&bl, &birim_sonda_re);
                if let Some(b) = satir_birimi {
                    birim = b;
                    tanim_parcalari = vec![satir_tanimi];
                }
            }
            let mut tanim = tanim_parcalari.join(" ").replace("  ", " ").trim().to_string();
            tanim = tanim.replace("  ", " ").trim().to_string();
            if !tanim.is_empty() || !poz_no.is_empty() {
                let final_birim = if birim.is_empty() { "---".to_string() } else { birim_normalize(birim.trim()) };
                pozlar.push(Poz {
                    poz_no, tanim: temiz_tanim(&tanim), birim: final_birim, fiyat,
                    kategori: mevcut_kategori.clone(), kitap_id,
                    kitap_adi: kitap_adi.to_string(), yil, ay,
                });
            }
        }
        i += 1;
    }
    log::info!("{} kitabindan {} poz ayrıştırıldı", kitap_adi, pozlar.len());
    pozlar
}

fn parse_fiyat(s: &str) -> Option<f64> {
    s.chars().filter(|c| c.is_ascii_digit() || *c == ',').collect::<String>().replace(',', ".").parse::<f64>().ok()
}

fn temiz_tanim(s: &str) -> String {
    s.trim().trim_matches(|c: char| c == '-' || c == '.' || c == ',' || c.is_whitespace()).to_string()
}

fn tanim_ve_birim_ayir(metin: &str, birim_sonda_re: &Regex) -> (String, Option<String>) {
    let metin = metin.trim();
    if let Some(eslesme) = birim_sonda_re.captures(metin) {
        if let Some(birim) = eslesme.get(1) {
            let tanim = metin[..birim.start()].trim().to_string();
            return (tanim, Some(birim.as_str().trim().to_string()));
        }
    }
    (metin.to_string(), None)
}

fn fiyatli_satir_ayir(
    satir: &str,
    fiyat_sonda_re: &Regex,
    fiyat_re: &Regex,
    birim_sonda_re: &Regex,
) -> Option<(String, Option<String>, String)> {
    if let Some(eslesme) = fiyat_sonda_re.captures(satir).and_then(|c| c.get(1)) {
        let fiyat_str = eslesme.as_str().to_string();
        let fiyat_oncesi = satir[..eslesme.start()].trim();
        let (tanim, birim) = tanim_ve_birim_ayir(fiyat_oncesi, birim_sonda_re);
        return Some((tanim, birim, fiyat_str));
    }

    for eslesme in fiyat_re.captures_iter(satir).filter_map(|c| c.get(1)) {
        let fiyat_oncesi = satir[..eslesme.start()].trim();
        let (tanim, birim) = tanim_ve_birim_ayir(fiyat_oncesi, birim_sonda_re);
        if birim.is_some() {
            return Some((tanim, birim, eslesme.as_str().to_string()));
        }
    }

    None
}

/// PDF çıkarımında üst simge ³/² kaybolduğunda düz ASCII karakterleri UTF-8 üst simgelere çevirir
fn birim_normalize(s: &str) -> String {
    let kucuk = s.trim().to_lowercase().replace(char::is_whitespace, "");
    match kucuk.as_str() {
        "m3" | "m³" | "³" => "m³".to_string(),
        "m2" | "m²" | "²" => "m²".to_string(),
        "100m2" | "100m²" => "100 m²".to_string(),
        "1000m2" | "1000m²" => "1000 m²".to_string(),
        "1000ad" => "1000 Ad".to_string(),
        "ton" => "Ton".to_string(),
        "kg" => "Kg".to_string(),
        "ad" => "Ad".to_string(),
        "m" | "mt" => "m".to_string(),
        _ => s.trim().to_string(),
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn birim_normalize_ust_simgeleri_cift_m_yapmaz() {
        assert_eq!(birim_normalize("m²"), "m²");
        assert_eq!(birim_normalize("m ²"), "m²");
        assert_eq!(birim_normalize("²"), "m²");
        assert_eq!(birim_normalize("m³"), "m³");
        assert_eq!(birim_normalize("m ³"), "m³");
        assert_eq!(birim_normalize("³"), "m³");
    }

    #[test]
    fn fiyat_oncesindeki_birim_sondan_ayrilir() {
        let pozlar = pozlari_ayristir(
            "15.180.1003\nPlywood ile düz yüzeyli betonarme kalıbı yapılması m² 1.123,51",
            1,
            "Test",
            2026,
            5,
        );

        assert_eq!(pozlar.len(), 1);
        assert_eq!(pozlar[0].birim, "m²");
        assert_eq!(pozlar[0].tanim, "Plywood ile düz yüzeyli betonarme kalıbı yapılması");
        assert_eq!(pozlar[0].fiyat, Some(1123.51));
    }

    #[test]
    fn uzun_birimler_kirpilmaz() {
        let pozlar = pozlari_ayristir(
            "15.999.0001\nÖrnek imalat 100 m² 12,34\n15.999.0002\nÖrnek sayım 1000 Ad 56,78",
            1,
            "Test",
            2026,
            5,
        );

        assert_eq!(pozlar[0].birim, "100 m²");
        assert_eq!(pozlar[1].birim, "1000 Ad");
    }

    #[test]
    fn fiyat_arkasina_baslik_yapissa_da_okunur() {
        let pozlar = pozlari_ayristir(
            "15.105.1002\nKazı ve dolgu alanında makine ile temizleme ve sökme işi yapılması 100 m² 811,55 Ağaç Kesilmesi ve Sökme İşi:",
            1,
            "Test",
            2026,
            5,
        );

        assert_eq!(pozlar.len(), 1);
        assert_eq!(pozlar[0].birim, "100 m²");
        assert_eq!(pozlar[0].fiyat, Some(811.55));
        assert_eq!(pozlar[0].tanim, "Kazı ve dolgu alanında makine ile temizleme ve sökme işi yapılması");
    }

    #[test]
    fn verilen_pdf_orneklerinde_mm2_kacagi_yok() {
        let ust_dizin = Path::new("..");
        let pdfler = [
            pdf_bul(ust_dizin, "PTT_A.s._YAPI_DAiRE_BAsKANLIgI_oZEL_BiRiM_FiYATLARI.pdf"),
            pdf_bul(ust_dizin, "2026-05-BF.pdf"),
        ];

        for pdf in pdfler.into_iter().flatten() {
            let metin = pdf_metin_cikar(&pdf).expect("PDF metni okunmalı");
            let pozlar = pozlari_ayristir(&metin, 1, "Test", 2026, 5);
            assert!(!pozlar.is_empty(), "{} içinden poz okunmalı", pdf.display());
            assert!(
                !pozlar.iter().any(|p| p.birim.contains("mm²") || p.birim.contains("mm³")),
                "{} içinde hatalı mm²/mm³ birimi olmamalı",
                pdf.display()
            );

            if let Some(poz) = pozlar.iter().find(|p| p.poz_no == "15.180.1003") {
                assert_eq!(poz.birim, "m²");
                assert_eq!(poz.fiyat, Some(1123.51));
            }
            if let Some(poz) = pozlar.iter().find(|p| p.poz_no == "15.105.1002") {
                assert_eq!(poz.birim, "100 m²");
                assert_eq!(poz.fiyat, Some(811.55));
            }
        }
    }

    fn pdf_bul(dizin: &Path, ad_parcasi: &str) -> Option<PathBuf> {
        std::fs::read_dir(dizin)
            .ok()?
            .filter_map(Result::ok)
            .map(|girdi| girdi.path())
            .find(|yol| yol.file_name().and_then(|a| a.to_str()).map(|ad| ad.contains(ad_parcasi)).unwrap_or(false))
    }
}
