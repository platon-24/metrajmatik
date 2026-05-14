use rust_xlsxwriter::*;
use std::path::Path;

use crate::models::KayitliMetraj;

/// Metrajı Excel dosyasına aktarır
pub fn metraj_excel_aktar(metraj: &KayitliMetraj, dosya_yolu: &Path) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

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

    let toplam_format = Format::new()
        .set_font_size(12)
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00")
        .set_background_color(Color::RGB(0x27AE60))
        .set_font_color(Color::White);

    // Başlık satırı
    worksheet
        .merge_range(0, 0, 0, 5, &format!("{} - METRAJ ÖZETİ", metraj.ad), &baslik_format)
        .map_err(|e| e.to_string())?;
    worksheet.set_row_height(0, 30).map_err(|e| e.to_string())?;

    worksheet
        .merge_range(1, 0, 1, 5, &format!("Tarih: {}", metraj.tarih), &metin_format)
        .map_err(|e| e.to_string())?;

    // Sütun başlıkları - 3. satır
    worksheet.write_with_format(3, 0, "Sıra No", &sutun_format).map_err(|e| e.to_string())?;
    worksheet.write_with_format(3, 1, "Poz No", &sutun_format).map_err(|e| e.to_string())?;
    worksheet.write_with_format(3, 2, "Açıklama", &sutun_format).map_err(|e| e.to_string())?;
    worksheet.write_with_format(3, 3, "Birim", &sutun_format).map_err(|e| e.to_string())?;
    worksheet.write_with_format(3, 4, "Birim Fiyat (TL)", &sutun_format).map_err(|e| e.to_string())?;
    worksheet.write_with_format(3, 5, "Miktar", &sutun_format).map_err(|e| e.to_string())?;
    worksheet.write_with_format(3, 6, "Tutar (TL)", &sutun_format).map_err(|e| e.to_string())?;

    worksheet.set_column_width(0, 8).map_err(|e| e.to_string())?;
    worksheet.set_column_width(1, 14).map_err(|e| e.to_string())?;
    worksheet.set_column_width(2, 55).map_err(|e| e.to_string())?;
    worksheet.set_column_width(3, 10).map_err(|e| e.to_string())?;
    worksheet.set_column_width(4, 15).map_err(|e| e.to_string())?;
    worksheet.set_column_width(5, 12).map_err(|e| e.to_string())?;
    worksheet.set_column_width(6, 15).map_err(|e| e.to_string())?;

    // Veri satırları
    let mut satir = 4u32;
    for (idx, kalem) in metraj.kalemler.iter().enumerate() {
        worksheet
            .write_with_format(satir, 0, (idx + 1) as u32, &metin_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_with_format(satir, 1, &kalem.poz_no, &metin_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_with_format(satir, 2, &kalem.tanim, &metin_format)
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

    // Toplam satırı
    satir += 1;
    worksheet
        .merge_range(satir, 0, satir, 5, "GENEL TOPLAM", &toplam_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .write_with_format(satir, 6, metraj.toplam_tutar(), &toplam_format)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_row_height(satir, 28)
        .map_err(|e| e.to_string())?;

    workbook.save(dosya_yolu).map_err(|e| e.to_string())?;
    Ok(())
}

/// Metrajı JSON dosyasına kaydeder
pub fn metraj_json_kaydet(metraj: &KayitliMetraj, dosya_yolu: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(metraj).map_err(|e| e.to_string())?;
    std::fs::write(dosya_yolu, json).map_err(|e| format!("Dosya yazılamadı: {}", e))?;
    Ok(())
}

/// JSON dosyasından metraj yükler
pub fn metraj_json_yukle(dosya_yolu: &Path) -> Result<KayitliMetraj, String> {
    let json = std::fs::read_to_string(dosya_yolu)
        .map_err(|e| format!("Dosya okunamadı: {}", e))?;
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
        let tanim_kisa = if kalem.tanim.len() > 38 {
            format!("{}...", &kalem.tanim[..35])
        } else {
            kalem.tanim.clone()
        };

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