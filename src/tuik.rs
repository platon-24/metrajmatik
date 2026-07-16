//! TÜİK Veri Portalı'ndan Yİ-ÜFE genel endeks serisini alma ve küçük bir yerel
//! önbelleğe dönüştürme.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

const YI_UFE_JSON_URL: &str =
    "https://veriportali.tuik.gov.tr/api/tr/dataflows/DF_YIUFE_EDO_V1+V1.0/file/json";
const PORTAL_REFERER: &str = "https://veriportali.tuik.gov.tr/tr/bulk-download";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct YiUfeSerisi {
    pub kaynak: String,
    pub endeksler: BTreeMap<String, f64>,
}

impl YiUfeSerisi {
    pub fn endeks(&self, ay: &str) -> Option<f64> {
        self.endeksler.get(ay.trim()).copied()
    }

    pub fn son_ay(&self) -> Option<&str> {
        self.endeksler.keys().next_back().map(String::as_str)
    }
}

pub fn ay_gecerli(ay: &str) -> bool {
    let mut parcalar = ay.trim().split('-');
    let yil = parcalar.next().and_then(|v| v.parse::<u32>().ok());
    let ay_no = parcalar.next().and_then(|v| v.parse::<u32>().ok());
    parcalar.next().is_none()
        && matches!(yil, Some(1900..=2200))
        && matches!(ay_no, Some(1..=12))
        && ay.trim().len() == 7
}

pub fn internetten_getir() -> Result<YiUfeSerisi, String> {
    let istemci = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(75))
        .build()
        .map_err(|e| format!("TÜİK bağlantısı hazırlanamadı: {e}"))?;

    let yanit = istemci
        .get(YI_UFE_JSON_URL)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Metrajmatik/0.2",
        )
        .header(reqwest::header::REFERER, PORTAL_REFERER)
        .header(reqwest::header::ACCEPT, "application/json,text/plain,*/*")
        .header(reqwest::header::ACCEPT_LANGUAGE, "tr-TR,tr;q=0.9")
        .send()
        .map_err(|e| format!("TÜİK Veri Portalı'na bağlanılamadı: {e}"))?
        .error_for_status()
        .map_err(|e| format!("TÜİK Veri Portalı yanıt vermedi: {e}"))?;

    let kok: Value = yanit
        .json()
        .map_err(|e| format!("TÜİK verisi okunamadı: {e}"))?;
    json_coz(&kok)
}

pub fn onbellegi_oku(yol: &Path) -> Result<YiUfeSerisi, String> {
    let veri = std::fs::read(yol).map_err(|e| format!("Yİ-ÜFE önbelleği okunamadı: {e}"))?;
    let seri: YiUfeSerisi =
        serde_json::from_slice(&veri).map_err(|e| format!("Yİ-ÜFE önbelleği bozuk: {e}"))?;
    dogrula(seri)
}

pub fn onbellegi_yaz(yol: &Path, seri: &YiUfeSerisi) -> Result<(), String> {
    let veri = serde_json::to_vec_pretty(seri)
        .map_err(|e| format!("Yİ-ÜFE önbelleği hazırlanamadı: {e}"))?;
    // Bu dosya yalnızca yeniden üretilebilir bir önbellektir. Doğrudan üzerine yazmak,
    // Windows'ta var olan dosyanın üstüne `rename` yapılamaması sorununu da önler.
    std::fs::write(yol, veri).map_err(|e| format!("Yİ-ÜFE önbelleği yazılamadı: {e}"))
}

fn json_coz(kok: &Value) -> Result<YiUfeSerisi, String> {
    let veri = kok
        .get("data")
        .ok_or_else(|| "TÜİK yanıtında data alanı yok.".to_string())?;
    let seri_boyutlari = veri
        .pointer("/structure/dimensions/series")
        .and_then(Value::as_array)
        .ok_or_else(|| "TÜİK yanıtında seri boyutları yok.".to_string())?;

    let boyut_konumu = |id: &str| -> Result<usize, String> {
        seri_boyutlari
            .iter()
            .position(|b| b.get("id").and_then(Value::as_str) == Some(id))
            .ok_or_else(|| format!("TÜİK yanıtında {id} boyutu yok."))
    };
    let degesim_konumu = boyut_konumu("DEGISIM")?;
    let urun_konumu = boyut_konumu("URUN_UFE_NACE_CPA")?;
    let grup_konumu = boyut_konumu("FAAL_GRUP")?;

    let degesim_endeks = boyut_deger_konumu(&seri_boyutlari[degesim_konumu], "1")?;
    let urun_genel = boyut_deger_konumu(&seri_boyutlari[urun_konumu], "B-E36")?;
    let grup_toplam = boyut_deger_konumu(&seri_boyutlari[grup_konumu], "_T")?;

    let seriler = veri
        .pointer("/dataSets/0/series")
        .and_then(Value::as_object)
        .ok_or_else(|| "TÜİK yanıtında veri serileri yok.".to_string())?;
    let (_, genel_seri) = seriler
        .iter()
        .find(|(anahtar, _)| {
            let indisler: Vec<usize> = anahtar
                .split(':')
                .filter_map(|v| v.parse::<usize>().ok())
                .collect();
            indisler.len() == seri_boyutlari.len()
                && indisler[degesim_konumu] == degesim_endeks
                && indisler[urun_konumu] == urun_genel
                && indisler[grup_konumu] == grup_toplam
        })
        .ok_or_else(|| "TÜİK yanıtında genel Yİ-ÜFE endeks serisi bulunamadı.".to_string())?;

    let zamanlar = veri
        .pointer("/structure/dimensions/observation")
        .and_then(Value::as_array)
        .and_then(|boyutlar| {
            boyutlar
                .iter()
                .find(|b| b.get("id").and_then(Value::as_str) == Some("TIME_PERIOD"))
        })
        .and_then(|b| b.get("values"))
        .and_then(Value::as_array)
        .ok_or_else(|| "TÜİK yanıtında zaman boyutu yok.".to_string())?;
    let gozlemler = genel_seri
        .get("observations")
        .and_then(Value::as_object)
        .ok_or_else(|| "TÜİK yanıtında Yİ-ÜFE gözlemleri yok.".to_string())?;

    let mut endeksler = BTreeMap::new();
    for (zaman_indisi, gozlem) in gozlemler {
        let Some(indis) = zaman_indisi.parse::<usize>().ok() else {
            continue;
        };
        let Some(ay) = zamanlar
            .get(indis)
            .and_then(|v| v.get("id"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let ilk = gozlem.as_array().and_then(|v| v.first());
        let endeks = ilk.and_then(Value::as_f64).or_else(|| {
            ilk.and_then(Value::as_str)
                .and_then(|v| v.parse::<f64>().ok())
        });
        if ay_gecerli(ay) {
            if let Some(endeks) = endeks.filter(|v| v.is_finite() && *v > 0.0) {
                endeksler.insert(ay.to_string(), endeks);
            }
        }
    }

    dogrula(YiUfeSerisi {
        kaynak: YI_UFE_JSON_URL.to_string(),
        endeksler,
    })
}

fn boyut_deger_konumu(boyut: &Value, aranan_id: &str) -> Result<usize, String> {
    boyut
        .get("values")
        .and_then(Value::as_array)
        .and_then(|degerler| {
            degerler
                .iter()
                .position(|v| v.get("id").and_then(Value::as_str) == Some(aranan_id))
        })
        .ok_or_else(|| format!("TÜİK yanıtında {aranan_id} kodu yok."))
}

fn dogrula(seri: YiUfeSerisi) -> Result<YiUfeSerisi, String> {
    if seri.endeksler.len() < 12 {
        Err("TÜİK Yİ-ÜFE serisi eksik veya boş.".to_string())
    } else {
        Ok(seri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ay_bicimini_dogrular() {
        assert!(ay_gecerli("2026-06"));
        assert!(!ay_gecerli("06.2026"));
        assert!(!ay_gecerli("2026-13"));
    }

    #[test]
    fn resmi_yapidaki_genel_seriyi_cozer() {
        let zamanlar: Vec<Value> = (1..=12)
            .map(|ay| json!({"id": format!("2025-{ay:02}")}))
            .collect();
        let kok = json!({
            "data": {
                "dataSets": [{"series": {
                    "0:0:0:0": {"observations": {
                        "0": ["100.25"], "1": ["101.0"], "2": ["102.0"],
                        "3": ["103.0"], "4": ["104.0"], "5": ["105.0"],
                        "6": ["106.0"], "7": ["107.0"], "8": ["108.0"],
                        "9": ["109.0"], "10": ["110.0"], "11": ["125.75"]
                    }}
                }}],
                "structure": {"dimensions": {
                    "series": [
                        {"id":"DEGISIM", "values":[{"id":"1"}]},
                        {"id":"URUN_UFE_NACE_CPA", "values":[{"id":"B-E36"}]},
                        {"id":"FAAL_GRUP", "values":[{"id":"_T"}]},
                        {"id":"FREQ", "values":[{"id":"M"}]}
                    ],
                    "observation": [{"id":"TIME_PERIOD", "values": zamanlar}]
                }}
            }
        });
        let seri = json_coz(&kok).unwrap();
        assert_eq!(seri.endeks("2025-01"), Some(100.25));
        assert_eq!(seri.endeks("2025-12"), Some(125.75));
    }

    #[test]
    #[ignore = "TÜİK Veri Portalı internet bağlantısı gerektirir"]
    fn resmi_portaldan_guncel_seriyi_alir() {
        let seri = internetten_getir().unwrap();
        assert!(seri.endeksler.len() > 500);
        assert_eq!(seri.endeks("2026-06"), Some(5552.52));
    }
}
