//! Metrajmatik'in ortak tasarım sistemi.
//! Renk, ritim, tipografi ve tekrar kullanılan arayüz bileşenleri burada tutulur.

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Response,
    RichText, Stroke, TextStyle, Ui, Widget, WidgetText,
};
use std::sync::Arc;

// Graphite zemin + güven veren mavi + şantiye amberi.
pub const ARKA_PLAN: Color32 = Color32::from_rgb(0x0B, 0x10, 0x16);
pub const YUZEY: Color32 = Color32::from_rgb(0x10, 0x17, 0x20);
pub const YUZEY_2: Color32 = Color32::from_rgb(0x16, 0x20, 0x2B);
pub const YUZEY_3: Color32 = Color32::from_rgb(0x1D, 0x2A, 0x37);
pub const KENAR: Color32 = Color32::from_rgb(0x2B, 0x3A, 0x49);
pub const KENAR_YUMUSAK: Color32 = Color32::from_rgb(0x20, 0x2C, 0x38);

pub const METIN: Color32 = Color32::from_rgb(0xF1, 0xF5, 0xF9);
pub const METIN_IKINCIL: Color32 = Color32::from_rgb(0xA9, 0xB5, 0xC3);
pub const METIN_SOLUK: Color32 = Color32::from_rgb(0x70, 0x7F, 0x90);

pub const VURGU: Color32 = Color32::from_rgb(0x4B, 0x8D, 0xFF);
pub const VURGU_HOVER: Color32 = Color32::from_rgb(0x72, 0xA7, 0xFF);
pub const VURGU_SOLUK: Color32 = Color32::from_rgb(0x16, 0x2B, 0x49);
pub const AKSAN: Color32 = Color32::from_rgb(0xF2, 0xA9, 0x3B);
pub const AKSAN_SOLUK: Color32 = Color32::from_rgb(0x35, 0x29, 0x18);

pub const BASARI: Color32 = Color32::from_rgb(0x3A, 0xD0, 0x91);
pub const BASARI_KOYU: Color32 = Color32::from_rgb(0x12, 0x32, 0x29);
pub const TEHLIKE: Color32 = Color32::from_rgb(0xFF, 0x64, 0x6D);
pub const TEHLIKE_KOYU: Color32 = Color32::from_rgb(0x3A, 0x1B, 0x22);
pub const UYARI: Color32 = AKSAN;
pub const UYARI_KOYU: Color32 = AKSAN_SOLUK;

pub const KOSE: u8 = 10;
pub const KOSE_KUCUK: u8 = 7;

const IKON_FONTU: &str = "metrajmatik_icons";

pub mod ikon {
    pub const PROJE: &str = "\u{E80F}";
    pub const METRAJ: &str = "\u{E8EF}";
    pub const ICMAL: &str = "\u{E8A5}";
    pub const HAKEDIS: &str = "\u{E9D5}";
    pub const IS_PROGRAMI: &str = "\u{E787}";
    pub const POZLAR: &str = "\u{E721}";
    pub const KITAPLAR: &str = "\u{E8F1}";
    pub const PDF_AKTAR: &str = "\u{E898}";
    pub const KILIT: &str = "\u{E72E}";
    pub const KAMU: &str = "\u{E825}";
    pub const OZEL: &str = "\u{EC06}";
    pub const ANA_GRUP: &str = "\u{E8F4}";
    pub const ALT_GRUP: &str = "\u{ED41}";
    pub const METRAJ_TABLOSU: &str = "\u{F0E3}";
    pub const AKTAR: &str = "\u{EDE1}";
    pub const LOGO: &str = "\u{F0B4}";
    pub const EKLE: &str = "\u{E710}";
    pub const SIL: &str = "\u{E74D}";
    pub const YENI_HAKEDIS: &str = "\u{E82E}";
    pub const ONAY: &str = "\u{E73E}";
    pub const UYARI: &str = "\u{E7BA}";
    pub const KAPAT: &str = "\u{E711}";
    pub const PROJE_AC: &str = "\u{E8E5}";
    pub const KAYDET: &str = "\u{E74E}";
    pub const FARKLI_KAYDET: &str = "\u{E792}";
    pub const DUZENLE: &str = "\u{E70F}";
    pub const YENILE: &str = "\u{E72C}";
    pub const NAKLIYE: &str = "\u{E7C0}";
    pub const PANO: &str = "\u{E8C8}";
    pub const ICE_AKTAR: &str = "\u{E896}";
    pub const SABITLE: &str = "\u{E718}";
}

pub fn ikon_fontu() -> FontFamily {
    FontFamily::Name(IKON_FONTU.into())
}

pub fn ikonlu_metin(ikon: &str, metin: &str) -> WidgetText {
    ikonlu_metin_boyut(ikon, metin, 13.0)
}

pub fn alan_ipucu(metin: impl Into<String>) -> RichText {
    RichText::new(metin).color(METIN_SOLUK).italics()
}

pub fn ikonlu_metin_boyut(ikon: &str, metin: &str, boyut: f32) -> WidgetText {
    ikonlu_metin_renk_boyut(ikon, metin, boyut, Color32::PLACEHOLDER)
}

fn ikonlu_metin_renk_boyut(ikon: &str, metin: &str, boyut: f32, renk: Color32) -> WidgetText {
    let mut duzen = LayoutJob::default();
    duzen.append(
        ikon,
        0.0,
        TextFormat {
            font_id: FontId::new(boyut + 1.5, ikon_fontu()),
            color: renk,
            ..Default::default()
        },
    );
    duzen.append(
        metin,
        5.0,
        TextFormat {
            font_id: FontId::proportional(boyut),
            color: renk,
            ..Default::default()
        },
    );
    duzen.into()
}

pub fn uygula(ctx: &egui::Context) {
    let mut fontlar = FontDefinitions::default();
    let ikon_verisi = std::fs::read(r"C:\Windows\Fonts\SegoeIcons.ttf")
        .or_else(|_| std::fs::read(r"C:\Windows\Fonts\segmdl2.ttf"));
    if let Ok(veri) = ikon_verisi {
        fontlar
            .font_data
            .insert(IKON_FONTU.to_owned(), Arc::new(FontData::from_owned(veri)));
        fontlar
            .families
            .insert(ikon_fontu(), vec![IKON_FONTU.to_owned()]);
        ctx.set_fonts(fontlar);
    }

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(22.0)),
        (TextStyle::Body, FontId::proportional(13.5)),
        (TextStyle::Button, FontId::proportional(13.0)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
        (TextStyle::Small, FontId::proportional(11.0)),
    ]
    .into();

    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(9.0, 8.0);
    spacing.button_padding = egui::vec2(12.0, 7.0);
    spacing.indent = 18.0;
    spacing.window_margin = Margin::same(14);
    spacing.menu_margin = Margin::same(9);
    spacing.interact_size.y = 32.0;
    spacing.scroll.bar_width = 8.0;

    let visuals = &mut style.visuals;
    visuals.dark_mode = true;
    visuals.override_text_color = Some(METIN);
    visuals.panel_fill = YUZEY;
    visuals.window_fill = YUZEY_2;
    visuals.window_stroke = Stroke::new(1.0, KENAR);
    visuals.window_corner_radius = CornerRadius::same(KOSE);
    visuals.extreme_bg_color = ARKA_PLAN;
    visuals.faint_bg_color = Color32::from_rgb(0x14, 0x1C, 0x25);
    visuals.hyperlink_color = VURGU_HOVER;
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(145),
    };
    visuals.popup_shadow = visuals.window_shadow;
    visuals.selection.bg_fill = VURGU_SOLUK;
    visuals.selection.stroke = Stroke::new(1.0, VURGU);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = YUZEY;
    widgets.noninteractive.weak_bg_fill = YUZEY;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, KENAR_YUMUSAK);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, METIN_IKINCIL);
    widgets.noninteractive.corner_radius = CornerRadius::same(KOSE_KUCUK);

    widgets.inactive.bg_fill = YUZEY_2;
    widgets.inactive.weak_bg_fill = YUZEY_2;
    widgets.inactive.bg_stroke = Stroke::new(1.0, KENAR);
    widgets.inactive.fg_stroke = Stroke::new(1.0, METIN);
    widgets.inactive.corner_radius = CornerRadius::same(KOSE_KUCUK);
    widgets.inactive.expansion = 0.0;

    widgets.hovered.bg_fill = YUZEY_3;
    widgets.hovered.weak_bg_fill = YUZEY_3;
    widgets.hovered.bg_stroke = Stroke::new(1.0, VURGU_HOVER);
    widgets.hovered.fg_stroke = Stroke::new(1.0, METIN);
    widgets.hovered.corner_radius = CornerRadius::same(KOSE_KUCUK);
    widgets.hovered.expansion = 0.5;

    widgets.active.bg_fill = VURGU;
    widgets.active.weak_bg_fill = VURGU;
    widgets.active.bg_stroke = Stroke::new(1.0, VURGU_HOVER);
    widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    widgets.active.corner_radius = CornerRadius::same(KOSE_KUCUK);
    widgets.active.expansion = 0.5;

    widgets.open.bg_fill = YUZEY_3;
    widgets.open.weak_bg_fill = YUZEY_3;
    widgets.open.bg_stroke = Stroke::new(1.0, VURGU);
    widgets.open.fg_stroke = Stroke::new(1.0, METIN);
    widgets.open.corner_radius = CornerRadius::same(KOSE_KUCUK);

    ctx.set_style(style);
}

pub fn kart<R>(ui: &mut Ui, ekle: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::default()
        .fill(YUZEY_2)
        .stroke(Stroke::new(1.0, KENAR_YUMUSAK))
        .corner_radius(CornerRadius::same(KOSE))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ekle(ui)
        })
        .inner
}

pub fn sayfa_basligi(ui: &mut Ui, ust: &str, baslik: &str, aciklama: &str) {
    ui.label(
        RichText::new(ust.to_uppercase())
            .size(10.5)
            .strong()
            .color(AKSAN),
    );
    ui.label(RichText::new(baslik).size(23.0).strong().color(METIN));
    if !aciklama.is_empty() {
        ui.label(RichText::new(aciklama).size(12.5).color(METIN_IKINCIL));
    }
    ui.add_space(6.0);
}

pub fn ikonlu_bolum_basligi(ui: &mut Ui, ikon: &str, baslik: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(
            RichText::new(ikon)
                .font(FontId::new(16.0, ikon_fontu()))
                .color(AKSAN),
        );
        ui.label(RichText::new(baslik).size(15.5).strong().color(METIN));
    });
    ui.add_space(2.0);
}

fn renkli_buton(ui: &mut Ui, metin: &str, zemin: Color32, yazi: Color32) -> Response {
    egui::Button::new(RichText::new(metin).color(yazi).strong())
        .fill(zemin)
        .min_size(egui::vec2(0.0, 32.0))
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .ui(ui)
}

fn renkli_ikonlu_buton(
    ui: &mut Ui,
    ikon: &str,
    metin: &str,
    zemin: Color32,
    yazi: Color32,
) -> Response {
    egui::Button::new(ikonlu_metin_renk_boyut(ikon, metin, 13.0, yazi))
        .fill(zemin)
        .min_size(egui::vec2(0.0, 32.0))
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .ui(ui)
}

pub fn birincil_buton(ui: &mut Ui, metin: &str) -> Response {
    renkli_buton(ui, metin, VURGU, Color32::WHITE)
}

pub fn birincil_ikonlu_buton(ui: &mut Ui, ikon: &str, metin: &str) -> Response {
    egui::Button::new(ikonlu_metin(ikon, metin))
        .fill(VURGU)
        .min_size(egui::vec2(0.0, 32.0))
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .ui(ui)
}

pub fn ikincil_buton(ui: &mut Ui, metin: &str) -> Response {
    egui::Button::new(RichText::new(metin).color(METIN).strong())
        .fill(YUZEY_3)
        .stroke(Stroke::new(1.0, KENAR))
        .min_size(egui::vec2(0.0, 32.0))
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .ui(ui)
}

pub fn ikincil_ikonlu_buton(ui: &mut Ui, ikon: &str, metin: &str) -> Response {
    egui::Button::new(ikonlu_metin(ikon, metin))
        .fill(YUZEY_3)
        .stroke(Stroke::new(1.0, KENAR))
        .min_size(egui::vec2(0.0, 32.0))
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .ui(ui)
}

pub fn basari_buton(ui: &mut Ui, metin: &str) -> Response {
    renkli_buton(ui, metin, BASARI, Color32::from_rgb(0x07, 0x2A, 0x1D))
}

pub fn basari_ikonlu_buton(ui: &mut Ui, ikon: &str, metin: &str) -> Response {
    renkli_ikonlu_buton(ui, ikon, metin, BASARI, Color32::from_rgb(0x07, 0x2A, 0x1D))
}

pub fn tehlike_buton(ui: &mut Ui, metin: &str) -> Response {
    renkli_buton(ui, metin, TEHLIKE, Color32::WHITE)
}

pub fn tehlike_ikonlu_buton(ui: &mut Ui, ikon: &str, metin: &str) -> Response {
    renkli_ikonlu_buton(ui, ikon, metin, TEHLIKE, Color32::WHITE)
}

pub fn ikincil_ikon_butonu(ui: &mut Ui, ikon: &str) -> Response {
    egui::Button::new(
        RichText::new(ikon)
            .font(FontId::new(13.0, ikon_fontu()))
            .color(METIN_IKINCIL),
    )
    .fill(YUZEY_3)
    .stroke(Stroke::new(1.0, KENAR))
    .min_size(egui::vec2(28.0, 28.0))
    .corner_radius(CornerRadius::same(KOSE_KUCUK))
    .ui(ui)
}

pub fn tehlike_ikon_butonu(ui: &mut Ui, ikon: &str) -> Response {
    egui::Button::new(
        RichText::new(ikon)
            .font(FontId::new(13.0, ikon_fontu()))
            .color(TEHLIKE),
    )
    .fill(TEHLIKE_KOYU)
    .stroke(Stroke::new(1.0, TEHLIKE))
    .min_size(egui::vec2(26.0, 26.0))
    .corner_radius(CornerRadius::same(KOSE_KUCUK))
    .ui(ui)
}

pub fn bildirim_seridi(ui: &mut Ui, metin: &str, zemin: Color32, kenar: Color32, yazi: Color32) {
    egui::Frame::default()
        .fill(zemin)
        .stroke(Stroke::new(1.0, kenar))
        .corner_radius(CornerRadius::same(KOSE_KUCUK))
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.label(RichText::new(metin).color(yazi));
        });
}

pub fn rozet(ui: &mut Ui, metin: &str, renk: Color32) {
    egui::Frame::default()
        .fill(YUZEY_2)
        .stroke(Stroke::new(1.0, KENAR_YUMUSAK))
        .corner_radius(CornerRadius::same(20))
        .inner_margin(Margin::symmetric(9, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(metin).size(11.5).strong().color(renk));
        });
}

pub fn ikonlu_rozet(ui: &mut Ui, ikon: &str, metin: &str, renk: Color32) {
    egui::Frame::default()
        .fill(YUZEY_2)
        .stroke(Stroke::new(1.0, KENAR_YUMUSAK))
        .corner_radius(CornerRadius::same(20))
        .inner_margin(Margin::symmetric(9, 4))
        .show(ui, |ui| {
            let mut duzen = LayoutJob::default();
            duzen.append(
                ikon,
                0.0,
                TextFormat {
                    font_id: FontId::new(12.5, ikon_fontu()),
                    color: renk,
                    ..Default::default()
                },
            );
            duzen.append(
                metin,
                5.0,
                TextFormat {
                    font_id: FontId::proportional(11.5),
                    color: renk,
                    ..Default::default()
                },
            );
            ui.label(duzen);
        });
}

pub fn istatistik(ui: &mut Ui, etiket: &str, deger: &str, aciklama: &str, renk: Color32) {
    egui::Frame::default()
        .fill(YUZEY_2)
        .stroke(Stroke::new(1.0, KENAR_YUMUSAK))
        .corner_radius(CornerRadius::same(KOSE))
        .inner_margin(Margin::symmetric(15, 11))
        .show(ui, |ui| {
            ui.set_min_width(172.0);
            ui.label(
                RichText::new(etiket.to_uppercase())
                    .size(9.5)
                    .strong()
                    .color(METIN_SOLUK),
            );
            ui.label(RichText::new(deger).size(17.0).strong().color(renk));
            if !aciklama.is_empty() {
                ui.label(RichText::new(aciklama).size(10.5).color(METIN_IKINCIL));
            }
        });
}
