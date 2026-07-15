//! `IsGrubu` ağacı üzerinde çalışan SAF yardımcılar: arama, silme, ilk yaprak, canlı
//! toplam. (Önceden app.rs'nin dibinde serbest fonksiyonlardı.) Bu modül egui'ye
//! bağlı DEĞİLDİR; ağaç çizimi (`is_grubu_agac_ciz`) görünüm katmanındadır.

use crate::models::{IsGrubu, MetrajKalemi};

/// Ağaçta verilen id'ye sahip grubu bulup değiştirilebilir referans döndürür.
pub(crate) fn grup_bul_mut<'a>(
    gruplar: &'a mut [IsGrubu],
    hedef_id: &str,
) -> Option<&'a mut IsGrubu> {
    for g in gruplar.iter_mut() {
        if g.id == hedef_id {
            return Some(g);
        }
        if let Some(bulunan) = grup_bul_mut(&mut g.alt_gruplar, hedef_id) {
            return Some(bulunan);
        }
    }
    None
}

/// Ağaçta verilen id'ye sahip grubu bulup salt-okunur referans döndürür.
pub(crate) fn grup_bul_ref<'a>(gruplar: &'a [IsGrubu], hedef_id: &str) -> Option<&'a IsGrubu> {
    for g in gruplar.iter() {
        if g.id == hedef_id {
            return Some(g);
        }
        if let Some(bulunan) = grup_bul_ref(&g.alt_gruplar, hedef_id) {
            return Some(bulunan);
        }
    }
    None
}

/// Ağaçtan verilen id'ye sahip grubu (ve alt ağacını) siler.
pub(crate) fn grup_sil(gruplar: &mut Vec<IsGrubu>, hedef_id: &str) -> bool {
    if let Some(pos) = gruplar.iter().position(|g| g.id == hedef_id) {
        gruplar.remove(pos);
        return true;
    }
    for g in gruplar.iter_mut() {
        if grup_sil(&mut g.alt_gruplar, hedef_id) {
            return true;
        }
    }
    false
}

/// Ağaçtaki ilk yaprak (alt grubu olmayan) grubun id'sini döndürür.
pub(crate) fn ilk_yaprak_grup_id(gruplar: &[IsGrubu]) -> Option<String> {
    for g in gruplar {
        if g.alt_gruplar.is_empty() {
            return Some(g.id.clone());
        }
        if let Some(id) = ilk_yaprak_grup_id(&g.alt_gruplar) {
            return Some(id);
        }
    }
    None
}

/// Bir grubun canlı toplamı: aktif grubun kalemleri düzenleme tamponundan
/// (`aktif_kalemler`) okunur, diğer grupların kalemleri ağaçtaki halinden.
pub(crate) fn grup_canli_toplam(
    grup: &IsGrubu,
    secili_id: Option<&str>,
    aktif_kalemler: &[MetrajKalemi],
) -> f64 {
    let kalemler_toplam: f64 = if secili_id == Some(grup.id.as_str()) {
        aktif_kalemler.iter().map(|k| k.tutar).sum()
    } else {
        grup.kalemler.iter().map(|k| k.tutar).sum()
    };
    let alt_toplam: f64 = grup
        .alt_gruplar
        .iter()
        .map(|g| grup_canli_toplam(g, secili_id, aktif_kalemler))
        .sum();
    kalemler_toplam + alt_toplam
}
