use regex::Regex;
use std::path::Path;

use crate::models::Poz;

pub fn pdf_metin_cikar(pdf_yolu: &Path) -> Result<String, String> {
    let bytes = std::fs::read(pdf_yolu).map_err(|e| format!("PDF okunamadı: {}", e))?;
    pdf_extract::extract_text_from_mem(&bytes).map_err(|e| format!("PDF metin çıkarılamadı: {}", e))
}

/// Bir kurumun birim fiyat PDF'ini ayrıştırma profili. Poz numarası deseni, kategori
/// başlık anahtarları ve atlanacak (başlık/altbilgi) satırlar profile göre değişir;
/// birim ve fiyat çözümü (Türk sayı biçimi) tüm profillerde ortaktır.
///
/// Yeni bir kurum eklemek artık **kod değil, veri** işidir: yeni bir profil tanımla.
pub struct AyristirmaProfili {
    pub ad: String,
    /// Poz satırı regex'i: grup 1 = poz numarası, grup 2 = satırın kalanı.
    pub poz_deseni: String,
    /// (BÜYÜK-HARF anahtar, kategori adı) — başlık satırı tespiti için.
    pub kategori_anahtarlari: Vec<(String, String)>,
    /// Satır bu metinlerden birini içeriyorsa atlanır (sayfa başlığı/altbilgi).
    pub atlama_anahtarlari: Vec<String>,
}

impl AyristirmaProfili {
    /// Çevre, Şehircilik ve İklim Değişikliği Bakanlığı (mevcut, test edilmiş davranış).
    /// Poz biçimi `XX.XXX.XXXX`. Ay adları genel tutuldu (yıla/aya kilitli değil).
    pub fn csb() -> Self {
        AyristirmaProfili {
            ad: "Çevre ve Şehircilik".into(),
            poz_deseni: r"^(\d{2}\.\d{3}\.\d{4})\s*(.*)".into(),
            kategori_anahtarlari: csb_kategorileri(),
            atlama_anahtarlari: [
                "Poz No",
                "TÜİK",
                "Endeksleriyle",
                "Güncel Fiyatlar",
                "OCAK",
                "ŞUBAT",
                "MART",
                "NİSAN",
                "MAYIS",
                "HAZİRAN",
                "TEMMUZ",
                "AĞUSTOS",
                "EYLÜL",
                "EKİM",
                "KASIM",
                "ARALIK",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }

    /// Vakıflar Genel Müdürlüğü / restorasyon: poz biçimi `01.V01` (harf içerir).
    pub fn vakiflar() -> Self {
        AyristirmaProfili {
            ad: "Vakıflar / Restorasyon".into(),
            poz_deseni: r"^(\d{2}\.[A-ZÇĞİÖŞÜ]\d{1,4}(?:\.\d{1,4})?)\s*(.*)".into(),
            kategori_anahtarlari: Vec::new(),
            atlama_anahtarlari: ["Poz No", "Sıra No", "Birim Fiyat", "Sayfa"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    /// Karayolları Ar-Ge birim fiyat listesi: poz biçimi `JH-1`, `M.1.3` (harf kodu).
    pub fn kgm() -> Self {
        AyristirmaProfili {
            ad: "Karayolları (Ar-Ge)".into(),
            poz_deseni: r"^([A-ZÇĞİÖŞÜ]{1,3}[-.]\d+(?:[-./]\d+)*)\s+(.*)".into(),
            kategori_anahtarlari: Vec::new(),
            atlama_anahtarlari: ["Poz No", "Sayfa", "Birim Fiyat", "İÇİNDEKİLER"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    /// Esnek genel profil: tüm yaygın poz biçimlerini kabul eder (elle seçim için).
    pub fn genel() -> Self {
        AyristirmaProfili {
            ad: "Genel".into(),
            poz_deseni: r"^((?:\d{2}\.\d{3}\.\d{4})|(?:\d{2}\.[A-ZÇĞİÖŞÜ]\d{1,4}(?:\.\d{1,4})?)|(?:[A-ZÇĞİÖŞÜ]{1,3}[-.]\d+(?:[-./]\d+)*)|(?:\d{2}\.\d{3}))\s*(.*)".into(),
            kategori_anahtarlari: csb_kategorileri(),
            atlama_anahtarlari: ["Poz No", "İÇİNDEKİLER"].iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Otomatik seçimde denenen profiller. (Genel elle seçilir.)
    pub fn hepsi() -> Vec<AyristirmaProfili> {
        vec![
            AyristirmaProfili::csb(),
            AyristirmaProfili::vakiflar(),
            AyristirmaProfili::kgm(),
        ]
    }
}

fn csb_kategorileri() -> Vec<(String, String)> {
    KATEGORI_ANAHATLARI
        .iter()
        .map(|(a, k)| (a.to_string(), k.to_string()))
        .collect()
}

/// Metne en uygun profili otomatik seçer: en çok poz satırı eşleşen profil.
/// Eşitlikte önce gelen (ÇŞB) kazanır.
pub fn profil_otomatik_sec(metin: &str) -> AyristirmaProfili {
    let mut en_iyi = AyristirmaProfili::csb();
    let mut en_cok = poz_eslesme_sayisi(metin, &en_iyi);
    for profil in AyristirmaProfili::hepsi().into_iter().skip(1) {
        let n = poz_eslesme_sayisi(metin, &profil);
        if n > en_cok {
            en_cok = n;
            en_iyi = profil;
        }
    }
    en_iyi
}

fn poz_eslesme_sayisi(metin: &str, profil: &AyristirmaProfili) -> usize {
    let re = match Regex::new(&profil.poz_deseni) {
        Ok(r) => r,
        Err(_) => return 0,
    };
    metin.lines().filter(|l| re.is_match(l.trim())).count()
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

fn atlanir_mi(satir: &str, atlama: &[String]) -> bool {
    if satir == "TL" || satir == "(TL)" {
        return true;
    }
    atlama.iter().any(|a| satir.contains(a.as_str()))
}

pub fn pozlari_ayristir(
    metin: &str,
    kitap_id: i64,
    kitap_adi: &str,
    yil: u32,
    ay: u32,
    profil: &AyristirmaProfili,
) -> Vec<Poz> {
    let poz_re = match Regex::new(&profil.poz_deseni) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Geçersiz poz deseni ({}): {}", profil.ad, e);
            return Vec::new();
        }
    };
    let fiyat_sonda_re = Regex::new(r"([\d]{1,3}(?:\.[\d]{3})*(?:,\d{2}))\s*$").unwrap();
    let fiyat_re = Regex::new(r"([\d]{1,3}(?:\.[\d]{3})*(?:,\d{2}))").unwrap();
    // Birim tablolarda fiyatın hemen solundadır. Bu yüzden birimi satır sonundan
    // ayırmak, açıklamadaki "m" harflerini yanlış birim sanmaktan daha güvenlidir.
    let birim_sonda_re = Regex::new(r"(?i)(?:^|\s)(1000\s*ad|1000\s*m\s*[²2]|100\s*m\s*[²2]|m\s*[³3]|m\s*[²2]|metre|adet|saat|ton|kg|km|sa|ad|mt|m|[³²])\s*(?:₺|tl)?\s*$").unwrap();
    // Yapışık birim (ör. DSİ "bedelimetre"): boşluk ŞARTI YOK, yalnız çok-harfli
    // yazılı birimler. Sadece normal (boşluklu) tespit başarısız olunca son çare olarak.
    let birim_bitisik_re =
        Regex::new(r"(?i)(metre|adet|saat|ton|kg|m\s*[³3]|m\s*[²2])\s*(?:₺|tl)?\s*$").unwrap();
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

        // Poz satırlarını başlık/kategori filtrelerinden ÖNCE koru: bir poz asla atlanmaz.
        let poz_caps = poz_re.captures(satir);
        if poz_caps.is_none() {
            if atlanir_mi(satir, &profil.atlama_anahtarlari) {
                i += 1;
                continue;
            }
            let mut yeni_kategori = None;
            for (anahtar, kategori_adi) in &profil.kategori_anahtarlari {
                let kontrol_metni = if i + 1 < satirlar.len() {
                    format!("{} {}", satir, satirlar[i + 1].trim())
                } else {
                    satir.to_string()
                };
                if kontrol_metni.to_uppercase().contains(anahtar.as_str()) {
                    yeni_kategori = Some(kategori_adi.clone());
                    break;
                }
            }
            if let Some(kat) = yeni_kategori {
                mevcut_kategori = kat;
            }
            i += 1;
            continue;
        }

        let caps = poz_caps.unwrap();
        let poz_no = caps[1].to_string();
        let kalan = caps
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
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
            if poz_re.is_match(sonraki) {
                break;
            }

            if let Some((satir_tanimi, satir_birimi, fiyat_str)) =
                fiyatli_satir_ayir(sonraki, &fiyat_sonda_re, &fiyat_re, &birim_sonda_re)
            {
                if let Some(b) = satir_birimi.as_ref() {
                    birim = b.clone();
                }
                if !satir_tanimi.is_empty() && satir_birimi.is_some() {
                    tanim_parcalari.push(satir_tanimi);
                }
                fiyat = parse_fiyat(&fiyat_str);
                i = j;
                break;
            }

            let mut baslik_mi = false;
            for (anahtar, _) in &profil.kategori_anahtarlari {
                if sonraki.to_uppercase().contains(anahtar.as_str()) && !poz_re.is_match(sonraki) {
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

        if fiyat.is_none() {
            let bl = tanim_parcalari.join(" ");
            if let Some((satir_tanimi, satir_birimi, fiyat_str)) =
                fiyatli_satir_ayir(&bl, &fiyat_sonda_re, &fiyat_re, &birim_sonda_re)
            {
                fiyat = parse_fiyat(&fiyat_str);
                if let Some(b) = satir_birimi {
                    birim = b;
                }
                tanim_parcalari = vec![satir_tanimi];
            }
        }
        if birim.is_empty() {
            let bl = tanim_parcalari.join(" ");
            let (satir_tanimi, satir_birimi) = tanim_ve_birim_ayir(&bl, &birim_sonda_re);
            if let Some(b) = satir_birimi {
                birim = b;
                tanim_parcalari = vec![satir_tanimi];
            } else {
                // Son çare: yapışık yazılı birim (DSİ "…boru bedelimetre" → "metre").
                let (t2, b2) = tanim_ve_birim_ayir(&bl, &birim_bitisik_re);
                if let Some(b) = b2 {
                    birim = b;
                    tanim_parcalari = vec![t2];
                }
            }
        }
        let mut tanim = tanim_parcalari
            .join(" ")
            .replace("  ", " ")
            .trim()
            .to_string();
        tanim = tanim.replace("  ", " ").trim().to_string();
        if !tanim.is_empty() || !poz_no.is_empty() {
            let final_birim = if birim.is_empty() {
                "---".to_string()
            } else {
                birim_normalize(birim.trim())
            };
            pozlar.push(Poz {
                poz_no,
                tanim: temiz_tanim(&tanim),
                birim: final_birim,
                fiyat,
                kategori: mevcut_kategori.clone(),
                kitap_id,
                kitap_adi: kitap_adi.to_string(),
                yil,
                ay,
            });
        }
        i += 1;
    }
    log::info!(
        "{} ({}) profiliyle {} poz ayrıştırıldı",
        kitap_adi,
        profil.ad,
        pozlar.len()
    );
    pozlar
}

fn parse_fiyat(s: &str) -> Option<f64> {
    s.chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect::<String>()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
}

fn temiz_tanim(s: &str) -> String {
    s.trim()
        .trim_matches(|c: char| c == '-' || c == '.' || c == ',' || c.is_whitespace())
        .to_string()
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
        "ad" | "adet" => "Ad".to_string(),
        "m" | "mt" | "metre" => "m".to_string(),
        "km" => "km".to_string(),
        "sa" | "saat" => "saat".to_string(),
        _ => s.trim().to_string(),
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::{Path, PathBuf};

    fn csb() -> AyristirmaProfili {
        AyristirmaProfili::csb()
    }

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
            &csb(),
        );

        assert_eq!(pozlar.len(), 1);
        assert_eq!(pozlar[0].birim, "m²");
        assert_eq!(
            pozlar[0].tanim,
            "Plywood ile düz yüzeyli betonarme kalıbı yapılması"
        );
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
            &csb(),
        );

        assert_eq!(pozlar[0].birim, "100 m²");
        assert_eq!(pozlar[1].birim, "1000 Ad");
    }

    #[test]
    fn fiyat_arkasina_baslik_yapissa_da_okunur() {
        let pozlar = pozlari_ayristir(
            "15.105.1002\nKazı ve dolgu alanında makine ile temizleme ve sökme işi yapılması 100 m² 811,55 Ağaç Kesilmesi ve Sökme İşi:",
            1, "Test", 2026, 5, &csb(),
        );

        assert_eq!(pozlar.len(), 1);
        assert_eq!(pozlar[0].birim, "100 m²");
        assert_eq!(pozlar[0].fiyat, Some(811.55));
        assert_eq!(
            pozlar[0].tanim,
            "Kazı ve dolgu alanında makine ile temizleme ve sökme işi yapılması"
        );
    }

    #[test]
    fn genel_profil_kisa_poz_bicimini_okur() {
        // "15.150" (XX.XXX) — ÇŞB deseni bunu okumaz, genel profil okur.
        let girdi = "15.150 Beton dökülmesi m³ 842,10";
        let csb_sonuc = pozlari_ayristir(girdi, 1, "T", 2026, 5, &AyristirmaProfili::csb());
        assert!(csb_sonuc.is_empty(), "ÇŞB deseni kısa pozu okumamalı");

        let genel_sonuc = pozlari_ayristir(girdi, 1, "T", 2026, 5, &AyristirmaProfili::genel());
        assert_eq!(genel_sonuc.len(), 1);
        assert_eq!(genel_sonuc[0].poz_no, "15.150");
        assert_eq!(genel_sonuc[0].fiyat, Some(842.10));
    }

    #[test]
    fn otomatik_secim_csb_bicimini_csb_ile_okur() {
        // ÇŞB biçimli metin → otomatik seçim ÇŞB profilini seçmeli (poz_no tam biçim).
        let girdi = "15.180.1003\nKalıp m² 1.123,51\n15.180.1004\nDemir Ton 200,00";
        let profil = profil_otomatik_sec(girdi);
        assert_eq!(profil.ad, "Çevre ve Şehircilik");
        let pozlar = pozlari_ayristir(girdi, 1, "T", 2026, 5, &profil);
        assert_eq!(pozlar.len(), 2);
        assert_eq!(pozlar[0].poz_no, "15.180.1003");
    }

    #[test]
    fn verilen_pdf_orneklerinde_mm2_kacagi_yok() {
        let ust_dizin = Path::new("..");
        let pdfler = [
            pdf_bul(
                ust_dizin,
                "PTT_A.s._YAPI_DAiRE_BAsKANLIgI_oZEL_BiRiM_FiYATLARI.pdf",
            ),
            pdf_bul(ust_dizin, "2026-05-BF.pdf"),
        ];

        for pdf in pdfler.into_iter().flatten() {
            let metin = pdf_metin_cikar(&pdf).expect("PDF metni okunmalı");
            let pozlar = pozlari_ayristir(&metin, 1, "Test", 2026, 5, &csb());
            assert!(!pozlar.is_empty(), "{} içinden poz okunmalı", pdf.display());
            assert!(
                !pozlar
                    .iter()
                    .any(|p| p.birim.contains("mm²") || p.birim.contains("mm³")),
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

    #[test]
    fn kgm_kodlu_pozlari_okur() {
        // KGM Ar-Ge: "JH-1", "M.1.3" gibi harf kodları; ₺ önekli fiyat; km birimi.
        let pozlar = pozlari_ayristir(
            "JH-1  1/25000 Ölçekli Koridor Jeolojik Etüdü  km  ₺109.137,50",
            1,
            "T",
            2026,
            1,
            &AyristirmaProfili::kgm(),
        );
        assert_eq!(pozlar.len(), 1);
        assert_eq!(pozlar[0].poz_no, "JH-1");
        assert_eq!(pozlar[0].birim, "km");
        assert_eq!(pozlar[0].fiyat, Some(109137.50));
    }

    #[test]
    fn vakiflar_v_kodlu_pozlari_okur() {
        // Vakıflar/restorasyon: "V.0001" kodu (harf kodu → KGM/genel deseniyle okunur).
        let pozlar = pozlari_ayristir(
            "V.0001 İnce Kum Bedeli m³ 431,00",
            1,
            "T",
            2026,
            1,
            &AyristirmaProfili::kgm(),
        );
        assert_eq!(pozlar.len(), 1);
        assert_eq!(pozlar[0].poz_no, "V.0001");
        assert_eq!(pozlar[0].birim, "m³");
        assert_eq!(pozlar[0].fiyat, Some(431.0));
    }

    #[test]
    fn saat_birimi_sa_kisaltmasindan_okunur() {
        // "SA" (saat) kısaltması — Vakıflar/KGM işçilik pozlarında.
        let pozlar = pozlari_ayristir(
            "V.01 Kalemkar Usta SA 440,00",
            1,
            "T",
            2026,
            1,
            &AyristirmaProfili::kgm(),
        );
        assert_eq!(pozlar.len(), 1);
        assert_eq!(pozlar[0].birim, "saat");
        assert_eq!(pozlar[0].fiyat, Some(440.0));
    }

    #[test]
    fn dsi_yapisik_birim_ayrilir() {
        // DSİ tek-satır: poz açıklamaya, birim "metre" de "bedeli"ye yapışık.
        let pozlar = pozlari_ayristir(
            "50.205.1001DSİ PE100 uygun PN6 DN110 boru bedelimetre 117,00\n50.205.1002DSİ PE100 uygun PN6 DN125 boru bedelimetre 152,00",
            1, "T", 2026, 1, &AyristirmaProfili::csb(),
        );
        assert_eq!(pozlar.len(), 2);
        assert_eq!(pozlar[0].poz_no, "50.205.1001");
        assert_eq!(pozlar[0].fiyat, Some(117.0));
        assert_eq!(pozlar[0].birim, "m"); // "metre" → normalize → "m"
        assert!(
            pozlar[0].tanim.contains("boru bedeli"),
            "tanım: {}",
            pozlar[0].tanim
        );
        assert!(
            !pozlar[0].tanim.contains("bedelimetre"),
            "birim ayrılmalı: {}",
            pozlar[0].tanim
        );
    }

    /// Gerçek kurum PDF'leriyle uçtan uca doğrulama (örnekler `D:\metrajmatik\` altında).
    /// Normal test koşusunda çalışmaz; elle: `cargo test gercek_kitaplar -- --ignored --nocapture`.
    #[test]
    #[ignore = "gerçek örnek PDF gerektirir"]
    fn gercek_kitaplar_otomatik_ayristirilir() {
        let dizin = Path::new("..");
        let beklenen: &[(&str, &str, usize)] = &[
            ("CSB 2026-06-BF.pdf", "Çevre ve Şehircilik", 1500),
            ("5-2026-yili-temmuz-ayi-altyapi-tesisleri-birim-fiyatlari-listesi.pdf", "Çevre ve Şehircilik", 2000),
            ("9ff3658c-2b13-4705-a553-69e9d470abdc_2026-1_YILI_PTT_A.s._YAPI_DAiRE_BAsKANLIgI_oZEL_BiRiM_FiYATLARI.pdf", "Çevre ve Şehircilik", 200),
            ("dsi_2026_yili_birim_fiyat_kitabi.pdf", "Çevre ve Şehircilik", 1500),
            ("KGM.pdf", "Karayolları (Ar-Ge)", 500),
            ("144790-0BA65348F5C79DC8802F577A9ADF32928D55A642.pdf", "Karayolları (Ar-Ge)", 1500),
        ];
        for (ad, beklenen_profil, en_az) in beklenen {
            let yol = dizin.join(ad);
            if !yol.exists() {
                eprintln!("ATLANDI (yok): {}", ad);
                continue;
            }
            let metin = pdf_metin_cikar(&yol).expect("PDF metni");
            let profil = profil_otomatik_sec(&metin);
            let pozlar = pozlari_ayristir(&metin, 1, "T", 2026, 1, &profil);
            let fiyatli = pozlar.iter().filter(|p| p.fiyat.is_some()).count();
            let birimli = pozlar.iter().filter(|p| p.birim != "---").count();
            eprintln!(
                "{}: profil={} poz={} fiyatli={} birimli={}",
                ad,
                profil.ad,
                pozlar.len(),
                fiyatli,
                birimli
            );
            assert_eq!(&profil.ad, beklenen_profil, "{} için yanlış profil", ad);
            assert!(
                fiyatli >= *en_az,
                "{}: {} fiyatlı poz (en az {} beklendi)",
                ad,
                fiyatli,
                en_az
            );
        }
    }

    fn pdf_bul(dizin: &Path, ad_parcasi: &str) -> Option<PathBuf> {
        std::fs::read_dir(dizin)
            .ok()?
            .filter_map(Result::ok)
            .map(|girdi| girdi.path())
            .find(|yol| {
                yol.file_name()
                    .and_then(|a| a.to_str())
                    .map(|ad| ad.contains(ad_parcasi))
                    .unwrap_or(false)
            })
    }
}
