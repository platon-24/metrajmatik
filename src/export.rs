use rust_xlsxwriter::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::maliyet::MaliyetOzeti;
use crate::models::{
    AnalizGirdisi, Hakedis, IsProgrami, KayitliMetraj, MetrajKalemi, ProjeBilgi, SozlesmeAyarlari,
    VeriPaketi,
};

static GUVENLI_YAZ_SAYACI: AtomicU64 = AtomicU64::new(1);

fn ekli_yol(yol: &Path, ek: &str) -> PathBuf {
    let mut ad = yol.as_os_str().to_os_string();
    ad.push(ek);
    PathBuf::from(ad)
}

/// Metni önce aynı dizindeki geçici dosyaya tamamen yazıp diske zorlar. Var olan
/// hedef, değiştirilmeden önce `.bak` olarak korunur; etkinleştirme başarısızsa
/// otomatik geri alınır. Böylece yarım JSON mevcut projeyi doğrudan bozmaz.
fn guvenli_metin_yaz(yol: &Path, icerik: &str) -> Result<(), String> {
    let sayac = GUVENLI_YAZ_SAYACI.fetch_add(1, Ordering::Relaxed);
    let gecici = ekli_yol(yol, &format!(".{}.{}.tmp", std::process::id(), sayac));
    let yedek = ekli_yol(yol, ".bak");

    let yazma = (|| -> std::io::Result<()> {
        let mut dosya = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&gecici)?;
        dosya.write_all(icerik.as_bytes())?;
        dosya.sync_all()?;
        drop(dosya);

        let hedef_vardi = yol.exists();
        if hedef_vardi {
            if yedek.exists() {
                std::fs::remove_file(&yedek)?;
            }
            std::fs::rename(yol, &yedek)?;
        }
        if let Err(e) = std::fs::rename(&gecici, yol) {
            if hedef_vardi {
                let _ = std::fs::rename(&yedek, yol);
            }
            return Err(e);
        }
        Ok(())
    })();

    if yazma.is_err() {
        let _ = std::fs::remove_file(&gecici);
    }
    yazma.map_err(|e| e.to_string())
}

/// Ay numarasını (1-12) Türkçe ay adına çevirir.
pub fn ay_adi(ay: u32) -> &'static str {
    const AYLAR: [&str; 12] = [
        "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
        "Kasım", "Aralık",
    ];
    AYLAR.get((ay.max(1) - 1) as usize).copied().unwrap_or("—")
}

/// Bir pozun analiz föyü: girdiler + poza uygulanan birim fiyat.
/// (Genel gider + kâr, birim fiyat ile ara toplam farkından geri hesaplanır.)
pub struct AnalizFoyu {
    pub poz_no: String,
    pub tanim: String,
    pub birim: String,
    pub birim_fiyat: f64,
    pub girdiler: Vec<AnalizGirdisi>,
}

/// Metrajı Excel dosyasına aktarır (Yaklaşık Maliyet + Metraj Cetveli + Analiz Föyleri).
pub fn metraj_excel_aktar(
    metraj: &KayitliMetraj,
    analizler: &[AnalizFoyu],
    dosya_yolu: &Path,
) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name("Yaklaşık Maliyet")
        .map_err(|e| e.to_string())?;

    // Başlık formatı
    let baslik_format = Format::new()
        .set_bold()
        .set_font_size(14)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x2C3E50))
        .set_border(FormatBorder::Thin)
        .set_text_wrap();

    // Sütun başlık formatı
    let sutun_format = Format::new()
        .set_bold()
        .set_font_size(11)
        .set_background_color(Color::RGB(0x34495E))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    // Veri formatı
    let metin_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();

    let sayi_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");

    let tutar_format = Format::new()
        .set_font_size(10)
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00")
        .set_background_color(Color::RGB(0xD5F5E3));

    let grup_baslik_format = Format::new()
        .set_bold()
        .set_font_size(11)
        .set_background_color(Color::RGB(0xEAEDED))
        .set_border(FormatBorder::Thin);

    let grup_alt_toplam_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_background_color(Color::RGB(0xF2F4F4))
        .set_num_format("#,##0.00")
        .set_border(FormatBorder::Thin);

    let toplam_format = Format::new()
        .set_font_size(12)
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00")
        .set_background_color(Color::RGB(0x27AE60))
        .set_font_color(Color::White);

    let gizli_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_font_color(Color::RGB(0xB03A2E))
        .set_align(FormatAlign::Center);

    let imza_baslik_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_align(FormatAlign::Center);

    let imza_format = Format::new()
        .set_font_size(10)
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();

    // Başlık bloğu — üst başlıkta işin adı (yoksa proje adı) kullanılır
    let ust_baslik = if metraj.proje_bilgi.is_adi.trim().is_empty() {
        metraj.ad.clone()
    } else {
        metraj.proje_bilgi.is_adi.clone()
    };
    worksheet
        .merge_range(
            0,
            0,
            0,
            6,
            &format!("{} — YAKLAŞIK MALİYET HESAP CETVELİ", ust_baslik),
            &baslik_format,
        )
        .map_err(|e| e.to_string())?;
    worksheet.set_row_height(0, 30).map_err(|e| e.to_string())?;

    // Proje künyesi (dolu ise): idare, İKN, iş yeri, tür, yüklenici/sözleşme
    let mut ust = 1u32;
    let pb = &metraj.proje_bilgi;
    if pb.dolu_mu() {
        worksheet
            .merge_range(
                ust,
                0,
                ust,
                3,
                &format!("İdarenin Adı: {}", pb.idare_adi),
                &metin_format,
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .merge_range(
                ust,
                4,
                ust,
                6,
                &format!("İşin Yeri: {}", pb.is_yeri),
                &metin_format,
            )
            .map_err(|e| e.to_string())?;
        ust += 1;
        worksheet
            .merge_range(
                ust,
                0,
                ust,
                3,
                &format!("İhale Kayıt No: {}", pb.ihale_kayit_no),
                &metin_format,
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .merge_range(
                ust,
                4,
                ust,
                6,
                &format!("İşin Türü: {}", pb.is_turu),
                &metin_format,
            )
            .map_err(|e| e.to_string())?;
        ust += 1;
        if !pb.yuklenici.trim().is_empty()
            || !pb.sozlesme_no.trim().is_empty()
            || !pb.sozlesme_tarihi.trim().is_empty()
        {
            let sozlesme = format!("Sözleşme: {} {}", pb.sozlesme_no, pb.sozlesme_tarihi);
            worksheet
                .merge_range(
                    ust,
                    0,
                    ust,
                    3,
                    &format!("Yüklenici: {}", pb.yuklenici),
                    &metin_format,
                )
                .map_err(|e| e.to_string())?;
            worksheet
                .merge_range(ust, 4, ust, 6, sozlesme.trim(), &metin_format)
                .map_err(|e| e.to_string())?;
            ust += 1;
        }
    }

    let hesap_turu_metni = if metraj.hesap_turu.kamu_mu() {
        "Kamu (KDV Hariç)"
    } else {
        "Özel (KDV Dahil)"
    };
    worksheet
        .merge_range(
            ust,
            0,
            ust,
            6,
            &format!(
                "Tarih: {}        Hesap Türü: {}",
                metraj.tarih, hesap_turu_metni
            ),
            &metin_format,
        )
        .map_err(|e| e.to_string())?;
    ust += 1;
    worksheet
        .merge_range(
            ust,
            0,
            ust,
            6,
            "⚠ GİZLİDİR — İhale onay belgesine esas yaklaşık maliyettir; isteklilere açıklanmaz.",
            &gizli_format,
        )
        .map_err(|e| e.to_string())?;
    ust += 1;

    // Sütun başlıkları (künyeye göre kayan satır)
    let baslik_satir = ust;
    worksheet
        .write_with_format(baslik_satir, 0, "Sıra No", &sutun_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(baslik_satir, 1, "Poz No", &sutun_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(baslik_satir, 2, "Açıklama", &sutun_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(baslik_satir, 3, "Birim", &sutun_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(baslik_satir, 4, "Birim Fiyat (TL)", &sutun_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(baslik_satir, 5, "Miktar", &sutun_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(baslik_satir, 6, "Tutar (TL)", &sutun_format)
        .map_err(|e| e.to_string())?;

    worksheet
        .set_column_width(0, 8)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(1, 14)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(2, 55)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(3, 10)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(4, 15)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(5, 12)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(6, 15)
        .map_err(|e| e.to_string())?;

    let mut satir = baslik_satir + 1;

    #[allow(clippy::too_many_arguments)]
    fn grup_yaz(
        worksheet: &mut Worksheet,
        grup: &crate::models::IsGrubu,
        satir: &mut u32,
        prefix: &str,
        grup_baslik_format: &Format,
        grup_alt_toplam_format: &Format,
        metin_format: &Format,
        sayi_format: &Format,
        tutar_format: &Format,
    ) -> Result<(), String> {
        let baslik = if prefix.is_empty() {
            grup.ad.clone()
        } else {
            format!("{}. {}", prefix, grup.ad)
        };
        worksheet
            .merge_range(*satir, 0, *satir, 5, &baslik, grup_baslik_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_with_format(*satir, 6, "", grup_baslik_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .set_row_height(*satir, 24)
            .map_err(|e| e.to_string())?;
        *satir += 1;

        for (idx, kalem) in grup.kalemler.iter().enumerate() {
            worksheet
                .write_with_format(*satir, 0, (idx + 1) as u32, metin_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(*satir, 1, &kalem.poz_no, metin_format)
                .map_err(|e| e.to_string())?;
            let aciklama = if kalem.imalat_cinsi.trim().is_empty() {
                kalem.tanim.clone()
            } else {
                format!("{} — {}", kalem.imalat_cinsi, kalem.tanim)
            };
            worksheet
                .write_with_format(*satir, 2, &aciklama, metin_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(*satir, 3, &kalem.birim, metin_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(*satir, 4, kalem.birim_fiyat, sayi_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(*satir, 5, kalem.miktar, sayi_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(*satir, 6, kalem.tutar, tutar_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .set_row_height(*satir, 22)
                .map_err(|e| e.to_string())?;
            *satir += 1;
        }

        for (idx, alt) in grup.alt_gruplar.iter().enumerate() {
            let yeni_prefix = if prefix.is_empty() {
                format!("{}", idx + 1)
            } else {
                format!("{}.{}", prefix, idx + 1)
            };
            grup_yaz(
                worksheet,
                alt,
                satir,
                &yeni_prefix,
                grup_baslik_format,
                grup_alt_toplam_format,
                metin_format,
                sayi_format,
                tutar_format,
            )?;
        }

        let alt_toplam_etiketi = format!("{} ALT TOPLAMI", grup.ad.to_uppercase());
        worksheet
            .merge_range(
                *satir,
                0,
                *satir,
                5,
                &alt_toplam_etiketi,
                grup_alt_toplam_format,
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_with_format(*satir, 6, grup.toplam_tutar(), grup_alt_toplam_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .set_row_height(*satir, 24)
            .map_err(|e| e.to_string())?;
        *satir += 1;

        Ok(())
    }

    if !metraj.is_gruplari.is_empty() {
        for (idx, grup) in metraj.is_gruplari.iter().enumerate() {
            let prefix = format!("{}", idx + 1);
            grup_yaz(
                worksheet,
                grup,
                &mut satir,
                &prefix,
                &grup_baslik_format,
                &grup_alt_toplam_format,
                &metin_format,
                &sayi_format,
                &tutar_format,
            )?;
        }
    } else {
        // Eski flat liste
        for (idx, kalem) in metraj.kalemler.iter().enumerate() {
            worksheet
                .write_with_format(satir, 0, (idx + 1) as u32, &metin_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(satir, 1, &kalem.poz_no, &metin_format)
                .map_err(|e| e.to_string())?;
            let aciklama = if kalem.imalat_cinsi.trim().is_empty() {
                kalem.tanim.clone()
            } else {
                format!("{} — {}", kalem.imalat_cinsi, kalem.tanim)
            };
            worksheet
                .write_with_format(satir, 2, &aciklama, &metin_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(satir, 3, &kalem.birim, &metin_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(satir, 4, kalem.birim_fiyat, &sayi_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(satir, 5, kalem.miktar, &sayi_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(satir, 6, kalem.tutar, &tutar_format)
                .map_err(|e| e.to_string())?;
            worksheet
                .set_row_height(satir, 22)
                .map_err(|e| e.to_string())?;
            satir += 1;
        }
    }

    // Ara toplam satırı
    satir += 1;
    let ara_toplam = metraj.toplam_tutar();
    worksheet
        .merge_range(
            satir,
            0,
            satir,
            5,
            "ARA TOPLAM (İşçilik + Malzeme)",
            &grup_alt_toplam_format,
        )
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(satir, 6, ara_toplam, &grup_alt_toplam_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_row_height(satir, 24)
        .map_err(|e| e.to_string())?;

    // Yaklaşık maliyet özeti: genel gider & kâr, KDV (tek kaynak: maliyet::MaliyetOzeti)
    let ozet = MaliyetOzeti::hesapla(
        ara_toplam,
        metraj.genel_gider_kar_orani,
        metraj.kdv_orani,
        metraj.hesap_turu,
    );

    // Özet satırları hesap türüne göre: kâr 0 ise kâr satırı yok; Kamu'da KDV satırları yok.
    let mut ozet_satirlari: Vec<(String, f64)> = Vec::new();
    if !metraj.hesap_turu.kamu_mu() && metraj.genel_gider_kar_orani > 0.0 {
        ozet_satirlari.push((
            format!(
                "Genel Gider & Müteahhit Kârı (% {:.1})",
                metraj.genel_gider_kar_orani
            ),
            ozet.kar,
        ));
    }
    if !metraj.hesap_turu.kamu_mu() {
        ozet_satirlari.push(("KDV Matrahı".to_string(), ozet.kdv_matrahi));
        ozet_satirlari.push((format!("KDV (% {:.1})", metraj.kdv_orani), ozet.kdv));
    }

    for (etiket, deger) in ozet_satirlari {
        satir += 1;
        worksheet
            .merge_range(satir, 0, satir, 5, &etiket, &grup_alt_toplam_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_with_format(satir, 6, deger, &grup_alt_toplam_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .set_row_height(satir, 22)
            .map_err(|e| e.to_string())?;
    }

    // Toplam yaklaşık maliyet satırı (Kamu: KDV hariç)
    satir += 1;
    let toplam_baslik = if metraj.hesap_turu.kamu_mu() {
        "TOPLAM YAKLAŞIK MALİYET (KDV Hariç)"
    } else {
        "TOPLAM YAKLAŞIK MALİYET (KDV Dahil)"
    };
    worksheet
        .merge_range(satir, 0, satir, 5, toplam_baslik, &toplam_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(satir, 6, ozet.genel_toplam, &toplam_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_row_height(satir, 28)
        .map_err(|e| e.to_string())?;

    // İmza bloğu (Düzenleyen / Kontrol Eden / Onaylayan)
    satir += 3;
    for (bas, son, unvan) in [
        (0u16, 2u16, "Düzenleyen"),
        (3, 4, "Kontrol Eden"),
        (5, 6, "Onaylayan"),
    ] {
        worksheet
            .merge_range(satir, bas, satir, son, unvan, &imza_baslik_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .merge_range(
                satir + 1,
                bas,
                satir + 3,
                son,
                "Ad Soyad / Ünvan / İmza",
                &imza_format,
            )
            .map_err(|e| e.to_string())?;
    }

    // İkinci sayfa: detaylı Metraj Cetveli
    metraj_cetveli_sayfasi(&mut workbook, metraj)?;
    // Üçüncü sayfa: Pursantaj (iş grubu ağırlıkları)
    pursantaj_sayfasi(&mut workbook, metraj)?;
    // Dördüncü sayfa: Analiz Föyleri (varsa)
    analiz_foyleri_sayfasi(&mut workbook, analizler)?;

    workbook.save(dosya_yolu).map_err(|e| e.to_string())?;
    Ok(())
}

/// Pursantaj sayfası: her üst iş grubunun toplam maliyet içindeki ağırlığı (%) ve
/// kümülatif yüzde — hakediş / iş programı planlaması için.
fn pursantaj_sayfasi(workbook: &mut Workbook, metraj: &KayitliMetraj) -> Result<(), String> {
    if metraj.is_gruplari.is_empty() {
        return Ok(());
    }
    let ara_toplam = metraj.toplam_tutar();
    if ara_toplam <= 0.0 {
        return Ok(());
    }
    let ws = workbook.add_worksheet();
    ws.set_name("Pursantaj").map_err(|e| e.to_string())?;

    let baslik_format = Format::new()
        .set_bold()
        .set_font_size(14)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x2C3E50))
        .set_align(FormatAlign::Center);
    let sutun_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_background_color(Color::RGB(0x34495E))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let metin_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin);
    let sayi_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let yuzde_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_num_format("0.00\"%\"");
    let toplam_format = Format::new()
        .set_bold()
        .set_font_size(11)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x27AE60))
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");

    ws.merge_range(
        0,
        0,
        0,
        4,
        &format!("{} — PURSANTAJ (İş Grubu Ağırlıkları)", metraj.ad),
        &baslik_format,
    )
    .map_err(|e| e.to_string())?;
    ws.set_row_height(0, 28).map_err(|e| e.to_string())?;
    for (c, b) in ["Sıra", "İş Grubu", "Tutar (TL)", "Ağırlık %", "Kümülatif %"]
        .iter()
        .enumerate()
    {
        ws.write_with_format(2, c as u16, *b, &sutun_format)
            .map_err(|e| e.to_string())?;
    }
    for (i, w) in [8.0, 42.0, 18.0, 14.0, 14.0].iter().enumerate() {
        ws.set_column_width(i as u16, *w)
            .map_err(|e| e.to_string())?;
    }

    let mut satir = 3u32;
    let mut kumulatif = 0.0;
    for (idx, grup) in metraj.is_gruplari.iter().enumerate() {
        let tutar = grup.toplam_tutar();
        let yuzde = tutar / ara_toplam * 100.0;
        kumulatif += yuzde;
        ws.write_with_format(satir, 0, (idx + 1) as u32, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 1, &grup.ad, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 2, tutar, &sayi_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 3, yuzde, &yuzde_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 4, kumulatif, &yuzde_format)
            .map_err(|e| e.to_string())?;
        satir += 1;
    }
    ws.merge_range(satir, 0, satir, 1, "TOPLAM", &toplam_format)
        .map_err(|e| e.to_string())?;
    ws.write_with_format(satir, 2, ara_toplam, &toplam_format)
        .map_err(|e| e.to_string())?;
    ws.write_with_format(satir, 3, 100.0, &toplam_format)
        .map_err(|e| e.to_string())?;
    ws.write_with_format(satir, 4, "", &toplam_format)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Dördüncü Excel sayfası: Birim Fiyat Analiz Föyleri. Her analizli poz için girdiler
/// (işçilik/malzeme/makine), ara toplam, genel gider + kâr ve sonuç birim fiyat.
fn analiz_foyleri_sayfasi(workbook: &mut Workbook, analizler: &[AnalizFoyu]) -> Result<(), String> {
    if analizler.is_empty() {
        return Ok(());
    }
    let ws = workbook.add_worksheet();
    ws.set_name("Analiz Föyleri").map_err(|e| e.to_string())?;

    let foy_baslik_format = Format::new()
        .set_bold()
        .set_font_size(12)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x2C3E50))
        .set_text_wrap();
    let sutun_format = Format::new()
        .set_bold()
        .set_font_size(9)
        .set_background_color(Color::RGB(0x34495E))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let metin_format = Format::new()
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();
    let sayi_format = Format::new()
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let ara_format = Format::new()
        .set_bold()
        .set_font_size(9)
        .set_background_color(Color::RGB(0xF2F4F4))
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let sonuc_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x27AE60))
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");

    for (i, w) in [14.0, 40.0, 8.0, 14.0, 12.0, 14.0, 12.0].iter().enumerate() {
        ws.set_column_width(i as u16, *w)
            .map_err(|e| e.to_string())?;
    }

    let mut satir = 0u32;
    for foy in analizler {
        ws.merge_range(
            satir,
            0,
            satir,
            6,
            &format!("ANALİZ  —  {} : {} ({})", foy.poz_no, foy.tanim, foy.birim),
            &foy_baslik_format,
        )
        .map_err(|e| e.to_string())?;
        ws.set_row_height(satir, 26).map_err(|e| e.to_string())?;
        satir += 1;

        for (c, b) in [
            "Girdi No",
            "Tanım",
            "Birim",
            "Birim Fiyat",
            "Miktar",
            "Tutar",
            "Tür",
        ]
        .iter()
        .enumerate()
        {
            ws.write_with_format(satir, c as u16, *b, &sutun_format)
                .map_err(|e| e.to_string())?;
        }
        satir += 1;

        let mut ara_toplam = 0.0;
        for g in &foy.girdiler {
            let tutar = g.miktar * g.birim_fiyat;
            ara_toplam += tutar;
            ws.write_with_format(satir, 0, &g.girdi_no, &metin_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 1, &g.tanim, &metin_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 2, &g.birim, &metin_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 3, g.birim_fiyat, &sayi_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 4, g.miktar, &sayi_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 5, tutar, &sayi_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 6, &g.tur, &metin_format)
                .map_err(|e| e.to_string())?;
            satir += 1;
        }

        // Ara toplam
        ws.merge_range(satir, 0, satir, 4, "ARA TOPLAM", &ara_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 5, ara_toplam, &ara_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 6, "", &ara_format)
            .map_err(|e| e.to_string())?;
        satir += 1;

        // Genel gider + kâr (birim fiyattan geri hesap)
        let kar = foy.birim_fiyat - ara_toplam;
        let kar_yuzde = if ara_toplam > 0.0 {
            (foy.birim_fiyat / ara_toplam - 1.0) * 100.0
        } else {
            0.0
        };
        ws.merge_range(
            satir,
            0,
            satir,
            4,
            &format!("Genel Gider + Müteahhit Kârı (% {:.1})", kar_yuzde),
            &ara_format,
        )
        .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 5, kar, &ara_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 6, "", &ara_format)
            .map_err(|e| e.to_string())?;
        satir += 1;

        // Sonuç birim fiyat
        ws.merge_range(
            satir,
            0,
            satir,
            4,
            &format!("SONUÇ — {} BİRİM FİYATI", foy.birim),
            &sonuc_format,
        )
        .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 5, foy.birim_fiyat, &sonuc_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 6, "", &sonuc_format)
            .map_err(|e| e.to_string())?;
        satir += 2; // föyler arası boşluk
    }

    Ok(())
}

/// İkinci Excel sayfası: resmî Metraj Cetveli. Her iş kalemi için imalat cinsi ve
/// altında ölçü (boyut) kırılımı; çıkan satırlar negatif miktarla gösterilir.
fn metraj_cetveli_sayfasi(workbook: &mut Workbook, metraj: &KayitliMetraj) -> Result<(), String> {
    let ws = workbook.add_worksheet();
    ws.set_name("Metraj Cetveli").map_err(|e| e.to_string())?;

    let baslik_format = Format::new()
        .set_bold()
        .set_font_size(14)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x2C3E50))
        .set_align(FormatAlign::Center);
    let sutun_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_background_color(Color::RGB(0x34495E))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_text_wrap();
    let kalem_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_background_color(Color::RGB(0xEAEDED))
        .set_border(FormatBorder::Thin)
        .set_text_wrap();
    let kalem_miktar_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_background_color(Color::RGB(0xEAEDED))
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.000");
    let metin_format = Format::new()
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();
    let sayi_format = Format::new()
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let miktar_format = Format::new()
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.000");

    ws.merge_range(
        0,
        0,
        0,
        9,
        &format!("{} — METRAJ CETVELİ", metraj.ad),
        &baslik_format,
    )
    .map_err(|e| e.to_string())?;
    ws.set_row_height(0, 28).map_err(|e| e.to_string())?;

    let basliklar = [
        "Sıra",
        "Poz No",
        "İmalatın Cinsi / Ölçü",
        "Adet",
        "En",
        "Boy",
        "Yük.",
        "Çıkan",
        "Miktar",
        "Birim",
    ];
    for (i, b) in basliklar.iter().enumerate() {
        ws.write_with_format(2, i as u16, *b, &sutun_format)
            .map_err(|e| e.to_string())?;
    }
    for (i, w) in [6.0, 14.0, 46.0, 8.0, 8.0, 8.0, 8.0, 9.0, 12.0, 8.0]
        .iter()
        .enumerate()
    {
        ws.set_column_width(i as u16, *w)
            .map_err(|e| e.to_string())?;
    }

    fn yaz_opt(
        ws: &mut Worksheet,
        satir: u32,
        col: u16,
        o: Option<f64>,
        sayi_fmt: &Format,
        metin_fmt: &Format,
    ) -> Result<(), String> {
        match o {
            Some(v) => ws
                .write_with_format(satir, col, v, sayi_fmt)
                .map(|_| ())
                .map_err(|e| e.to_string()),
            None => ws
                .write_with_format(satir, col, "", metin_fmt)
                .map(|_| ())
                .map_err(|e| e.to_string()),
        }
    }

    let mut satir = 3u32;
    for (idx, kalem) in metraj.kalemler.iter().enumerate() {
        // İş kalemi başlık satırı
        let cins = if kalem.imalat_cinsi.trim().is_empty() {
            kalem.tanim.clone()
        } else {
            format!("{} — {}", kalem.imalat_cinsi, kalem.tanim)
        };
        ws.write_with_format(satir, 0, (idx + 1) as u32, &kalem_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 1, &kalem.poz_no, &kalem_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 2, &cins, &kalem_format)
            .map_err(|e| e.to_string())?;
        for c in 3..8u16 {
            ws.write_with_format(satir, c, "", &kalem_format)
                .map_err(|e| e.to_string())?;
        }
        ws.write_with_format(satir, 8, kalem.miktar, &kalem_miktar_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 9, &kalem.birim, &kalem_format)
            .map_err(|e| e.to_string())?;
        satir += 1;

        // Ölçü (boyut) kırılımı
        for d in &kalem.detaylar {
            ws.write_with_format(satir, 0, "", &metin_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 1, "", &metin_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 2, &d.aciklama, &metin_format)
                .map_err(|e| e.to_string())?;
            yaz_opt(ws, satir, 3, d.adet, &sayi_format, &metin_format)?;
            yaz_opt(ws, satir, 4, d.en, &sayi_format, &metin_format)?;
            yaz_opt(ws, satir, 5, d.boy, &sayi_format, &metin_format)?;
            yaz_opt(ws, satir, 6, d.yukseklik, &sayi_format, &metin_format)?;
            ws.write_with_format(
                satir,
                7,
                if d.cikan { "çıkan (−)" } else { "" },
                &metin_format,
            )
            .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 8, d.hesaplanan_miktar(), &miktar_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 9, "", &metin_format)
                .map_err(|e| e.to_string())?;
            satir += 1;
        }
    }

    Ok(())
}

// ==================== CSV (Excel gidiş-geliş) ====================
fn csv_temiz(s: &str) -> String {
    s.replace(';', ",").replace(['\n', '\r'], " ")
}

/// Metrajı CSV'ye yazar. Türk Excel uyumu: `;` ayraç, virgül ondalık.
/// Sütun sırası içe aktarımla uyumludur (Poz No, Miktar önce).
pub fn metraj_csv_aktar(metraj: &KayitliMetraj, dosya_yolu: &Path) -> Result<(), String> {
    let mut s = String::from("Poz No;Miktar;İmalat Cinsi;Açıklama;Birim;Birim Fiyat;Tutar\n");
    for kalem in &metraj.kalemler {
        s.push_str(&format!(
            "{};{};{};{};{};{};{}\n",
            csv_temiz(&kalem.poz_no),
            format!("{:.3}", kalem.miktar).replace('.', ","),
            csv_temiz(&kalem.imalat_cinsi),
            csv_temiz(&kalem.tanim),
            csv_temiz(&kalem.birim),
            format!("{:.2}", kalem.birim_fiyat).replace('.', ","),
            format!("{:.2}", kalem.tutar).replace('.', ","),
        ));
    }
    std::fs::write(dosya_yolu, s).map_err(|e| format!("CSV yazılamadı: {}", e))
}

/// CSV'den (poz_no, miktar, imalat_cinsi) satırlarını okur. Beklenen sütunlar:
/// `Poz No; Miktar; [İmalat Cinsi; …]`. İlk satır başlıksa atlanır; fazla sütun yok sayılır.
pub fn metraj_csv_oku(dosya_yolu: &Path) -> Result<Vec<(String, f64, String)>, String> {
    let icerik =
        std::fs::read_to_string(dosya_yolu).map_err(|e| format!("CSV okunamadı: {}", e))?;
    let mut sonuc = Vec::new();
    for satir in icerik.lines() {
        let satir = satir.trim();
        if satir.is_empty() {
            continue;
        }
        let alanlar: Vec<&str> = satir.split(';').collect();
        let poz_no = alanlar[0].trim().trim_matches('"').to_string();
        if poz_no.is_empty() {
            continue;
        }
        // Başlık / sayısal olmayan miktar satırını atla
        let miktar = match alanlar.get(1).and_then(|a| crate::bicim::sayi_oku(a)) {
            Some(m) => m,
            None => continue,
        };
        let imalat = alanlar
            .get(2)
            .map(|a| a.trim().trim_matches('"').to_string())
            .unwrap_or_default();
        sonuc.push((poz_no, miktar, imalat));
    }
    Ok(sonuc)
}

// ==================== HAKEDİŞ RAPORU ====================
/// Bir hakedişi resmî hakediş raporu olarak Excel'e aktarır: poz bazında kümülatif /
/// bu hakediş miktar ve tutarları + kesintiler + net ödeme.
pub fn hakedis_excel_aktar(
    proje_adi: &str,
    pb: &ProjeBilgi,
    kesif: &[MetrajKalemi],
    hakedis: &Hakedis,
    onceki: Option<&Hakedis>,
    sozlesme: &SozlesmeAyarlari,
    dosya_yolu: &Path,
) -> Result<(), String> {
    let hesaplar = crate::hakedis::poz_hesaplari(kesif, hakedis, onceki, sozlesme.tenzilat_orani());
    let ic = crate::hakedis::icmal(&hesaplar, hakedis);

    let mut workbook = Workbook::new();
    let ws = workbook.add_worksheet();
    ws.set_name(format!("Hakediş {}", hakedis.no))
        .map_err(|e| e.to_string())?;

    let baslik_format = Format::new()
        .set_bold()
        .set_font_size(14)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x2C3E50))
        .set_align(FormatAlign::Center);
    let meta_format = Format::new()
        .set_font_size(11)
        .set_border(FormatBorder::Thin);
    let sutun_format = Format::new()
        .set_bold()
        .set_font_size(9)
        .set_background_color(Color::RGB(0x34495E))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_text_wrap();
    let metin_format = Format::new()
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();
    let sayi_format = Format::new()
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let miktar_format = Format::new()
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.000");
    let tutar_format = Format::new()
        .set_font_size(9)
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00")
        .set_background_color(Color::RGB(0xD5F5E3));
    let icmal_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_background_color(Color::RGB(0xF2F4F4))
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let net_format = Format::new()
        .set_bold()
        .set_font_size(12)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x27AE60))
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");

    let ust_baslik = if pb.is_adi.trim().is_empty() {
        proje_adi.to_string()
    } else {
        pb.is_adi.clone()
    };
    ws.merge_range(
        0,
        0,
        0,
        10,
        &format!("{} — {}. HAKEDİŞ ({})", ust_baslik, hakedis.no, hakedis.tur),
        &baslik_format,
    )
    .map_err(|e| e.to_string())?;
    ws.set_row_height(0, 28).map_err(|e| e.to_string())?;

    let mut ust = 1u32;
    if pb.dolu_mu() {
        ws.merge_range(
            ust,
            0,
            ust,
            5,
            &format!("İdarenin Adı: {}", pb.idare_adi),
            &meta_format,
        )
        .map_err(|e| e.to_string())?;
        ws.merge_range(
            ust,
            6,
            ust,
            10,
            &format!("İhale Kayıt No: {}", pb.ihale_kayit_no),
            &meta_format,
        )
        .map_err(|e| e.to_string())?;
        ust += 1;
        if !pb.yuklenici.trim().is_empty() || !pb.sozlesme_no.trim().is_empty() {
            let sozlesme = format!("Sözleşme: {} {}", pb.sozlesme_no, pb.sozlesme_tarihi);
            ws.merge_range(
                ust,
                0,
                ust,
                5,
                &format!("Yüklenici: {}", pb.yuklenici),
                &meta_format,
            )
            .map_err(|e| e.to_string())?;
            ws.merge_range(ust, 6, ust, 10, sozlesme.trim(), &meta_format)
                .map_err(|e| e.to_string())?;
            ust += 1;
        }
    }
    ws.merge_range(
        ust,
        0,
        ust,
        10,
        &format!("Tarih: {}", hakedis.tarih),
        &meta_format,
    )
    .map_err(|e| e.to_string())?;
    ust += 1;

    let basliklar = [
        "Sıra",
        "Poz No",
        "Açıklama",
        "Birim",
        "B.Fiyat",
        "Sözleşme Mik.",
        "Önceki Küm.",
        "Bu Küm.",
        "Bu Hakediş Mik.",
        "Bu Hakediş Tutar",
        "Sözleşme Farkı",
    ];
    let baslik_satir = ust;
    for (c, b) in basliklar.iter().enumerate() {
        ws.write_with_format(baslik_satir, c as u16, *b, &sutun_format)
            .map_err(|e| e.to_string())?;
    }
    for (i, w) in [
        6.0, 14.0, 38.0, 8.0, 12.0, 12.0, 12.0, 12.0, 13.0, 15.0, 13.0,
    ]
    .iter()
    .enumerate()
    {
        ws.set_column_width(i as u16, *w)
            .map_err(|e| e.to_string())?;
    }

    let mut satir = baslik_satir + 1;
    for (idx, h) in hesaplar.iter().enumerate() {
        ws.write_with_format(satir, 0, (idx + 1) as u32, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 1, &h.poz_no, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 2, &h.tanim, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 3, &h.birim, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 4, h.birim_fiyat, &sayi_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 5, h.sozlesme_miktar, &miktar_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 6, h.onceki_kumulatif, &miktar_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 7, h.kumulatif, &miktar_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 8, h.bu_hakedis_miktar, &miktar_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 9, h.bu_hakedis_tutar, &tutar_format)
            .map_err(|e| e.to_string())?;
        // Sözleşme farkı: kümülatif − sözleşme (kesin hesap; + fazla, − eksik imalat)
        ws.write_with_format(satir, 10, h.kumulatif - h.sozlesme_miktar, &miktar_format)
            .map_err(|e| e.to_string())?;
        satir += 1;
    }

    // İcmal
    satir += 1;
    let icmal_satiri = |ws: &mut Worksheet,
                        satir: &mut u32,
                        etiket: &str,
                        deger: f64,
                        fmt: &Format|
     -> Result<(), String> {
        ws.merge_range(*satir, 0, *satir, 9, etiket, fmt)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(*satir, 10, deger, fmt)
            .map_err(|e| e.to_string())?;
        *satir += 1;
        Ok(())
    };
    icmal_satiri(
        ws,
        &mut satir,
        "Bu Hakediş Ham İmalat",
        ic.bu_hakedis_ham,
        &icmal_format,
    )?;
    if ic.tenzilat_tutari != 0.0 {
        icmal_satiri(
            ws,
            &mut satir,
            &format!("Tenzilat (% {:.6})", sozlesme.tenzilat_orani()),
            -ic.tenzilat_tutari,
            &icmal_format,
        )?;
    }
    icmal_satiri(
        ws,
        &mut satir,
        "Bu Hakediş (Tenzilat Sonrası)",
        ic.bu_hakedis_brut,
        &icmal_format,
    )?;
    if ic.fiyat_farki != 0.0 {
        icmal_satiri(
            ws,
            &mut satir,
            "Fiyat Farkı (±)",
            ic.fiyat_farki,
            &icmal_format,
        )?;
    }
    icmal_satiri(ws, &mut satir, "Tahakkuk Eden", ic.tahakkuk, &icmal_format)?;
    icmal_satiri(
        ws,
        &mut satir,
        &format!("Damga Vergisi (‰ {:.2})", hakedis.damga_orani),
        -ic.damga,
        &icmal_format,
    )?;
    if ic.teminat != 0.0 {
        icmal_satiri(
            ws,
            &mut satir,
            &format!("Teminat Kesintisi (% {:.1})", hakedis.teminat_orani),
            -ic.teminat,
            &icmal_format,
        )?;
    }
    if ic.sgk != 0.0 {
        icmal_satiri(
            ws,
            &mut satir,
            &format!("SGK (% {:.1})", hakedis.sgk_orani),
            -ic.sgk,
            &icmal_format,
        )?;
    }
    if ic.avans_mahsup != 0.0 {
        icmal_satiri(
            ws,
            &mut satir,
            "Avans Mahsubu",
            -ic.avans_mahsup,
            &icmal_format,
        )?;
    }
    icmal_satiri(
        ws,
        &mut satir,
        "KDV Hariç Net Tahakkuk",
        ic.net_odeme,
        &icmal_format,
    )?;
    if ic.kdv > 0.0 {
        icmal_satiri(
            ws,
            &mut satir,
            &format!("KDV (% {:.0})", hakedis.kdv_orani),
            ic.kdv,
            &icmal_format,
        )?;
    }
    if ic.tevkifat != 0.0 {
        icmal_satiri(
            ws,
            &mut satir,
            &format!("KDV Tevkifatı (× {:.2})", hakedis.tevkifat_orani),
            -ic.tevkifat,
            &icmal_format,
        )?;
    }
    icmal_satiri(
        ws,
        &mut satir,
        "ÖDENECEK TUTAR",
        ic.odenecek_tutar,
        &net_format,
    )?;
    ws.set_row_height(satir - 1, 26)
        .map_err(|e| e.to_string())?;

    // Kesin hesap: sözleşme bedeli vs gerçekleşen
    let sozlesme_bedeli = crate::bicim::kurus_yuvarla(
        hesaplar
            .iter()
            .map(|h| h.birim_fiyat * h.sozlesme_miktar)
            .sum(),
    );
    satir += 1;
    icmal_satiri(
        ws,
        &mut satir,
        "Sözleşme Bedeli",
        sozlesme_bedeli,
        &icmal_format,
    )?;
    icmal_satiri(
        ws,
        &mut satir,
        "Gerçekleşen (Kümülatif Brüt)",
        ic.kumulatif_brut,
        &icmal_format,
    )?;
    icmal_satiri(
        ws,
        &mut satir,
        "Sözleşme Farkı (+ fazla / − eksik)",
        ic.kumulatif_brut - sozlesme_bedeli,
        &icmal_format,
    )?;

    // İmza blokları
    satir += 2;
    let imza_baslik = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_align(FormatAlign::Center);
    let imza = Format::new()
        .set_font_size(10)
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();
    for (bas, son, unvan) in [
        (1u16, 3u16, "Düzenleyen"),
        (4, 6, "Kontrol Eden"),
        (7, 9, "Onaylayan"),
    ] {
        ws.merge_range(satir, bas, satir, son, unvan, &imza_baslik)
            .map_err(|e| e.to_string())?;
        ws.merge_range(
            satir + 1,
            bas,
            satir + 3,
            son,
            "Ad Soyad / Ünvan / İmza",
            &imza,
        )
        .map_err(|e| e.to_string())?;
    }

    workbook.save(dosya_yolu).map_err(|e| e.to_string())?;
    Ok(())
}

/// Pursantajlı iş programını Excel'e aktarır: aylık dağılım + kümülatif ilerleme (S) eğrisi.
pub fn is_programi_excel_aktar(
    proje_adi: &str,
    pb: &ProjeBilgi,
    toplam_bedel: f64,
    prog: &IsProgrami,
    dosya_yolu: &Path,
) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let ws = workbook.add_worksheet();
    ws.set_name("İş Programı").map_err(|e| e.to_string())?;

    let baslik_format = Format::new()
        .set_bold()
        .set_font_size(14)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x2C3E50))
        .set_align(FormatAlign::Center);
    let meta_format = Format::new()
        .set_font_size(11)
        .set_border(FormatBorder::Thin);
    let sutun_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_background_color(Color::RGB(0x34495E))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_text_wrap();
    let metin_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin);
    let yuzde_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_num_format("0.00\"%\"");
    let tutar_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let toplam_format = Format::new()
        .set_bold()
        .set_font_size(11)
        .set_background_color(Color::RGB(0xD5F5E3))
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let toplam_yuzde = Format::new()
        .set_bold()
        .set_font_size(11)
        .set_background_color(Color::RGB(0xD5F5E3))
        .set_border(FormatBorder::Thin)
        .set_num_format("0.00\"%\"");

    let ust_baslik = if pb.is_adi.trim().is_empty() {
        proje_adi.to_string()
    } else {
        pb.is_adi.clone()
    };
    ws.merge_range(
        0,
        0,
        0,
        5,
        &format!("{} — İŞ PROGRAMI (Pursantaj Cetveli)", ust_baslik),
        &baslik_format,
    )
    .map_err(|e| e.to_string())?;
    ws.set_row_height(0, 28).map_err(|e| e.to_string())?;
    ws.merge_range(
        1,
        0,
        1,
        5,
        &format!(
            "Sözleşme Bedeli: {:.2} TL   |   Süre: {} ay   |   Başlangıç: {} {}",
            toplam_bedel,
            prog.sure_ay,
            ay_adi(prog.baslangic_ay),
            prog.baslangic_yil
        ),
        &meta_format,
    )
    .map_err(|e| e.to_string())?;
    if pb.dolu_mu() {
        ws.merge_range(
            2,
            0,
            2,
            5,
            &format!(
                "İdarenin Adı: {}   |   İhale Kayıt No: {}",
                pb.idare_adi, pb.ihale_kayit_no
            ),
            &meta_format,
        )
        .map_err(|e| e.to_string())?;
    }

    let basliklar = [
        "Ay",
        "Dönem",
        "Pursantaj (%)",
        "Aylık Tutar (TL)",
        "Kümülatif (%)",
        "Kümülatif Tutar (TL)",
    ];
    for (c, b) in basliklar.iter().enumerate() {
        ws.write_with_format(3, c as u16, *b, &sutun_format)
            .map_err(|e| e.to_string())?;
    }
    for (i, w) in [6.0, 16.0, 14.0, 18.0, 14.0, 20.0].iter().enumerate() {
        ws.set_column_width(i as u16, *w)
            .map_err(|e| e.to_string())?;
    }

    let mut satir = 4u32;
    let mut kum_yuzde = 0.0;
    for (i, yuzde) in prog.dagilim.iter().enumerate() {
        let (yil, ay) = prog.ay_etiketi(i);
        kum_yuzde += *yuzde;
        let aylik_tutar = toplam_bedel * yuzde / 100.0;
        let kum_tutar = toplam_bedel * kum_yuzde / 100.0;
        ws.write_with_format(satir, 0, (i + 1) as u32, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 1, format!("{} {}", ay_adi(ay), yil), &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 2, *yuzde, &yuzde_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 3, aylik_tutar, &tutar_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 4, kum_yuzde, &yuzde_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 5, kum_tutar, &tutar_format)
            .map_err(|e| e.to_string())?;
        satir += 1;
    }

    // Toplam satırı
    ws.write_with_format(satir, 0, "", &toplam_format)
        .map_err(|e| e.to_string())?;
    ws.write_with_format(satir, 1, "TOPLAM", &toplam_format)
        .map_err(|e| e.to_string())?;
    ws.write_with_format(satir, 2, prog.toplam_yuzde(), &toplam_yuzde)
        .map_err(|e| e.to_string())?;
    ws.write_with_format(
        satir,
        3,
        toplam_bedel * prog.toplam_yuzde() / 100.0,
        &toplam_format,
    )
    .map_err(|e| e.to_string())?;
    ws.write_with_format(satir, 4, "", &toplam_format)
        .map_err(|e| e.to_string())?;
    ws.write_with_format(satir, 5, "", &toplam_format)
        .map_err(|e| e.to_string())?;

    workbook.save(dosya_yolu).map_err(|e| e.to_string())?;
    Ok(())
}

/// Birim Fiyat Teklif Cetveli + Teklif Mektubu (Excel, 2 sayfa).
/// `dolu` true ise proje birim fiyatları teklif olarak yazılır (isteklinin çalışma
/// kopyası); false ise birim fiyat/tutar sütunları boş bırakılır (isteklilere
/// dağıtılacak boş cetvel). Teklif bedeli KDV hariçtir.
pub fn teklif_cetveli_excel_aktar(
    metraj: &KayitliMetraj,
    dolu: bool,
    dosya_yolu: &Path,
) -> Result<(), String> {
    // Kalemleri düzleştir (iş ağacı veya düz liste).
    let kalemler: Vec<MetrajKalemi> = if metraj.is_gruplari.is_empty() {
        metraj.kalemler.clone()
    } else {
        let mut v = Vec::new();
        for g in &metraj.is_gruplari {
            v.extend(g.tum_kalemler_duz());
        }
        v
    };
    let toplam =
        crate::bicim::kurus_yuvarla(kalemler.iter().map(|k| k.miktar * k.birim_fiyat).sum());

    let mut workbook = Workbook::new();
    let ws = workbook.add_worksheet();
    ws.set_name("Teklif Cetveli").map_err(|e| e.to_string())?;

    let baslik_format = Format::new()
        .set_bold()
        .set_font_size(14)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x2C3E50))
        .set_align(FormatAlign::Center);
    let meta_format = Format::new()
        .set_font_size(11)
        .set_border(FormatBorder::Thin);
    let sutun_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_background_color(Color::RGB(0x34495E))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_text_wrap();
    let metin_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin);
    let miktar_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.000");
    let sayi_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");
    let bos_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::RGB(0xFCF3CF));
    let toplam_format = Format::new()
        .set_bold()
        .set_font_size(11)
        .set_background_color(Color::RGB(0xD5F5E3))
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");

    let pb = &metraj.proje_bilgi;
    let ust_baslik = if pb.is_adi.trim().is_empty() {
        metraj.ad.clone()
    } else {
        pb.is_adi.clone()
    };
    ws.merge_range(
        0,
        0,
        0,
        6,
        &format!("{} — BİRİM FİYAT TEKLİF CETVELİ", ust_baslik),
        &baslik_format,
    )
    .map_err(|e| e.to_string())?;
    ws.set_row_height(0, 28).map_err(|e| e.to_string())?;
    let mut ust = 1u32;
    if pb.dolu_mu() {
        ws.merge_range(
            ust,
            0,
            ust,
            3,
            &format!("İdarenin Adı: {}", pb.idare_adi),
            &meta_format,
        )
        .map_err(|e| e.to_string())?;
        ws.merge_range(
            ust,
            4,
            ust,
            6,
            &format!("İhale Kayıt No: {}", pb.ihale_kayit_no),
            &meta_format,
        )
        .map_err(|e| e.to_string())?;
        ust += 1;
    }

    let basliklar = [
        "Sıra No",
        "Poz No",
        "İş Kaleminin Adı",
        "Birimi",
        "Miktarı",
        "Teklif Edilen Birim Fiyat (TL)",
        "Tutarı (TL)",
    ];
    let baslik_satir = ust;
    for (c, b) in basliklar.iter().enumerate() {
        ws.write_with_format(baslik_satir, c as u16, *b, &sutun_format)
            .map_err(|e| e.to_string())?;
    }
    for (i, w) in [7.0, 14.0, 46.0, 9.0, 12.0, 18.0, 16.0].iter().enumerate() {
        ws.set_column_width(i as u16, *w)
            .map_err(|e| e.to_string())?;
    }

    let mut satir = baslik_satir + 1;
    for (idx, k) in kalemler.iter().enumerate() {
        let ad = if k.imalat_cinsi.trim().is_empty() {
            k.tanim.clone()
        } else {
            format!("{} — {}", k.imalat_cinsi, k.tanim)
        };
        ws.write_with_format(satir, 0, (idx + 1) as u32, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 1, &k.poz_no, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 2, &ad, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 3, &k.birim, &metin_format)
            .map_err(|e| e.to_string())?;
        ws.write_with_format(satir, 4, k.miktar, &miktar_format)
            .map_err(|e| e.to_string())?;
        if dolu {
            ws.write_with_format(satir, 5, k.birim_fiyat, &sayi_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(
                satir,
                6,
                crate::bicim::kurus_yuvarla(k.miktar * k.birim_fiyat),
                &sayi_format,
            )
            .map_err(|e| e.to_string())?;
        } else {
            // İstekli dolduracak → sarı boş hücreler
            ws.write_with_format(satir, 5, "", &bos_format)
                .map_err(|e| e.to_string())?;
            ws.write_with_format(satir, 6, "", &bos_format)
                .map_err(|e| e.to_string())?;
        }
        satir += 1;
    }

    // Toplam
    ws.merge_range(
        satir,
        0,
        satir,
        5,
        "TOPLAM TEKLİF BEDELİ (KDV Hariç)",
        &toplam_format,
    )
    .map_err(|e| e.to_string())?;
    if dolu {
        ws.write_with_format(satir, 6, toplam, &toplam_format)
            .map_err(|e| e.to_string())?;
    } else {
        ws.write_with_format(satir, 6, "", &toplam_format)
            .map_err(|e| e.to_string())?;
    }
    satir += 2;
    if dolu {
        ws.merge_range(
            satir,
            0,
            satir,
            6,
            &format!("Yazı ile: {}", crate::bicim::sayi_yaziya(toplam)),
            &meta_format,
        )
        .map_err(|e| e.to_string())?;
    }

    // ---- 2. Sayfa: Teklif Mektubu ----
    let ws2 = workbook.add_worksheet();
    ws2.set_name("Teklif Mektubu").map_err(|e| e.to_string())?;
    ws2.set_column_width(0, 100.0).map_err(|e| e.to_string())?;
    let mkt_baslik = Format::new()
        .set_bold()
        .set_font_size(13)
        .set_align(FormatAlign::Center);
    let mkt_metin = Format::new()
        .set_font_size(11)
        .set_text_wrap()
        .set_align(FormatAlign::Left);
    let mkt_vurgu = Format::new().set_bold().set_font_size(11);

    ws2.write_with_format(0, 0, "BİRİM FİYAT TEKLİF MEKTUBU", &mkt_baslik)
        .map_err(|e| e.to_string())?;
    let idare_satiri = if pb.idare_adi.trim().is_empty() {
        "…".to_string()
    } else {
        pb.idare_adi.clone()
    };
    let govde = format!(
        "{} İHALE KOMİSYONU BAŞKANLIĞINA\n\n\
         \"{}\" işine ait ihale dokümanını oluşturan bütün belgeler incelenmiş, okunmuş ve herhangi bir ayrım ve sınırlama yapılmadan bütün koşullarıyla kabul edilmiştir. İhaleye ilişkin olarak aşağıdaki hususları içeren teklifimizin kabulünü arz ederiz.\n\n\
         1) İhale Kayıt Numarası: {}\n\
         2) Yukarıda belirtilen işi, ekli birim fiyat teklif cetvelinde belirtilen her bir iş kalemi için teklif ettiğimiz birim fiyatlar üzerinden Katma Değer Vergisi hariç toplam bedel karşılığında yapmayı kabul ve taahhüt ederiz.\n\
         3) Teklifimiz ihale tarihinden itibaren geçerlidir.",
        idare_satiri, ust_baslik, if pb.ihale_kayit_no.trim().is_empty() { "…".to_string() } else { pb.ihale_kayit_no.clone() }
    );
    ws2.write_with_format(2, 0, &govde, &mkt_metin)
        .map_err(|e| e.to_string())?;
    ws2.set_row_height(2, 200).map_err(|e| e.to_string())?;

    let bedel_metni = if dolu {
        format!(
            "Teklif Edilen Toplam Bedel (KDV Hariç): {:.2} TL\nYazı ile: {}",
            toplam,
            crate::bicim::sayi_yaziya(toplam)
        )
    } else {
        "Teklif Edilen Toplam Bedel (KDV Hariç): ………………………… TL\nYazı ile: …………………………".to_string()
    };
    ws2.write_with_format(4, 0, &bedel_metni, &mkt_vurgu)
        .map_err(|e| e.to_string())?;
    ws2.set_row_height(4, 34).map_err(|e| e.to_string())?;

    let istekli = if pb.yuklenici.trim().is_empty() {
        "İstekli (Adı / Ünvanı):".to_string()
    } else {
        format!("İstekli: {}", pb.yuklenici)
    };
    ws2.write_with_format(
        6,
        0,
        format!("{}\nTarih: {}\nKaşe / İmza:", istekli, metraj.tarih),
        &mkt_metin,
    )
    .map_err(|e| e.to_string())?;
    ws2.set_row_height(6, 60).map_err(|e| e.to_string())?;

    workbook.save(dosya_yolu).map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== VERİ PAKETİ (.mvp) ====================
pub fn veri_paketi_kaydet(paket: &VeriPaketi, dosya_yolu: &Path) -> Result<(), String> {
    let json = serde_json::to_string(paket).map_err(|e| e.to_string())?;
    guvenli_metin_yaz(dosya_yolu, &json).map_err(|e| format!("Paket yazılamadı: {}", e))
}

pub fn veri_paketi_yukle(dosya_yolu: &Path) -> Result<VeriPaketi, String> {
    let json =
        std::fs::read_to_string(dosya_yolu).map_err(|e| format!("Paket okunamadı: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Paket ayrıştırılamadı: {}", e))
}

/// Metrajı JSON dosyasına kaydeder
pub fn metraj_json_kaydet(metraj: &KayitliMetraj, dosya_yolu: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(metraj).map_err(|e| e.to_string())?;
    guvenli_metin_yaz(dosya_yolu, &json).map_err(|e| format!("Dosya yazılamadı: {}", e))?;
    Ok(())
}

/// JSON dosyasından metraj yükler
pub fn metraj_json_yukle(dosya_yolu: &Path) -> Result<KayitliMetraj, String> {
    let json =
        std::fs::read_to_string(dosya_yolu).map_err(|e| format!("Dosya okunamadı: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("JSON ayrıştırılamadı: {}", e))
}

/// Metrajı panoya kopyalanabilir metin formatında döndürür
pub fn metraj_metin_ozet(metraj: &KayitliMetraj) -> String {
    let mut cikti = String::new();
    cikti.push_str(&format!("{}\n", "=".repeat(80)));
    cikti.push_str(&format!("  {} - METRAJ ÖZETİ\n", metraj.ad));
    cikti.push_str(&format!("  Tarih: {}\n", metraj.tarih));
    cikti.push_str(&format!("{}\n", "=".repeat(80)));
    cikti.push_str(&format!(
        "  {:<6} {:<14} {:<40} {:<8} {:>14} {:>12} {:>14}\n",
        "Sıra", "Poz No", "Açıklama", "Birim", "Birim Fiyat", "Miktar", "Tutar"
    ));
    cikti.push_str(&format!("{}\n", "-".repeat(80)));

    for (idx, kalem) in metraj.kalemler.iter().enumerate() {
        let tanim_kisa = crate::bicim::metni_kisalt(&kalem.tanim, 38);

        cikti.push_str(&format!(
            "  {:<6} {:<14} {:<40} {:<8} {:>14.2} {:>12.2} {:>14.2}\n",
            idx + 1,
            kalem.poz_no,
            tanim_kisa,
            kalem.birim,
            kalem.birim_fiyat,
            kalem.miktar,
            kalem.tutar
        ));
    }

    cikti.push_str(&format!("{}\n", "-".repeat(80)));
    cikti.push_str(&format!(
        "  GENEL TOPLAM: {:>14.2} TL\n",
        metraj.toplam_tutar()
    ));
    cikti.push_str(&format!("{}\n", "=".repeat(80)));

    cikti
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::models::{HesapTuru, IsGrubu, MetrajKalemi, MiktarDetay};
    use std::sync::atomic::{AtomicU32, Ordering};

    static SAYAC: AtomicU32 = AtomicU32::new(0);

    fn ornek_metraj() -> KayitliMetraj {
        let kalem = MetrajKalemi {
            id: crate::models::yeni_kalem_id(),
            poz_no: "15.150.1001".into(),
            tanim: "Beton dökülmesi".into(),
            birim: "m³".into(),
            birim_fiyat: 1000.0,
            miktar: 27.0,
            tutar: 27000.0,
            kitap_adi: "ÇŞB (5/2026)".into(),
            imalat_cinsi: "Zemin kat perde duvarları".into(),
            detaylar: vec![
                MiktarDetay {
                    aciklama: "duvar".into(),
                    miktar: 30.0,
                    adet: Some(1.0),
                    en: Some(10.0),
                    boy: Some(3.0),
                    yukseklik: None,
                    cikan: false,
                },
                MiktarDetay {
                    aciklama: "pencere".into(),
                    miktar: -3.0,
                    adet: Some(2.0),
                    en: Some(1.5),
                    boy: Some(1.0),
                    yukseklik: None,
                    cikan: true,
                },
            ],
            kitap_id: 0,
        };
        KayitliMetraj {
            ad: "Test Projesi".into(),
            kalemler: vec![kalem.clone()],
            is_gruplari: vec![IsGrubu {
                id: "g1".into(),
                ad: "İnşaat".into(),
                alt_gruplar: vec![],
                kalemler: vec![kalem],
            }],
            tarih: "2026-07-12".into(),
            genel_gider_kar_orani: 0.0,
            kdv_orani: 20.0,
            hesap_turu: HesapTuru::Kamu,
            hakedisler: vec![],
            is_programi: crate::models::IsProgrami::default(),
            proje_bilgi: crate::models::ProjeBilgi::default(),
            proje_asamasi: crate::models::ProjeAsamasi::Metraj,
            sozlesme_ayarlari: crate::models::SozlesmeAyarlari::default(),
        }
    }

    fn gecici_yol(uzanti: &str) -> std::path::PathBuf {
        let n = SAYAC.fetch_add(1, Ordering::SeqCst);
        let mut yol = std::env::temp_dir();
        yol.push(format!("mm_export_{}_{}.{}", std::process::id(), n, uzanti));
        let _ = std::fs::remove_file(&yol);
        yol
    }

    #[test]
    fn excel_uc_sayfa_analiz_foyu_ile_uretilir() {
        let yol = gecici_yol("xlsx");
        let foy = AnalizFoyu {
            poz_no: "15.150.1001".into(),
            tanim: "Beton".into(),
            birim: "m³".into(),
            birim_fiyat: 1000.0,
            girdiler: vec![
                AnalizGirdisi {
                    girdi_no: "10.100.1001".into(),
                    tanim: "işçi".into(),
                    birim: "saat".into(),
                    birim_fiyat: 100.0,
                    miktar: 2.0,
                    tur: "İşçilik".into(),
                },
                AnalizGirdisi {
                    girdi_no: "10.130".into(),
                    tanim: "çimento".into(),
                    birim: "kg".into(),
                    birim_fiyat: 5.0,
                    miktar: 50.0,
                    tur: "Malzeme".into(),
                },
            ],
        };
        metraj_excel_aktar(&ornek_metraj(), &[foy], &yol).expect("Excel üretilmeli");
        assert!(
            std::fs::metadata(&yol).unwrap().len() > 0,
            "Excel boş olmamalı"
        );
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn excel_proje_kunyesi_ile_hatasiz_uretilir() {
        // Künye dolu iken başlık satırları kayar (baslik_satir dinamik) — hata vermemeli.
        let yol = gecici_yol("xlsx");
        let mut m = ornek_metraj();
        m.proje_bilgi = crate::models::ProjeBilgi {
            idare_adi: "Test Belediyesi".into(),
            is_adi: "24 Derslikli Okul".into(),
            is_yeri: "Ankara".into(),
            ihale_kayit_no: "2026/123456".into(),
            is_turu: "Yapım".into(),
            yuklenici: "Örnek İnş. Ltd.".into(),
            sozlesme_no: "S-2026-1".into(),
            sozlesme_tarihi: "01.02.2026".into(),
        };
        assert!(m.proje_bilgi.dolu_mu());
        metraj_excel_aktar(&m, &[], &yol).expect("Künyeli Excel üretilmeli");
        assert!(
            std::fs::metadata(&yol).unwrap().len() > 0,
            "Excel boş olmamalı"
        );
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn json_kaydet_yukle_roundtrip() {
        let yol = gecici_yol("mrj");
        let m = ornek_metraj();
        metraj_json_kaydet(&m, &yol).expect("kaydedilmeli");
        let okunan = metraj_json_yukle(&yol).expect("okunmalı");
        assert_eq!(okunan.ad, "Test Projesi");
        assert_eq!(okunan.hesap_turu, HesapTuru::Kamu);
        assert_eq!(okunan.kalemler.len(), 1);
        assert_eq!(okunan.kalemler[0].imalat_cinsi, "Zemin kat perde duvarları");
        assert!(okunan.kalemler[0].detaylar[1].cikan);
        let _ = std::fs::remove_file(&yol);
        let _ = std::fs::remove_file(ekli_yol(&yol, ".bak"));
    }

    #[test]
    fn ikinci_json_kaydi_onceki_surumu_yedekte_korur() {
        let yol = gecici_yol("mrj");
        let mut ilk = ornek_metraj();
        ilk.ad = "İlk sürüm".into();
        metraj_json_kaydet(&ilk, &yol).expect("ilk sürüm kaydedilmeli");

        let mut ikinci = ilk.clone();
        ikinci.ad = "İkinci sürüm".into();
        metraj_json_kaydet(&ikinci, &yol).expect("ikinci sürüm kaydedilmeli");

        assert_eq!(metraj_json_yukle(&yol).unwrap().ad, "İkinci sürüm");
        assert_eq!(
            metraj_json_yukle(&ekli_yol(&yol, ".bak")).unwrap().ad,
            "İlk sürüm"
        );
        let _ = std::fs::remove_file(&yol);
        let _ = std::fs::remove_file(ekli_yol(&yol, ".bak"));
    }

    #[test]
    fn pano_ozeti_turkce_karakter_sinirinda_paniklemez() {
        let mut m = ornek_metraj();
        m.kalemler[0].tanim = format!("{}ş uzun açıklama", "a".repeat(34));
        let ozet = metraj_metin_ozet(&m);
        assert!(ozet.contains("..."));
    }

    #[test]
    fn hakedis_excel_hatasiz_uretilir() {
        use crate::models::{Hakedis, HakedisSatiri};
        let yol = gecici_yol("xlsx");
        let kesif = ornek_metraj().kalemler; // 15.150.1001, b.fiyat 1000, sözleşme 27
        let mut h = Hakedis::yeni(1, "İlk", "2026-07-12".into());
        h.satirlar = vec![HakedisSatiri {
            kalem_id: kesif[0].id.clone(),
            poz_no: "15.150.1001".into(),
            kumulatif_miktar: 10.0,
            detaylar: vec![],
        }];
        h.teminat_orani = 5.0;
        hakedis_excel_aktar(
            "Test Projesi",
            &crate::models::ProjeBilgi::default(),
            &kesif,
            &h,
            None,
            &crate::models::SozlesmeAyarlari::default(),
            &yol,
        )
        .expect("hakediş Excel üretilmeli");
        assert!(
            std::fs::metadata(&yol).unwrap().len() > 0,
            "Excel boş olmamalı"
        );
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn teklif_cetveli_dolu_ve_bos_uretilir() {
        let m = ornek_metraj();
        for dolu in [true, false] {
            let yol = gecici_yol("xlsx");
            teklif_cetveli_excel_aktar(&m, dolu, &yol).expect("teklif cetveli üretilmeli");
            assert!(
                std::fs::metadata(&yol).unwrap().len() > 0,
                "teklif cetveli boş olmamalı"
            );
            let _ = std::fs::remove_file(&yol);
        }
    }

    #[test]
    fn csv_yaz_oku_roundtrip() {
        let yol = gecici_yol("csv");
        metraj_csv_aktar(&ornek_metraj(), &yol).expect("CSV yazılmalı");
        let okunan = metraj_csv_oku(&yol).expect("CSV okunmalı");
        assert_eq!(okunan.len(), 1, "başlık atlanmalı, 1 kalem okunmalı");
        assert_eq!(okunan[0].0, "15.150.1001");
        assert_eq!(okunan[0].1, 27.0);
        assert_eq!(okunan[0].2, "Zemin kat perde duvarları");
        let _ = std::fs::remove_file(&yol);
    }
}
