//! Uygulama genelinde tek bir tasarım dili sağlayan tema ve bileşen yardımcıları.
//! Renkler, boşluklar, köşe yuvarlamaları ve yazı tipleri tek bir yerden yönetilir.

use eframe::egui;
use egui::{Color32, CornerRadius, FontId, Margin, Response, RichText, Stroke, TextStyle, Ui, Widget};

// ==================== RENK PALETİ (Koyu, profesyonel) ====================
pub const ARKA_PLAN: Color32 = Color32::from_rgb(0x12, 0x16, 0x1D); // uygulama zemini
pub const YUZEY: Color32 = Color32::from_rgb(0x19, 0x1F, 0x28); // paneller
pub const YUZEY_2: Color32 = Color32::from_rgb(0x22, 0x2A, 0x35); // kartlar / girişler
pub const YUZEY_3: Color32 = Color32::from_rgb(0x2D, 0x37, 0x45); // hover
pub const KENAR: Color32 = Color32::from_rgb(0x32, 0x3D, 0x4C); // ince kenarlıklar
pub const KENAR_YUMUSAK: Color32 = Color32::from_rgb(0x28, 0x31, 0x3D);

pub const METIN: Color32 = Color32::from_rgb(0xE7, 0xEC, 0xF2); // ana metin
pub const METIN_IKINCIL: Color32 = Color32::from_rgb(0xA7, 0xB2, 0xC0); // ikincil
pub const METIN_SOLUK: Color32 = Color32::from_rgb(0x6C, 0x78, 0x88); // ipucu / pasif

pub const VURGU: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6); // birincil (mavi)
pub const VURGU_HOVER: Color32 = Color32::from_rgb(0x5A, 0x9B, 0xFF);
pub const VURGU_SOLUK: Color32 = Color32::from_rgb(0x1E, 0x2F, 0x4A); // seçili satır zemini

pub const BASARI: Color32 = Color32::from_rgb(0x34, 0xC7, 0x59); // yeşil (tutar)
pub const BASARI_KOYU: Color32 = Color32::from_rgb(0x16, 0x3A, 0x24);
pub const TEHLIKE: Color32 = Color32::from_rgb(0xEF, 0x52, 0x52); // kırmızı
pub const TEHLIKE_KOYU: Color32 = Color32::from_rgb(0x3A, 0x1C, 0x1C);
pub const UYARI: Color32 = Color32::from_rgb(0xF5, 0xA6, 0x23); // amber
pub const UYARI_KOYU: Color32 = Color32::from_rgb(0x3A, 0x2D, 0x12);

// ==================== ÖLÇÜLER ====================
pub const KOSE: u8 = 8; // standart köşe yuvarlama
pub const KOSE_KUCUK: u8 = 6;

/// Tüm egui stilini (renkler, yazı tipleri, boşluklar) tek seferde ayarlar.
pub fn uygula(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // --- Yazı tipi boyutları ---
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(19.0)),
        (TextStyle::Body, FontId::proportional(14.0)),
        (TextStyle::Button, FontId::proportional(14.0)),
        (TextStyle::Monospace, FontId::monospace(13.0)),
        (TextStyle::Small, FontId::proportional(11.5)),
    ]
    .into();

    // --- Boşluklar ---
    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 7.0);
    s.button_padding = egui::vec2(11.0, 6.0);
    s.indent = 16.0;
    s.window_margin = Margin::same(12);
    s.menu_margin = Margin::same(8);
    s.interact_size.y = 28.0;
    s.scroll.bar_width = 10.0;

    // --- Görünüm (renkler) ---
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.override_text_color = Some(METIN);
    v.panel_fill = YUZEY;
    v.window_fill = YUZEY_2;
    v.window_stroke = Stroke::new(1.0, KENAR);
    v.window_corner_radius = CornerRadius::same(KOSE);
    v.extreme_bg_color = ARKA_PLAN; // metin kutusu zemini
    v.faint_bg_color = Color32::from_rgb(0x1E, 0x25, 0x2F); // çizgili satır
    v.hyperlink_color = VURGU_HOVER;
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };
    v.popup_shadow = v.window_shadow;

    // Seçim (selectable_label aktif, metin seçimi)
    v.selection.bg_fill = VURGU_SOLUK;
    v.selection.stroke = Stroke::new(1.0, VURGU);

    // Widget durumları
    let w = &mut v.widgets;
    // Pasif / etkileşimsiz (etiketler, ayraçlar)
    w.noninteractive.bg_fill = YUZEY;
    w.noninteractive.weak_bg_fill = YUZEY;
    w.noninteractive.bg_stroke = Stroke::new(1.0, KENAR_YUMUSAK);
    w.noninteractive.fg_stroke = Stroke::new(1.0, METIN_IKINCIL);
    w.noninteractive.corner_radius = CornerRadius::same(KOSE_KUCUK);

    // Etkileşimli ama vurgusuz (butonlar, combobox - normal hal)
    w.inactive.bg_fill = YUZEY_2;
    w.inactive.weak_bg_fill = YUZEY_2;
    w.inactive.bg_stroke = Stroke::new(1.0, KENAR);
    w.inactive.fg_stroke = Stroke::new(1.0, METIN);
    w.inactive.corner_radius = CornerRadius::same(KOSE_KUCUK);
    w.inactive.expansion = 0.0;

    // Üzerine gelince
    w.hovered.bg_fill = YUZEY_3;
    w.hovered.weak_bg_fill = YUZEY_3;
    w.hovered.bg_stroke = Stroke::new(1.0, VURGU);
    w.hovered.fg_stroke = Stroke::new(1.0, METIN);
    w.hovered.corner_radius = CornerRadius::same(KOSE_KUCUK);
    w.hovered.expansion = 1.0;

    // Tıklanınca / aktif
    w.active.bg_fill = VURGU;
    w.active.weak_bg_fill = VURGU;
    w.active.bg_stroke = Stroke::new(1.0, VURGU_HOVER);
    w.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    w.active.corner_radius = CornerRadius::same(KOSE_KUCUK);
    w.active.expansion = 1.0;

    // Açık (combobox açık vb.)
    w.open.bg_fill = YUZEY_3;
    w.open.weak_bg_fill = YUZEY_3;
    w.open.bg_stroke = Stroke::new(1.0, VURGU);
    w.open.fg_stroke = Stroke::new(1.0, METIN);
    w.open.corner_radius = CornerRadius::same(KOSE_KUCUK);

    ctx.set_style(style);
}

// ==================== BİLEŞEN YARDIMCILARI ====================

/// İçeriği yumuşak köşeli, kenarlıklı bir "kart" çerçevesi içine alır.
pub fn kart<R>(ui: &mut Ui, ekle: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::default()
        .fill(YUZEY_2)
        .stroke(Stroke::new(1.0, KENAR))
        .corner_radius(CornerRadius::same(KOSE))
        .inner_margin(Margin::same(12))
        .show(ui, ekle)
        .inner
}

/// Bölüm başlığı: ikon + başlık metni, vurgu renginde.
pub fn bolum_basligi(ui: &mut Ui, ikon: &str, baslik: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(ikon).size(17.0));
        ui.label(RichText::new(baslik).size(16.0).strong().color(METIN));
    });
    ui.add_space(2.0);
}

/// Belirgin köşeli renkli buton üreten dahili yardımcı.
fn renkli_buton(ui: &mut Ui, metin: &str, zemin: Color32, yazi: Color32) -> Response {
    egui::Button::new(RichText::new(metin).color(yazi).strong())
        .fill(zemin)
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .ui(ui)
}

/// Birincil eylem butonu (mavi).
pub fn birincil_buton(ui: &mut Ui, metin: &str) -> Response {
    renkli_buton(ui, metin, VURGU, Color32::WHITE)
}

/// Olumlu eylem butonu (yeşil).
pub fn basari_buton(ui: &mut Ui, metin: &str) -> Response {
    renkli_buton(ui, metin, BASARI, Color32::from_rgb(0x06, 0x2A, 0x12))
}

/// Yıkıcı eylem butonu (kırmızı).
pub fn tehlike_buton(ui: &mut Ui, metin: &str) -> Response {
    renkli_buton(ui, metin, TEHLIKE, Color32::WHITE)
}

/// Renkli zeminli bildirim şeridi (başarı / hata / uyarı).
pub fn bildirim_seridi(ui: &mut Ui, metin: &str, zemin: Color32, kenar: Color32, yazi: Color32) {
    egui::Frame::default()
        .fill(zemin)
        .stroke(Stroke::new(1.0, kenar))
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .inner_margin(Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.label(RichText::new(metin).color(yazi));
        });
}

/// Etiket-değer rozeti (durum çubuğu / özet için).
pub fn rozet(ui: &mut Ui, metin: &str, renk: Color32) {
    egui::Frame::default()
        .fill(YUZEY_2)
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(metin).size(12.0).color(renk));
        });
}
