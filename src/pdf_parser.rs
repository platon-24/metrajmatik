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

pub fn pozlari_ayristir(metin: &str, kitap_id: i64, kitap_adi: &str) -> Vec<Poz> {
    let poz_re = Regex::new(r"^(\d{2}\.\d{3}\.\d{4})\s*(.*)").unwrap();
    let fiyat_re = Regex::new(r"([\d]{1,3}(?:\.[\d]{3})*(?:,\d{2}))\s*$").unwrap();
    let birim_re = Regex::new(r"\b(m³|m²|Ton|Kg|Ad|m\b|100\s*m²|1000\s*Ad|1000\s*m²)\b").unwrap();
    let sayfa_no_re = Regex::new(r"^\s*\d+\s*$").unwrap();

    let mut pozlar: Vec<Poz> = Vec::new();
    let mut mevcut_kategori = "DİĞER".to_string();
    let satirlar: Vec<&str> = metin.lines().collect();
    let mut i = 0;

    while i < satirlar.len() {
        let satir = satirlar[i].trim();

        if satir.is_empty() {
            i += 1;
            continue;
        }
        if sayfa_no_re.is_match(satir) && satir.len() <= 3 {
            i += 1;
            continue;
        }
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

        let mut yeni_kategori = None;
        for (anahtar, kategori_adi) in KATEGORI_ANAHATLARI {
            let kontrol_metni = if i + 1 < satirlar.len() {
                format!("{} {}", satir, satirlar[i + 1].trim())
            } else {
                satir.to_string()
            };
            if kontrol_metni.to_uppercase().contains(anahtar) && poz_re.captures(satir).is_none() {
                yeni_kategori = Some(kategori_adi.to_string());
                break;
            }
        }
        if let Some(kat) = yeni_kategori {
            mevcut_kategori = kat;
            i += 1;
            continue;
        }

        if let Some(caps) = poz_re.captures(satir) {
            let poz_no = caps[1].to_string();
            let kalan = caps[2].to_string();
            let mut tanim_parcalari = vec![kalan];
            let mut fiyat: Option<f64> = None;
            let mut birim = String::new();
            let mut j = i + 1;

            while j < satirlar.len() && j < i + 20 {
                let sonraki = satirlar[j].trim();
                if sonraki.is_empty() || sayfa_no_re.is_match(sonraki) {
                    j += 1; continue;
                }
                if poz_re.is_match(sonraki) { break; }

                if let Some(f_caps) = fiyat_re.captures(sonraki) {
                    let fiyat_str = f_caps[1].to_string();
                    let once = &sonraki[..sonraki.len() - fiyat_str.len()].trim();
                    if let Some(b_caps) = birim_re.captures(once) {
                        birim = b_caps[1].to_string();
                    } else if birim.is_empty() {
                        birim = once.to_string();
                    }
                    let once_trimmed = once.replace(&birim, "").trim().to_string();
                    if !once_trimmed.is_empty()
                        && !once_trimmed.starts_with("m³") && !once_trimmed.starts_with("m²")
                        && !once_trimmed.starts_with("Ton") && !once_trimmed.starts_with("Kg")
                        && !once_trimmed.starts_with("Ad")
                    {
                        tanim_parcalari.push(once_trimmed);
                    }
                    fiyat = parse_fiyat(&fiyat_str);
                    i = j;
                    break;
                }

                let mut baslik_mi = false;
                for (anahtar, _) in KATEGORI_ANAHATLARI {
                    if sonraki.to_uppercase().contains(anahtar) && !poz_re.is_match(sonraki) {
                        baslik_mi = true; break;
                    }
                }
                if baslik_mi { break; }
                tanim_parcalari.push(sonraki.to_string());
                j += 1;
            }

            if fiyat.is_none() {
                let birlestirilmis = tanim_parcalari.join(" ");
                if let Some(f_caps) = fiyat_re.captures(&birlestirilmis) {
                    let fiyat_str = f_caps[1].to_string();
                    fiyat = parse_fiyat(&fiyat_str);
                    let fiyat_pos = birlestirilmis.rfind(&fiyat_str).unwrap_or(0);
                    let tanim_oncu = birlestirilmis[..fiyat_pos].trim();
                    if let Some(b_caps) = birim_re.captures(tanim_oncu) {
                        birim = b_caps[1].to_string();
                    }
                    tanim_parcalari = vec![tanim_oncu.replace(&birim, "").trim().to_string()];
                }
            }
            if birim.is_empty() {
                let birlestirilmis = tanim_parcalari.join(" ");
                if let Some(b_caps) = birim_re.captures(&birlestirilmis) {
                    birim = b_caps[1].to_string();
                }
            }

            let tanim = tanim_parcalari.join(" ").replace("  ", " ").trim().to_string();
            if !tanim.is_empty() || !poz_no.is_empty() {
                let final_birim = if birim.is_empty() { "---".to_string() } else { birim.trim().to_string() };
                pozlar.push(Poz {
                    poz_no,
                    tanim: temiz_tanim(&tanim),
                    birim: final_birim,
                    fiyat,
                    kategori: mevcut_kategori.clone(),
                    kitap_id,
                    kitap_adi: kitap_adi.to_string(),
                });
            }
        }
        i += 1;
    }
    log::info!("{} kitabından {} poz ayrıştırıldı", kitap_adi, pozlar.len());
    pozlar
}

fn parse_fiyat(s: &str) -> Option<f64> {
    let temiz: String = s.chars().filter(|c| c.is_ascii_digit() || *c == ',').collect();
    temiz.replace(',', ".").parse::<f64>().ok()
}

fn temiz_tanim(s: &str) -> String {
    s.trim().trim_matches(|c: char| c == '-' || c == '.' || c == ',' || c.is_whitespace()).to_string()
}