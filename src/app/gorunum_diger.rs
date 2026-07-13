//! Metraj dışındaki sekmelerin çizimi: Kitap Yöneticisi, Keşif İcmali / Yaklaşık
//! Maliyet, Pozlar (poz ekle/düzenle/sil form ve onayları) ve PDF yükleme.

use eframe::egui;
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui, Vec2};
use std::path::PathBuf;

use crate::bicim::{metni_kisalt, para_formatla, teklif_ortalamasi};
use crate::is_grubu::grup_canli_toplam;
use crate::maliyet::MaliyetOzeti;
use crate::models::{Donem, HesapTuru, Kitap, Poz};
use crate::tema;

use super::MetrajApp;

impl MetrajApp {
    // ==================== KURUM / KİTAP YÖNETİCİSİ ====================
    pub(crate) fn render_kitap_yoneticisi(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "📚", "Kurum / Fiyat Kitabı Yöneticisi");
        ui.add_space(6.0);

        // Yeni kurum ekleme (yalnız ad — dönem yok; fiyatlar PDF Yükle'den yüklenir)
        tema::kart(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Kurum Adı").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(TextEdit::singleline(&mut self.yeni_kitap_adi).hint_text("örn: Çevre ve Şehircilik Bakanlığı").desired_width(320.0));
                if tema::birincil_buton(ui, "＋ Kurum Ekle").clicked() {
                    let ad = self.yeni_kitap_adi.trim().to_string();
                    if ad.is_empty() { self.hata_mesaji = "Kurum adı girin.".into(); }
                    else if let Some(ref db) = self.db {
                        match db.kitap_ekle(&ad) {
                            Ok(_) => { self.basarili_mesaj = format!("'{}' eklendi. Aylık fiyatları PDF Yükle sekmesinden döneme göre yükleyin.", ad); self.yeni_kitap_adi.clear(); self.kitaplari_yenile(); }
                            Err(e) => self.hata_mesaji = format!("{}", e),
                        }
                    } else { self.hata_mesaji = "Veritabanı açık değil!".into(); }
                }
            });
            ui.add_space(2.0);
            ui.label(RichText::new("Kurum bir kez eklenir; aynı kurumun aylık fiyatları (dönemleri) altına birikir.").color(tema::METIN_SOLUK).size(11.0));
        });

        ui.add_space(10.0);
        ui.label(RichText::new("Kurumlar").strong().size(14.0).color(tema::METIN)); ui.add_space(6.0);
        if self.kitaplar.is_empty() { ui.label(RichText::new("Henüz kurum yok.").color(tema::METIN_SOLUK)); return; }

        // Dönemleri önceden çek (grid içinde db borrow çakışmasını önler)
        let kitaplar_snapshot = self.kitaplar.clone();
        let mut donem_map: std::collections::HashMap<i64, Vec<Donem>> = std::collections::HashMap::new();
        if let Some(ref db) = self.db {
            for k in &kitaplar_snapshot {
                donem_map.insert(k.id, db.donemler(k.id).unwrap_or_default());
            }
        }

        let mut secilecek: Option<Kitap> = None;
        let mut duzenlenecek: Option<Kitap> = None;
        let mut silinecek: Option<Kitap> = None;
        let mut silinecek_donem: Option<(i64, u32, u32)> = None;
        ScrollArea::vertical().show(ui, |ui| {
            for kitap in &kitaplar_snapshot {
                let secili = self.secili_kitap.as_ref().map(|k| k.id == kitap.id).unwrap_or(false);
                let donemler = donem_map.get(&kitap.id).cloned().unwrap_or_default();
                egui::Frame::default()
                    .fill(if secili { tema::VURGU_SOLUK } else { tema::YUZEY_2 })
                    .stroke(egui::Stroke::new(1.0, if secili { tema::VURGU } else { tema::KENAR }))
                    .corner_radius(egui::CornerRadius::same(tema::KOSE))
                    .inner_margin(egui::Margin::symmetric(12, 9))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(if secili { "📗" } else { "📘" }).size(16.0));
                            if ui.selectable_label(secili, RichText::new(&kitap.ad).size(14.0).strong()).clicked() {
                                secilecek = Some(kitap.clone());
                            }
                            tema::rozet(ui, &format!("{} poz", kitap.poz_sayisi), tema::METIN_IKINCIL);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::Button::new(RichText::new("🗑").color(tema::TEHLIKE)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::new(1.0, tema::KENAR))).on_hover_text("Kurumu ve tüm dönemlerini sil").clicked() { silinecek = Some(kitap.clone()); }
                                if ui.button("✏ Ad").clicked() { duzenlenecek = Some(kitap.clone()); }
                            });
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Dönemler:").color(tema::METIN_SOLUK).size(11.5));
                            if donemler.is_empty() {
                                ui.label(RichText::new("henüz fiyat yok — PDF Yükle sekmesinden ekleyin").color(tema::UYARI).size(11.5));
                            } else {
                                for d in &donemler {
                                    tema::rozet(ui, &format!("{}/{} · {} poz", d.ay, d.yil, d.poz_sayisi), tema::VURGU_HOVER);
                                    if ui.add(egui::Button::new(RichText::new("✕").color(tema::TEHLIKE).size(10.0)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::NONE)).on_hover_text("Bu dönemi sil (PDF'ten yeniden yüklenebilir)").clicked() {
                                        silinecek_donem = Some((kitap.id, d.yil, d.ay));
                                    }
                                }
                            }
                        });
                    });
                ui.add_space(6.0);
            }
        });

        if let Some(k) = secilecek {
            self.secili_kitap = Some(k.clone());
            self.kategorileri_yukle();
            self.basarili_mesaj = format!("{} seçildi.", k.ad);
        }
        if let Some(k) = duzenlenecek {
            self.duzenlenen_kitap = Some(k.clone());
            self.duzenleme_adi = k.ad;
        }
        // Kurum silme: doğrudan silme, onay ekranı iste (render_kitap_sil_onay_popup)
        if let Some(k) = silinecek {
            self.silinecek_kitap = Some(k);
        }
        if let Some((kid, y, a)) = silinecek_donem {
            if let Some(ref db) = self.db {
                if db.donem_sil(kid, y, a).is_ok() {
                    self.basarili_mesaj = format!("{}/{} dönemi silindi.", a, y);
                    self.kitaplari_yenile();
                    self.pozlar_tablosu_yenile();
                }
            }
        }
    }

    /// Kurum silme onay ekranı: kurum "tak diye" silinmez, önce onay istenir.
    pub(crate) fn render_kitap_sil_onay_popup(&mut self, ctx: &egui::Context) {
        let kitap = match self.silinecek_kitap.clone() {
            Some(k) => k,
            None => return,
        };
        let mut sil = false;
        let mut iptal = false;
        egui::Window::new("⚠ Kurumu Sil")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new(format!("'{}' kurumu silinecek.", kitap.ad)).strong().size(14.0).color(tema::METIN));
                ui.add_space(4.0);
                ui.label(RichText::new(format!("⚠ Bu işlem kurumun TÜM dönemlerini, {} pozunu ve bağlı analizleri kalıcı olarak siler.", kitap.poz_sayisi)).color(tema::UYARI));
                ui.label(RichText::new("Geri alınamaz. (Kaydedilmiş metrajlarınız etkilenmez — fiyatlar kalemlere kopyalanmıştır.)").color(tema::METIN_SOLUK).size(12.0));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if tema::tehlike_buton(ui, "🗑 Evet, Sil").clicked() { sil = true; }
                    if ui.button("Vazgeç").clicked() { iptal = true; }
                });
            });
        if iptal {
            self.silinecek_kitap = None;
        }
        if sil {
            if let Some(ref db) = self.db {
                match db.kitap_sil(kitap.id) {
                    Ok(()) => {
                        if self.secili_kitap.as_ref().map(|k| k.id == kitap.id).unwrap_or(false) { self.secili_kitap = None; }
                        self.basarili_mesaj = format!("'{}' kurumu silindi.", kitap.ad);
                        self.hata_mesaji.clear();
                        self.kitaplari_yenile();
                        self.pozlar_tablosu_yenile();
                    }
                    Err(e) => self.hata_mesaji = format!("{}", e),
                }
            }
            self.silinecek_kitap = None;
        }
    }

    // ==================== İCMAL / YAKLAŞIK MALİYET ====================
    pub(crate) fn render_icmal(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "📊", "Keşif İcmali / Yaklaşık Maliyet");
        ui.add_space(6.0);

        let secili = self.secili_grup_id.as_deref();
        // Üst düzey grupların canlı toplamları
        let grup_satirlari: Vec<(String, f64)> = self.is_gruplari.iter()
            .map(|g| (g.ad.clone(), grup_canli_toplam(g, secili, &self.metraj_kalemleri)))
            .collect();
        let ara_toplam: f64 = self.toplam_tutar();

        // Hesap türü + oran ayarları
        tema::kart(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hesap Türü").color(tema::METIN_IKINCIL).size(12.0));
                if ui.selectable_label(self.hesap_turu == HesapTuru::Kamu, "🏛 Kamu (KDV hariç)").clicked() {
                    self.hesap_turu = HesapTuru::Kamu; self.degisiklik_var = true;
                }
                if ui.selectable_label(self.hesap_turu == HesapTuru::Ozel, "🏢 Özel (KDV dahil)").clicked() {
                    self.hesap_turu = HesapTuru::Ozel; self.degisiklik_var = true;
                }
                ui.add_space(18.0);
                ui.separator();
                ui.add_space(18.0);
                ui.label(RichText::new("Genel Gider + Müteahhit Kârı").color(tema::METIN_IKINCIL).size(12.0));
                if ui.add(egui::DragValue::new(&mut self.genel_gider_kar_orani).speed(0.5).range(0.0..=100.0).suffix(" %")).changed() { self.degisiklik_var = true; }
                if self.hesap_turu == HesapTuru::Ozel {
                    ui.add_space(18.0);
                    ui.label(RichText::new("KDV").color(tema::METIN_IKINCIL).size(12.0));
                    if ui.add(egui::DragValue::new(&mut self.kdv_orani).speed(0.5).range(0.0..=100.0).suffix(" %")).changed() { self.degisiklik_var = true; }
                }
            });
            if self.hesap_turu == HesapTuru::Kamu {
                ui.add_space(5.0);
                ui.label(RichText::new("⚠ Kurum birim fiyatları kâr ve genel gideri zaten içerir. Bu oranı yalnızca analiz/rayiç ile fiyatlandırdığınız özel pozlar için girin. Kamu yaklaşık maliyeti KDV hariç hesaplanır.").color(tema::UYARI).size(11.0));
            }
        });
        ui.add_space(8.0);

        if grup_satirlari.is_empty() {
            tema::bildirim_seridi(ui, "Henüz iş grubu yok. Metraj sekmesinden grup ve poz ekleyin.", tema::UYARI_KOYU, tema::UYARI, tema::UYARI);
            return;
        }

        // İş grupları icmal tablosu
        tema::kart(ui, |ui| {
            egui::Grid::new("icmal_grid").num_columns(4).spacing(egui::vec2(16.0, 9.0)).striped(true).show(ui, |ui| {
                ui.label(RichText::new("Sıra").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("İş Grubu").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Tutar (TL)").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Oran").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.end_row();
                for (i, (ad, tutar)) in grup_satirlari.iter().enumerate() {
                    let yuzde = if ara_toplam > 0.0 { tutar / ara_toplam * 100.0 } else { 0.0 };
                    ui.label(RichText::new(format!("{}", i + 1)).color(tema::METIN_SOLUK));
                    ui.label(RichText::new(ad).size(13.5).strong().color(tema::METIN));
                    ui.label(RichText::new(para_formatla(*tutar)).size(13.0).color(tema::METIN));
                    ui.label(RichText::new(format!("% {:.1}", yuzde)).size(12.5).color(tema::VURGU_HOVER));
                    ui.end_row();
                }
            });
        });
        ui.add_space(10.0);

        // Yaklaşık maliyet özeti (tek kaynak: maliyet::MaliyetOzeti)
        let ozet = MaliyetOzeti::hesapla(ara_toplam, self.genel_gider_kar_orani, self.kdv_orani, self.hesap_turu);
        let kamu = self.hesap_turu == HesapTuru::Kamu;
        let kar_orani = self.genel_gider_kar_orani;
        let kdv_orani = self.kdv_orani;

        egui::Frame::default()
            .fill(tema::YUZEY_2)
            .stroke(egui::Stroke::new(1.0, tema::KENAR))
            .corner_radius(egui::CornerRadius::same(tema::KOSE))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                let satir = |ui: &mut egui::Ui, etiket: &str, deger: f64, vurgulu: bool| {
                    ui.horizontal(|ui| {
                        let renk = if vurgulu { tema::METIN } else { tema::METIN_IKINCIL };
                        ui.label(RichText::new(etiket).color(renk).size(if vurgulu { 14.0 } else { 13.0 }).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{} TL", para_formatla(deger))).color(if vurgulu { tema::BASARI } else { tema::METIN }).strong().size(if vurgulu { 16.0 } else { 13.0 }));
                        });
                    });
                };
                satir(ui, "Ara Toplam (İşçilik + Malzeme)", ozet.ara_toplam, false);
                // Kâr satırı yalnızca bir oran girildiğinde (analiz pozları için) gösterilir.
                if kar_orani > 0.0 {
                    ui.add_space(3.0);
                    satir(ui, &format!("Genel Gider & Müteahhit Kârı (% {:.1})", kar_orani), ozet.kar, false);
                }
                // KDV yalnızca Özel türünde.
                if !kamu {
                    ui.add_space(3.0);
                    satir(ui, "KDV Matrahı", ozet.kdv_matrahi, false);
                    ui.add_space(3.0);
                    satir(ui, &format!("KDV (% {:.1})", kdv_orani), ozet.kdv, false);
                }
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                let toplam_baslik = if kamu { "TOPLAM YAKLAŞIK MALİYET (KDV Hariç)" } else { "TOPLAM YAKLAŞIK MALİYET (KDV Dahil)" };
                satir(ui, toplam_baslik, ozet.genel_toplam, true);
            });
    }

    // ==================== POZLAR ====================
    fn poz_formunu_yeni_icin_ac(&mut self) {
        if self.secili_kitap.is_none() {
            self.hata_mesaji = "Önce pozun ekleneceği kurumu seçin.".into();
            return;
        }
        self.poz_form_acik = true;
        self.poz_form_duzenleme = false;
        self.poz_form_eski_poz_no.clear();
        self.poz_form_poz_no.clear();
        self.poz_form_tanim.clear();
        self.poz_form_birim.clear();
        self.poz_form_fiyat.clear();
        self.poz_form_teklifler.clear();
        self.poz_form_kategori = "Özel Poz".into();
        // Yeni pozun dönemi varsayılan olarak kurumun en son dönemi (yoksa 2026/1)
        let (y, a) = self.secili_kitap.as_ref().map(|k| if k.yil > 0 { (k.yil, k.ay) } else { (2026, 1) }).unwrap_or((2026, 1));
        self.poz_form_yil = y;
        self.poz_form_ay = a;
    }

    fn poz_formunu_duzenleme_icin_ac(&mut self, poz: Poz) {
        self.poz_form_acik = true;
        self.poz_form_duzenleme = true;
        self.poz_form_eski_poz_no = poz.poz_no.clone();
        self.poz_form_poz_no = poz.poz_no;
        self.poz_form_tanim = poz.tanim;
        self.poz_form_birim = poz.birim;
        self.poz_form_fiyat = poz.fiyat.map(para_formatla).unwrap_or_default();
        self.poz_form_kategori = poz.kategori;
        self.poz_form_teklifler.clear();
        // Düzenlenen pozun görüntülenen dönemi
        self.poz_form_yil = poz.yil;
        self.poz_form_ay = poz.ay;
    }

    fn poz_form_fiyat_degeri(&mut self) -> Option<Option<f64>> {
        let fiyat = self.poz_form_fiyat.trim();
        if fiyat.is_empty() {
            return Some(None);
        }
        match fiyat.replace(',', ".").parse::<f64>() {
            Ok(deger) => Some(Some(deger)),
            Err(_) => {
                self.hata_mesaji = "Birim fiyat sayi olmali. Ornek: 1250,50".into();
                None
            }
        }
    }

    fn poz_form_kaydet(&mut self) {
        let kitap = match self.secili_kitap.clone() {
            Some(kitap) => kitap,
            None => {
                self.hata_mesaji = "Once kitap secin.".into();
                return;
            }
        };
        let poz_no = self.poz_form_poz_no.trim().to_string();
        let tanim = self.poz_form_tanim.trim().to_string();
        let birim = self.poz_form_birim.trim().to_string();
        let kategori = self.poz_form_kategori.trim().to_string();
        if poz_no.is_empty() || tanim.is_empty() || birim.is_empty() || kategori.is_empty() {
            self.hata_mesaji = "Poz no, aciklama, birim ve kategori zorunlu.".into();
            return;
        }
        let fiyat = match self.poz_form_fiyat_degeri() {
            Some(fiyat) => fiyat,
            None => return,
        };
        // Fiyat, formda seçilen döneme yazılır.
        let (yil, ay) = (self.poz_form_yil, self.poz_form_ay);
        if let Some(ref db) = self.db {
            let sonuc = if self.poz_form_duzenleme {
                db.poz_guncelle(kitap.id, yil, ay, &self.poz_form_eski_poz_no, &poz_no, &tanim, &birim, fiyat, &kategori)
            } else {
                db.poz_ekle(kitap.id, yil, ay, &poz_no, &tanim, &birim, fiyat, &kategori)
            };
            match sonuc {
                Ok(()) => {
                    self.poz_form_acik = false;
                    self.basarili_mesaj = if self.poz_form_duzenleme {
                        format!("{} guncellendi.", poz_no)
                    } else {
                        format!("{} eklendi.", poz_no)
                    };
                    self.hata_mesaji.clear();
                    self.kitaplari_yenile();
                    self.kategorileri_yukle();
                    self.pozlar_tablosu_yenile();
                }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }

    pub(crate) fn render_poz_form_popup(&mut self, ctx: &egui::Context) {
        if !self.poz_form_acik {
            return;
        }
        let baslik = if self.poz_form_duzenleme { "Poz Düzenle" } else { "Poz Ekle" };
        let mut acik = self.poz_form_acik;
        let mut kaydet = false;
        let mut iptal = false;
        egui::Window::new(baslik)
            .collapsible(false)
            .resizable(false)
            .open(&mut acik)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                if let Some(ref kitap) = self.secili_kitap {
                    ui.label(RichText::new(format!("📚 {}", kitap.ad)).color(tema::METIN_IKINCIL).size(12.0));
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Poz No").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_poz_no).desired_width(220.0));
                });
                ui.label(RichText::new("Açıklama").color(tema::METIN_IKINCIL).size(12.0));
                ui.add(TextEdit::multiline(&mut self.poz_form_tanim).desired_width(420.0).desired_rows(3));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Birim").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_birim).desired_width(80.0));
                    ui.label(RichText::new("B.Fiyat").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_fiyat).hint_text("boş olabilir").desired_width(120.0));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Fiyat Araştırması").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_teklifler).hint_text("teklifler: 1200,00 1350,00 1180,00").desired_width(240.0));
                    if ui.button("Ort. → B.Fiyat").on_hover_text("Tekliflerin ortalamasını birim fiyat yap (piyasa rayici)").clicked() {
                        if let Some(ort) = teklif_ortalamasi(&self.poz_form_teklifler) {
                            self.poz_form_fiyat = format!("{:.2}", ort).replace('.', ",");
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Kategori").color(tema::METIN_IKINCIL).size(12.0));
                    ui.add(TextEdit::singleline(&mut self.poz_form_kategori).desired_width(260.0));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Dönem").color(tema::METIN_IKINCIL).size(12.0));
                    egui::ComboBox::from_id_salt("poz_form_yil").selected_text(format!("{}", self.poz_form_yil)).width(70.0).show_ui(ui, |ui| {
                        for y in [2024u32, 2025, 2026, 2027, 2028] { if ui.selectable_label(self.poz_form_yil == y, format!("{}", y)).clicked() { self.poz_form_yil = y; } }
                    });
                    egui::ComboBox::from_id_salt("poz_form_ay").selected_text(format!("{}", self.poz_form_ay)).width(50.0).show_ui(ui, |ui| {
                        for a in 1u32..=12 { if ui.selectable_label(self.poz_form_ay == a, format!("{}", a)).clicked() { self.poz_form_ay = a; } }
                    });
                    ui.label(RichText::new("bu fiyatın ait olduğu ay/yıl").color(tema::METIN_SOLUK).size(11.0));
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if tema::basari_buton(ui, "💾 Kaydet").clicked() { kaydet = true; }
                    if ui.button("İptal").clicked() { iptal = true; }
                });
            });
        self.poz_form_acik = acik;
        if iptal {
            self.poz_form_acik = false;
        }
        if kaydet {
            self.poz_form_kaydet();
        }
    }

    pub(crate) fn render_poz_sil_onay_popup(&mut self, ctx: &egui::Context) {
        let poz = match self.silinecek_poz.clone() {
            Some(poz) => poz,
            None => return,
        };
        let mut sil = false;
        let mut iptal = false;
        egui::Window::new("Poz Sil")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("⚠  Bu poz kalıcı olarak silinecek.").color(tema::UYARI));
                ui.add_space(4.0);
                ui.label(RichText::new(&poz.poz_no).monospace().strong().color(tema::METIN));
                ui.label(RichText::new(metni_kisalt(&poz.tanim, 90)).color(tema::METIN_IKINCIL));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if tema::tehlike_buton(ui, "🗑 Sil").clicked() { sil = true; }
                    if ui.button("Vazgeç").clicked() { iptal = true; }
                });
            });
        if iptal {
            self.silinecek_poz = None;
        }
        if sil {
            if let Some(ref db) = self.db {
                match db.poz_sil(poz.kitap_id, &poz.poz_no) {
                    Ok(()) => {
                        if self.secili_poz.as_ref().map(|p| p.poz_no == poz.poz_no && p.kitap_id == poz.kitap_id).unwrap_or(false) {
                            self.secili_poz = None;
                        }
                        self.silinecek_poz = None;
                        self.basarili_mesaj = format!("{} silindi.", poz.poz_no);
                        self.hata_mesaji.clear();
                        self.kitaplari_yenile();
                        self.kategorileri_yukle();
                        self.pozlar_tablosu_yenile();
                    }
                    Err(e) => self.hata_mesaji = format!("{}", e),
                }
            }
        }
    }

    pub(crate) fn render_pozlar_tablosu(&mut self, ui: &mut Ui) {
        if self.secili_kitap.as_ref().map(|k| k.id) != self.pozlar_yuklu_kitap_id {
            self.pozlar_tablosu_yenile();
        }

        tema::bolum_basligi(ui, "🔎", "Pozlar");
        ui.add_space(6.0);
        tema::kart(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Kurum").color(tema::METIN_IKINCIL).size(12.0));
                let km = self.secili_kitap.as_ref().map(|k| k.ad.clone()).unwrap_or_else(|| "Kurum seçin".into());
                egui::ComboBox::from_id_salt("pozlar_kitap_secici").selected_text(&km).width(300.0).show_ui(ui, |ui| {
                    for k in self.kitaplar.clone() {
                        if ui.selectable_label(self.secili_kitap.as_ref().map(|sk| sk.id == k.id).unwrap_or(false), &k.ad).clicked() {
                            self.secili_kitap = Some(k);
                            self.pozlar_donem = None; // kurum değişti → en son döneme dön
                            self.pozlar_tablosu_yenile();
                        }
                    }
                });
                // Dönem seçici — eski fiyatları görmek için
                if self.secili_kitap.is_some() && !self.pozlar_donemler.is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new("Dönem").color(tema::METIN_IKINCIL).size(12.0));
                    let dm = match self.pozlar_donem {
                        Some((y, a)) => format!("{}/{}", a, y),
                        None => "En son (güncel)".into(),
                    };
                    egui::ComboBox::from_id_salt("pozlar_donem_secici").selected_text(&dm).width(150.0).show_ui(ui, |ui| {
                        if ui.selectable_label(self.pozlar_donem.is_none(), "En son (güncel)").clicked() {
                            self.pozlar_donem = None;
                            self.pozlar_tablosu_yenile();
                        }
                        for d in self.pozlar_donemler.clone() {
                            let secili = self.pozlar_donem == Some((d.yil, d.ay));
                            if ui.selectable_label(secili, format!("{}/{} · {} poz", d.ay, d.yil, d.poz_sayisi)).clicked() {
                                self.pozlar_donem = Some((d.yil, d.ay));
                                self.pozlar_tablosu_yenile();
                            }
                        }
                    });
                }
            });
            if self.secili_kitap.is_some() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍").size(13.0));
                    if ui.add_sized(Vec2::new(340.0, 26.0), TextEdit::singleline(&mut self.pozlar_arama_metni).hint_text("poz no, açıklama, birim veya kategori")).changed() {
                        self.pozlar_tablosu_yenile();
                    }
                    if !self.pozlar_arama_metni.is_empty() && ui.button("Temizle").clicked() {
                        self.pozlar_arama_metni.clear();
                        self.pozlar_tablosu_yenile();
                    }
                    tema::rozet(ui, &format!("{} poz", self.pozlar_tablosu.len()), tema::METIN_IKINCIL);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if tema::birincil_buton(ui, "＋ Poz Ekle").clicked() {
                            self.poz_formunu_yeni_icin_ac();
                        }
                    });
                });
            }
        });
        ui.add_space(8.0);

        if self.secili_kitap.is_none() {
            tema::bildirim_seridi(ui, "Poz listesini görmek için bir kitap seçin.", tema::UYARI_KOYU, tema::UYARI, tema::UYARI);
            return;
        }

        if self.pozlar_tablosu.is_empty() {
            ui.label(RichText::new("Sonuç bulunamadı.").color(tema::METIN_SOLUK));
            return;
        }

        let pozlar = self.pozlar_tablosu.clone();
        ScrollArea::vertical().max_height(ui.available_height()).auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("pozlar_grid").num_columns(7).min_col_width(60.0).spacing(egui::vec2(12.0, 8.0)).striped(true).show(ui, |ui: &mut egui::Ui| {
                ui.label(RichText::new("Poz No").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Açıklama").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Birim").strong().size(12.0));
                ui.label(RichText::new("B.Fiyat").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Kategori").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("Kitap").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.label(RichText::new("İşlem").strong().size(12.0).color(tema::METIN_IKINCIL));
                ui.end_row();

                for poz in pozlar {
                    let fiyat = poz.fiyat.map(|f| format!("{} TL", para_formatla(f))).unwrap_or_else(|| "Formül".into());
                    let fiyat_renk = if poz.fiyat.is_some() { tema::BASARI } else { tema::UYARI };
                    let aciklama = metni_kisalt(&poz.tanim, 85);
                    let analizli = self.analizli_pozlar.contains(&poz.poz_no);
                    let poz_no_goster = if analizli { format!("🧮 {}", poz.poz_no) } else { poz.poz_no.clone() };
                    let poz_lbl = ui.label(RichText::new(poz_no_goster).monospace().size(11.5).color(tema::METIN));
                    if analizli { poz_lbl.on_hover_text("Birim fiyatı analizden üretildi"); }
                    ui.label(RichText::new(aciklama).size(11.5).color(tema::METIN_IKINCIL)).on_hover_text(&poz.tanim);
                    ui.label(RichText::new(&poz.birim).size(11.0).color(tema::METIN_IKINCIL));
                    ui.label(RichText::new(fiyat).size(11.5).color(fiyat_renk));
                    ui.label(RichText::new(&poz.kategori).size(10.5).color(tema::METIN_SOLUK));
                    ui.label(RichText::new(format!("{}/{}", poz.ay, poz.yil)).size(10.5).color(tema::METIN_SOLUK));
                    ui.horizontal(|ui| {
                        if ui.button("🧮 Analiz").on_hover_text("Birim fiyat analizi yap / düzenle").clicked() {
                            self.analiz_popup_ac(poz.clone());
                        }
                        if ui.button("✏ Düzenle").clicked() {
                            self.poz_formunu_duzenleme_icin_ac(poz.clone());
                        }
                        if ui.add(egui::Button::new(RichText::new("🗑").color(tema::TEHLIKE)).stroke(egui::Stroke::new(1.0, tema::KENAR))).clicked() {
                            self.silinecek_poz = Some(poz.clone());
                        }
                    });
                    ui.end_row();
                }
            });
        });
    }

    // ==================== PDF YUKLE ====================
    pub(crate) fn render_pdf_yukle(&mut self, ui: &mut Ui) {
        tema::bolum_basligi(ui, "📄", "PDF Birim Fiyat Listesi Yükle");
        ui.add_space(6.0);
        if self.kitaplar.is_empty() {
            tema::bildirim_seridi(ui, "⚠  Önce Kurum Yöneticisi'nden bir kurum ekleyin.", tema::TEHLIKE_KOYU, tema::TEHLIKE, tema::TEHLIKE);
            return;
        }
        tema::kart(ui, |ui| {
            ui.label(RichText::new("PDF birim fiyat listesini bir KURUMA ve DÖNEME yükleyin.").color(tema::METIN_IKINCIL).size(12.0));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hedef Kurum").color(tema::METIN_IKINCIL).size(12.0));
                let km = self.secili_kitap.as_ref().map(|k| k.ad.clone()).unwrap_or_else(|| "Kurum seçin".into());
                egui::ComboBox::from_id_salt("pdf_kitap_secici").selected_text(&km).width(280.0).show_ui(ui, |ui| {
                    for k in &self.kitaplar.clone() {
                        if ui.selectable_label(self.secili_kitap.as_ref().map(|sk| sk.id == k.id).unwrap_or(false), &k.ad).clicked() { self.secili_kitap = Some(k.clone()); }
                    }
                });
                ui.add_space(12.0);
                ui.label(RichText::new("Dönem").color(tema::METIN_IKINCIL).size(12.0));
                egui::ComboBox::from_id_salt("pdf_yil").selected_text(format!("{}", self.yeni_kitap_yil)).width(70.0).show_ui(ui, |ui| {
                    for y in [2024u32, 2025, 2026, 2027, 2028] { if ui.selectable_label(self.yeni_kitap_yil == y, format!("{}", y)).clicked() { self.yeni_kitap_yil = y; } }
                });
                egui::ComboBox::from_id_salt("pdf_ay").selected_text(format!("{}", self.yeni_kitap_ay)).width(50.0).show_ui(ui, |ui| {
                    for a in 1u32..=12 { if ui.selectable_label(self.yeni_kitap_ay == a, format!("{}", a)).clicked() { self.yeni_kitap_ay = a; } }
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Ayrıştırma Profili").color(tema::METIN_IKINCIL).size(12.0));
                egui::ComboBox::from_id_salt("pdf_profil").selected_text(&self.import_profili).width(210.0).show_ui(ui, |ui| {
                    for p in ["Otomatik", "Çevre ve Şehircilik", "Vakıflar / Restorasyon", "Karayolları (Ar-Ge)", "Genel"] {
                        if ui.selectable_label(self.import_profili == p, p).clicked() { self.import_profili = p.to_string(); }
                    }
                });
                ui.label(RichText::new("Otomatik: ÇŞB / Vakıflar / KGM arasından en uygun seçilir").color(tema::METIN_SOLUK).size(11.0));
            });
            ui.add_space(3.0);
            ui.label(RichText::new("Aynı kuruma her ay yeni PDF yükleyin: pozlar tekilleşir, fiyatlar döneme göre birikir.").color(tema::METIN_SOLUK).size(11.0));
            ui.add_space(8.0);
            if self.pdf_yukleniyor {
                ui.horizontal(|ui| { ui.spinner(); ui.label(RichText::new("PDF işleniyor…").color(tema::METIN_IKINCIL)); });
            } else if tema::birincil_buton(ui, "📂 PDF Dosyası Seç ve Yükle").clicked() {
                self.pdf_sec_ve_yukle();
            }
            if !self.pdf_durumu.is_empty() { ui.add_space(6.0); ui.label(RichText::new(&self.pdf_durumu).color(tema::BASARI)); }
        });
        let alt = PathBuf::from("20206-05-BF.pdf");
        if alt.exists() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hızlı yükleme:").color(tema::METIN_SOLUK).size(12.0));
                if ui.button("📄 20206-05-BF.pdf").clicked() { self.pdf_yukle(alt); }
            });
        }
    }
}
