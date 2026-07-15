//! Metraj sekmesinin çizimi: fiyat kitabı seçici, poz arama paneli, iş grupları ağacı,
//! metraj kalem tablosu, miktar-detay popup'ı ve özet rozetleri. Ayrıca iş grubu ağaç
//! çizimi ve miktar-detay (boyut) dönüştürme yardımcıları burada.

use eframe::egui;
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui, Vec2};

use crate::bicim::{metni_kisalt, para_formatla, sayi_oku};
use crate::is_grubu::{
    grup_bul_mut, grup_bul_ref, grup_canli_toplam, grup_sil, ilk_yaprak_grup_id,
};
use crate::models::{IsGrubu, MetrajKalemi, MiktarDetay, ProjeAsamasi};
use crate::tema;

use super::{MetrajApp, PopupDetaySatiri, Sekme};

impl MetrajApp {
    // ==================== METRAJ TABLOSU ====================
    pub(crate) fn render_metraj_tablosu(&mut self, ui: &mut Ui) {
        if self.proje_asamasi == ProjeAsamasi::Hakedis {
            tema::sayfa_basligi(
                ui,
                "Salt okunur sözleşme bazı",
                "Metraj donduruldu",
                "Hakediş tutarlılığını korumak için bu çalışma alanı artık düzenlenemez.",
            );
            ui.add_space(6.0);
            tema::bildirim_seridi(
                ui,
                "Bu proje hakediş aşamasına dönüştürüldü. Sözleşme bazını korumak için poz, miktar ve iş grubu düzenleme kapalıdır.",
                tema::UYARI_KOYU,
                tema::UYARI,
                tema::UYARI,
            );
            ui.add_space(10.0);
            tema::kart(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "Dondurulan keşif: {} TL",
                        para_formatla(self.sozlesme_ayarlari.kesif_bedeli)
                    ))
                    .strong(),
                );
                ui.label(format!(
                    "Sözleşme bedeli: {} TL · Tenzilat: % {:.6}",
                    para_formatla(self.sozlesme_ayarlari.hesaplanan_sozlesme_bedeli()),
                    self.sozlesme_ayarlari.tenzilat_orani()
                ));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if tema::birincil_buton(ui, "Hakedişe Git").clicked() {
                        self.sekme_ac(Sekme::Hakedis);
                    }
                    if ui.button("İş Programına Git").clicked() {
                        self.sekme_ac(Sekme::IsProgrami);
                    }
                });
            });
            return;
        }
        tema::sayfa_basligi(
            ui,
            "Keşif çalışma alanı",
            "Metraj",
            "Pozu bulun, iş grubuna yerleştirin ve ölçü detaylarını tek akışta yönetin.",
        );
        ui.horizontal_wrapped(|ui| {
            tema::istatistik(
                ui,
                "Aktif kalem",
                &self.metraj_kalemleri.len().to_string(),
                "Seçili iş grubunda",
                tema::VURGU_HOVER,
            );
            tema::istatistik(
                ui,
                "İş grubu",
                &self.is_gruplari.len().to_string(),
                "Proje kırılımı",
                tema::AKSAN,
            );
            tema::istatistik(
                ui,
                "Keşif toplamı",
                &format!("{} TL", para_formatla(self.toplam_tutar())),
                "Güncel miktarlarla",
                tema::BASARI,
            );
        });
        ui.add_space(10.0);
        let kompakt_duzen = ui.available_width() < 1040.0;
        let kitap_genisligi = (ui.available_width() - 260.0).clamp(180.0, 360.0);
        tema::kart(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Fiyat Kitabı")
                        .color(tema::METIN_IKINCIL)
                        .strong(),
                );
                let km = self
                    .secili_kitap
                    .as_ref()
                    .map(|k| format!("{} ({}/{})", k.ad, k.ay, k.yil))
                    .unwrap_or_else(|| "TÜM KİTAPLAR".into());
                egui::ComboBox::from_id_salt("kitap_secici")
                    .selected_text(&km)
                    .width(kitap_genisligi)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.secili_kitap.is_none(), "TÜM KİTAPLAR")
                            .clicked()
                        {
                            self.secili_kitap = None;
                            self.kategorileri_yukle();
                        }
                        for k in self.kitaplar.clone() {
                            if ui
                                .selectable_label(false, format!("{} ({}/{})", k.ad, k.ay, k.yil))
                                .clicked()
                            {
                                self.secili_kitap = Some(k);
                                self.kategorileri_yukle();
                            }
                        }
                    });
                if !kompakt_duzen {
                    ui.label(
                        RichText::new("Arama yapılacak fiyat kaynağı")
                            .color(tema::METIN_SOLUK)
                            .size(12.0),
                    );
                }
            });
        });
        ui.add_space(8.0);

        if kompakt_duzen {
            let panel_frame = egui::Frame::default()
                .fill(tema::YUZEY)
                .stroke(egui::Stroke::new(1.0, tema::KENAR_YUMUSAK))
                .inner_margin(egui::Margin::same(9));
            egui::SidePanel::left("grup_panel_kompakt")
                .frame(panel_frame)
                .resizable(true)
                .default_width(235.0)
                .min_width(205.0)
                .max_width(300.0)
                .show_inside(ui, |ui| {
                    self.render_is_gruplari_paneli(ui);
                });
            egui::TopBottomPanel::top("arama_panel_kompakt")
                .frame(panel_frame)
                .resizable(true)
                .default_height(180.0)
                .min_height(135.0)
                .show_inside(ui, |ui| {
                    self.render_arama_paneli(ui);
                });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .fill(tema::ARKA_PLAN)
                        .inner_margin(egui::Margin {
                            left: 10,
                            right: 0,
                            top: 10,
                            bottom: 0,
                        }),
                )
                .show_inside(ui, |ui| {
                    self.render_metraj_listesi(ui);
                });
            return;
        }

        let panel_frame = egui::Frame::default()
            .fill(tema::YUZEY)
            .inner_margin(egui::Margin::same(10));
        egui::SidePanel::left("sol_panel")
            .frame(panel_frame)
            .resizable(true)
            .default_width(360.0)
            .min_width(300.0)
            .show_inside(ui, |ui| {
                self.render_arama_paneli(ui);
            });
        egui::SidePanel::left("grup_panel")
            .frame(panel_frame)
            .resizable(true)
            .default_width(245.0)
            .min_width(210.0)
            .show_inside(ui, |ui| {
                self.render_is_gruplari_paneli(ui);
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(tema::ARKA_PLAN)
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 0,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show_inside(ui, |ui| {
                self.render_metraj_listesi(ui);
            });
    }

    pub(crate) fn render_metraj_onaylari(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.silinecek_grup_id.clone() {
            let (ad, kalem_sayisi) = grup_bul_ref(&self.is_gruplari, &id)
                .map(|grup| (grup.ad.clone(), grup.tum_kalemler_duz().len()))
                .unwrap_or_else(|| ("Seçili grup".into(), 0));
            let mut sil = false;
            let mut vazgec = false;
            egui::Window::new("İş grubunu sil")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_width(420.0);
                    ui.label(RichText::new(&ad).strong().size(15.0));
                    ui.label(
                        RichText::new(format!(
                            "Bu grup, alt grupları ve içindeki {} metraj kalemi silinecek.",
                            kalem_sayisi
                        ))
                        .color(tema::METIN_IKINCIL),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("İşlemi hemen Ctrl+Z ile geri alabilirsiniz.")
                            .color(tema::UYARI)
                            .size(12.0),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if tema::tehlike_buton(ui, "Grubu Sil").clicked() {
                            sil = true;
                        }
                        if ui.button("Vazgeç").clicked() {
                            vazgec = true;
                        }
                    });
                });

            if sil {
                self.anlik_goruntu_al();
                grup_sil(&mut self.is_gruplari, &id);
                self.secili_grup_id = None;
                self.metraj_kalemleri.clear();
                if let Some(yeni_id) = ilk_yaprak_grup_id(&self.is_gruplari) {
                    self.grup_sec(yeni_id);
                }
                self.degisiklik_var = true;
                self.silinecek_grup_id = None;
                self.basarili_mesaj = format!("'{}' grubu silindi.", ad);
            } else if vazgec {
                self.silinecek_grup_id = None;
            }
        }

        if self.metraj_temizleme_onayi {
            let kalem_sayisi = self.metraj_kalemleri.len();
            let grup_adi = self
                .secili_grup_id
                .as_ref()
                .and_then(|id| grup_bul_ref(&self.is_gruplari, id))
                .map(|grup| grup.ad.clone())
                .unwrap_or_else(|| "Seçili grup".into());
            let mut temizle = false;
            let mut vazgec = false;
            egui::Window::new("Metraj grubunu temizle")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_width(420.0);
                    ui.label(RichText::new(&grup_adi).strong().size(15.0));
                    ui.label(
                        RichText::new(format!(
                            "Bu gruptaki {} metraj kaleminin tamamı kaldırılacak.",
                            kalem_sayisi
                        ))
                        .color(tema::METIN_IKINCIL),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("İşlemi hemen Ctrl+Z ile geri alabilirsiniz.")
                            .color(tema::UYARI)
                            .size(12.0),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if tema::tehlike_buton(ui, "Tümünü Temizle").clicked() {
                            temizle = true;
                        }
                        if ui.button("Vazgeç").clicked() {
                            vazgec = true;
                        }
                    });
                });

            if temizle {
                self.anlik_goruntu_al();
                self.metraj_kalemleri.clear();
                self.aktif_grubu_senkronize();
                self.degisiklik_var = true;
                self.metraj_temizleme_onayi = false;
                self.basarili_mesaj = format!("'{}' grubu temizlendi.", grup_adi);
            } else if vazgec {
                self.metraj_temizleme_onayi = false;
            }
        }
    }

    pub(crate) fn render_is_gruplari_paneli(&mut self, ui: &mut Ui) {
        tema::ikonlu_bolum_basligi(ui, tema::ikon::ALT_GRUP, "İş Grupları");
        ui.add_space(4.0);

        tema::kart(ui, |ui| {
            ui.add(
                TextEdit::singleline(&mut self.yeni_grup_adi)
                    .hint_text("Yeni grup adı…")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if tema::birincil_ikonlu_buton(ui, tema::ikon::ANA_GRUP, "Ana Grup").clicked() {
                    let ad = self.yeni_grup_adi.trim().to_string();
                    if ad.is_empty() {
                        self.hata_mesaji = "Grup adı boş olamaz.".into();
                    } else {
                        self.anlik_goruntu_al();
                        self.is_gruplari.push(IsGrubu::yeni(&ad));
                        self.yeni_grup_adi.clear();
                        self.degisiklik_var = true;
                        self.hata_mesaji.clear();
                    }
                }
                if ui
                    .button(tema::ikonlu_metin(tema::ikon::ALT_GRUP, "Alt Grup"))
                    .clicked()
                {
                    let ad = self.yeni_grup_adi.trim().to_string();
                    match (self.secili_grup_id.clone(), ad.is_empty()) {
                        (None, _) => self.hata_mesaji = "Önce bir üst grup seçin.".into(),
                        (_, true) => self.hata_mesaji = "Grup adı boş olamaz.".into(),
                        (Some(ust_id), false) => {
                            self.anlik_goruntu_al();
                            if let Some(ust) = grup_bul_mut(&mut self.is_gruplari, &ust_id) {
                                ust.alt_gruplar.push(IsGrubu::yeni(&ad));
                            }
                            self.yeni_grup_adi.clear();
                            self.degisiklik_var = true;
                            self.hata_mesaji.clear();
                        }
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("✏ Adlandır").clicked() {
                    let ad = self.yeni_grup_adi.trim().to_string();
                    match (self.secili_grup_id.clone(), ad.is_empty()) {
                        (None, _) => self.hata_mesaji = "Önce bir grup seçin.".into(),
                        (_, true) => self.hata_mesaji = "Yeni ad boş olamaz.".into(),
                        (Some(id), false) => {
                            self.anlik_goruntu_al();
                            if let Some(g) = grup_bul_mut(&mut self.is_gruplari, &id) {
                                g.ad = ad;
                            }
                            self.yeni_grup_adi.clear();
                            self.degisiklik_var = true;
                            self.hata_mesaji.clear();
                        }
                    }
                }
                if tema::tehlike_buton(ui, "🗑 Sil").clicked() {
                    if let Some(id) = self.secili_grup_id.clone() {
                        self.silinecek_grup_id = Some(id);
                    } else {
                        self.hata_mesaji = "Silinecek grubu seçin.".into();
                    }
                }
            });
        });
        ui.add_space(8.0);

        if self.is_gruplari.is_empty() {
            ui.label(
                RichText::new("Henüz iş grubu yok.\nYukarıdan ekleyin.")
                    .color(tema::METIN_SOLUK)
                    .size(12.0),
            );
            return;
        }

        let secili = self.secili_grup_id.clone();
        let mut secilen: Option<String> = None;
        ScrollArea::vertical().show(ui, |ui| {
            is_grubu_agac_ciz(
                ui,
                &self.is_gruplari,
                secili.as_deref(),
                &self.metraj_kalemleri,
                &mut secilen,
            );
        });
        if let Some(id) = secilen {
            self.grup_sec(id);
        }
    }

    pub(crate) fn render_arama_paneli(&mut self, ui: &mut Ui) {
        tema::ikonlu_bolum_basligi(ui, tema::ikon::POZLAR, "Poz Arama");
        ui.add_space(4.0);

        tema::kart(ui, |ui| {
            ui.add(
                TextEdit::singleline(&mut self.akilli_arama_metni)
                    .hint_text("⚡ Hızlı ara: 15.180 veya plywood kalıp")
                    .desired_width(f32::INFINITY),
            )
            .changed()
            .then(|| self.akilli_ara());
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Poz No")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                if ui
                    .add_sized(
                        Vec2::new(110.0, 26.0),
                        TextEdit::singleline(&mut self.poz_arama_metni).hint_text("15.100"),
                    )
                    .changed()
                {
                    self.akilli_arama_metni.clear();
                    self.poz_no_ara();
                }
                ui.label(
                    RichText::new("Açıklama")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                if ui
                    .add_sized(
                        Vec2::new(ui.available_width(), 26.0),
                        TextEdit::singleline(&mut self.aciklama_arama_metni).hint_text("beton"),
                    )
                    .changed()
                {
                    self.akilli_arama_metni.clear();
                    if self.aciklama_arama_metni.is_empty() {
                        self.arama_sonuclari.clear();
                    } else {
                        self.aciklama_ara();
                    }
                }
            });
            if !self.kategoriler.is_empty() {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Kategori")
                            .color(tema::METIN_IKINCIL)
                            .size(12.0),
                    );
                    egui::ComboBox::from_id_salt("kategori_combo")
                        .selected_text(&self.secili_kategori)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(false, "TÜMÜ").clicked() {
                                self.secili_kategori = "TÜMÜ".into();
                                self.kategori_pozlar.clear();
                            }
                            for kat in &self.kategoriler.clone() {
                                if ui.selectable_label(false, kat).clicked() {
                                    self.secili_kategori = kat.clone();
                                    self.kategori_filtrele();
                                }
                            }
                        });
                });
            }
        });
        ui.add_space(8.0);

        let pl = if !self.kategori_pozlar.is_empty() {
            &self.kategori_pozlar
        } else {
            &self.arama_sonuclari
        };
        let arama_var = !self.akilli_arama_metni.is_empty()
            || !self.poz_arama_metni.is_empty()
            || !self.aciklama_arama_metni.is_empty();
        if !pl.is_empty() {
            ui.label(
                RichText::new(format!("{} sonuç", pl.len()))
                    .color(tema::METIN_SOLUK)
                    .size(12.0),
            );
        } else if arama_var {
            ui.label(
                RichText::new("Sonuç bulunamadı.")
                    .color(tema::METIN_SOLUK)
                    .size(12.0),
            );
        } else {
            ui.label(
                RichText::new("👆 Yukarıdan arama yapın")
                    .color(tema::METIN_SOLUK)
                    .size(12.0),
            );
        }
        ui.add_space(4.0);

        self.cift_tiklama_ekle = false;
        let secili_poz_var = self.secili_poz.is_some();
        let liste_yuksekligi = if secili_poz_var {
            (ui.available_height() - 160.0).max(120.0)
        } else {
            ui.available_height() - 8.0
        };
        ScrollArea::vertical()
            .max_height(liste_yuksekligi)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for poz in pl.iter() {
                    let secili = self
                        .secili_poz
                        .as_ref()
                        .map(|s| s.poz_no == poz.poz_no && s.kitap_id == poz.kitap_id)
                        .unwrap_or(false);
                    let fm = match poz.fiyat {
                        Some(f) => format!("{} TL", para_formatla(f)),
                        None => "Formül".into(),
                    };
                    let fiyat_rengi = if poz.fiyat.is_some() {
                        tema::BASARI
                    } else {
                        tema::UYARI
                    };

                    let cerceve = egui::Frame::default()
                        .fill(if secili {
                            tema::VURGU_SOLUK
                        } else {
                            tema::YUZEY_2
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if secili {
                                tema::VURGU
                            } else {
                                tema::KENAR_YUMUSAK
                            },
                        ))
                        .corner_radius(egui::CornerRadius::same(tema::KOSE_KUCUK))
                        .inner_margin(egui::Margin::symmetric(9, 7));
                    let ic = cerceve.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&poz.poz_no)
                                    .monospace()
                                    .size(12.0)
                                    .strong()
                                    .color(if secili { Color32::WHITE } else { tema::METIN }),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(fm).size(11.5).strong().color(fiyat_rengi),
                                    );
                                    ui.label(
                                        RichText::new(&poz.birim)
                                            .size(11.0)
                                            .color(tema::METIN_SOLUK),
                                    );
                                },
                            );
                        });
                        ui.label(RichText::new(&poz.tanim).size(11.5).color(if secili {
                            tema::METIN
                        } else {
                            tema::METIN_IKINCIL
                        }));
                    });
                    let response = ic.response.interact(egui::Sense::click());
                    if response.clicked() {
                        self.secili_poz = Some(poz.clone());
                        self.yeni_poz_no = poz.poz_no.clone();
                    }
                    if response.double_clicked() {
                        self.secili_poz = Some(poz.clone());
                        self.yeni_poz_no = poz.poz_no.clone();
                        self.cift_tiklama_ekle = true;
                    }
                    response.on_hover_text(format!(
                        "{}/{} | {}\nÇift tıkla: metraja ekle",
                        poz.ay, poz.yil, poz.tanim
                    ));
                    ui.add_space(4.0);
                }
            });

        if self.cift_tiklama_ekle {
            self.kalem_ekle();
        }

        let mut secili_poz_ekle = false;
        let mut nakliye_ac = false;
        if let Some(poz) = self.secili_poz.clone() {
            ui.add_space(6.0);
            tema::kart(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("📌 Seçili Poz")
                            .color(tema::METIN_IKINCIL)
                            .size(12.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match poz.fiyat {
                            Some(f) => ui.label(
                                RichText::new(format!("{} TL", para_formatla(f)))
                                    .color(tema::BASARI)
                                    .strong()
                                    .size(14.0),
                            ),
                            None => ui.label(RichText::new("Formül").color(tema::UYARI).strong()),
                        };
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&poz.poz_no)
                            .monospace()
                            .strong()
                            .size(15.0)
                            .color(tema::METIN),
                    );
                    ui.label(
                        RichText::new(format!(
                            "· {} · {} ({}/{})",
                            poz.birim, poz.kitap_adi, poz.ay, poz.yil
                        ))
                        .color(tema::METIN_SOLUK)
                        .size(11.5),
                    );
                });
                ui.label(
                    RichText::new(&poz.tanim)
                        .size(12.0)
                        .color(tema::METIN_IKINCIL),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if tema::birincil_buton(ui, "＋ Metraja Ekle").clicked() {
                        secili_poz_ekle = true;
                    }
                    if ui
                        .button("🚚 Nakliye")
                        .on_hover_text("Taşıma bedeli hesapla: birim fiyat × miktar × mesafe")
                        .clicked()
                    {
                        nakliye_ac = true;
                    }
                });
            });
        }
        if secili_poz_ekle {
            self.kalem_ekle();
        }
        if nakliye_ac {
            self.nakliye_popup_ac();
        }
    }

    pub(crate) fn render_metraj_listesi(&mut self, ui: &mut Ui) {
        let aktif_grup_adi = self
            .secili_grup_id
            .as_ref()
            .and_then(|id| grup_bul_ref(&self.is_gruplari, id))
            .map(|g| g.ad.clone());

        // Başlık ve bağlam
        ui.horizontal_wrapped(|ui| {
            ui.label(tema::ikonlu_metin_boyut(
                tema::ikon::METRAJ_TABLOSU,
                "Metraj Tablosu",
                16.0,
            ));
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(3.0);
            ui.label(
                RichText::new("Seçili grup:")
                    .size(12.0)
                    .color(tema::METIN_IKINCIL),
            );
            match &aktif_grup_adi {
                Some(ad) => {
                    ui.label(
                        RichText::new(tema::ikon::ALT_GRUP)
                            .font(egui::FontId::new(13.0, tema::ikon_fontu()))
                            .color(tema::VURGU_HOVER),
                    );
                    ui.label(RichText::new(ad).size(12.5).strong().color(tema::METIN));
                }
                None if !self.is_gruplari.is_empty() => {
                    ui.label(
                        RichText::new("Grup seçili değil")
                            .size(12.5)
                            .strong()
                            .color(tema::UYARI),
                    );
                }
                None => {}
            }
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if tema::basari_buton(ui, "💾 Kaydet").clicked() {
                self.metraj_kaydet();
            }
            if ui.button("📂 Aç").clicked() {
                self.metraj_yukle_diyalog();
            }
            ui.separator();
            if ui.button("📊 Excel").clicked() {
                self.metraj_excel_diyalog();
            }
            ui.menu_button(tema::ikonlu_metin(tema::ikon::AKTAR, "Aktar"), |ui| {
                if ui.button("📤 CSV dışa aktar").clicked() {
                    self.metraj_csv_diyalog();
                    ui.close_menu();
                }
                if ui.button("📥 CSV içe aktar").clicked() {
                    self.metraj_csv_ice_aktar_diyalog();
                    ui.close_menu();
                }
                if ui.button("📋 Özeti panoya kopyala").clicked() {
                    let m = self.proje_olustur();
                    let ozet = crate::export::metraj_metin_ozet(&m);
                    ui.ctx().copy_text(ozet);
                    self.basarili_mesaj = "Metraj özeti panoya kopyalandı.".into();
                    ui.close_menu();
                }
            });
            if ui
                .add_enabled(
                    !self.metraj_kalemleri.is_empty(),
                    egui::Button::new("🗑 Grubu Temizle"),
                )
                .on_hover_text("Seçili gruptaki tüm metraj kalemlerini kaldırır")
                .clicked()
            {
                self.metraj_temizleme_onayi = true;
            }
        });
        ui.add_space(8.0);

        // Giriş kartı: metraj adı + hızlı poz ekleme + toplu fiyat
        tema::kart(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Metraj Adı")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                if ui
                    .add(
                        TextEdit::singleline(&mut self.metraj_adi)
                            .hint_text("Proje / metraj adı")
                            .desired_width(240.0),
                    )
                    .changed()
                {
                    self.degisiklik_var = true;
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Poz No")
                        .color(tema::METIN_IKINCIL)
                        .size(12.0),
                );
                let poz_no_response = ui.add(
                    TextEdit::singleline(&mut self.yeni_poz_no)
                        .hint_text("15.100.1001")
                        .desired_width(140.0),
                );
                if poz_no_response.changed() {
                    self.poz_sorgula();
                }
                if poz_no_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.poz_sorgula();
                    self.kalem_ekle();
                }
                if tema::birincil_buton(ui, "＋ Kalem Ekle").clicked() {
                    self.poz_sorgula();
                    self.kalem_ekle();
                }
            });
            // Rayiç güncelleme: kuruma/döneme göre yeniden fiyatlandırma veya Yİ-ÜFE endeksi
            if !self.metraj_kalemleri.is_empty() || !self.is_gruplari.is_empty() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("🔄 Rayiç Güncelle").on_hover_text("Tüm kalemleri kuruma/döneme göre veya Yİ-ÜFE endeksiyle topluca güncelle").clicked() {
                        self.fiyat_guncelle_acik = true;
                    }
                    ui.label(RichText::new("İhale tarihine göre rayiç veya endeks güncellemesi").color(tema::METIN_SOLUK).size(11.0));
                });
            }
        });
        ui.add_space(8.0);

        self.render_metraj_ozetleri(ui);
        ui.add_space(8.0);

        ScrollArea::both()
            .max_height(ui.available_height() - 64.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.render_metraj_kalem_tablosu(ui);
            });

        // Alt toplam çubuğu
        ui.add_space(6.0);
        egui::Frame::default()
            .fill(tema::YUZEY_2)
            .stroke(egui::Stroke::new(1.0, tema::KENAR))
            .corner_radius(egui::CornerRadius::same(tema::KOSE))
            .inner_margin(egui::Margin::symmetric(14, 9))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if !self.is_gruplari.is_empty() && self.secili_grup_id.is_some() {
                        let alt_toplam: f64 = self.metraj_kalemleri.iter().map(|k| k.tutar).sum();
                        ui.label(
                            RichText::new("Grup Alt Toplamı")
                                .color(tema::METIN_SOLUK)
                                .size(12.0),
                        );
                        ui.label(
                            RichText::new(format!("{} TL", para_formatla(alt_toplam)))
                                .size(14.0)
                                .strong()
                                .color(tema::VURGU_HOVER),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} TL", para_formatla(self.toplam_tutar())))
                                .size(19.0)
                                .strong()
                                .color(tema::BASARI),
                        );
                        ui.label(
                            RichText::new("GENEL TOPLAM")
                                .color(tema::METIN_IKINCIL)
                                .size(13.0)
                                .strong(),
                        );
                    });
                });
            });
    }

    /// Rayiç güncelleme modalı: kuruma/döneme göre yeniden fiyatlandırma veya Yİ-ÜFE endeksi.
    pub(crate) fn render_fiyat_guncelle_popup(&mut self, ctx: &egui::Context) {
        if !self.fiyat_guncelle_acik {
            return;
        }
        let mut uygula = false;
        let mut kapat = false;
        egui::Window::new("🔄 Rayiç / Fiyat Güncelleme")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(460.0);
                ui.label(
                    RichText::new("Metrajdaki tüm kalemlerin birim fiyatlarını topluca günceller.")
                        .color(tema::METIN_SOLUK)
                        .size(11.5),
                );
                ui.add_space(8.0);

                // Kip seçimi
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(!self.fiyat_guncelle_endeks_mod, "🏛 Kuruma göre")
                        .clicked()
                    {
                        self.fiyat_guncelle_endeks_mod = false;
                    }
                    if ui
                        .selectable_label(
                            self.fiyat_guncelle_endeks_mod,
                            "📈 Endekse göre (Yİ-ÜFE)",
                        )
                        .clicked()
                    {
                        self.fiyat_guncelle_endeks_mod = true;
                    }
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                if self.fiyat_guncelle_endeks_mod {
                    ui.label(
                        RichText::new("Fiyatları Yİ-ÜFE oranıyla güncelle (güncel ÷ temel).")
                            .size(12.5)
                            .color(tema::METIN),
                    );
                    ui.add_space(6.0);
                    egui::Grid::new("fg_endeks_grid")
                        .num_columns(2)
                        .spacing(egui::vec2(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Temel endeks (sözleşme/eski)")
                                    .color(tema::METIN_IKINCIL)
                                    .size(12.5),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.fiyat_endeks_temel)
                                    .speed(0.1)
                                    .range(0.01..=1_000_000.0),
                            );
                            ui.end_row();
                            ui.label(
                                RichText::new("Güncel endeks (yeni)")
                                    .color(tema::METIN_IKINCIL)
                                    .size(12.5),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.fiyat_endeks_guncel)
                                    .speed(0.1)
                                    .range(0.01..=1_000_000.0),
                            );
                            ui.end_row();
                        });
                    let carpan = if self.fiyat_endeks_temel > 0.0 {
                        self.fiyat_endeks_guncel / self.fiyat_endeks_temel
                    } else {
                        0.0
                    };
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "Çarpan: × {:.4}   (% {:+.1})",
                            carpan,
                            (carpan - 1.0) * 100.0
                        ))
                        .color(tema::VURGU_HOVER)
                        .strong(),
                    );
                } else {
                    let hedef_metni = self
                        .fiyat_guncelle_hedef
                        .as_ref()
                        .map(|k| k.ad.clone())
                        .unwrap_or_else(|| "Kurum seçin".into());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Hedef kurum")
                                .color(tema::METIN_IKINCIL)
                                .size(12.5),
                        );
                        egui::ComboBox::from_id_salt("fg_kurum")
                            .selected_text(&hedef_metni)
                            .width(260.0)
                            .show_ui(ui, |ui| {
                                for k in &self.kitaplar.clone() {
                                    let secili = self
                                        .fiyat_guncelle_hedef
                                        .as_ref()
                                        .map(|h| h.id == k.id)
                                        .unwrap_or(false);
                                    if ui.selectable_label(secili, &k.ad).clicked() {
                                        self.fiyat_guncelle_hedef = Some(k.clone());
                                    }
                                }
                            });
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.fiyat_guncelle_en_son, "En son fiyat")
                            .clicked()
                        {
                            self.fiyat_guncelle_en_son = true;
                        }
                        if ui
                            .selectable_label(!self.fiyat_guncelle_en_son, "İhale tarihine göre")
                            .clicked()
                        {
                            self.fiyat_guncelle_en_son = false;
                        }
                    });
                    if !self.fiyat_guncelle_en_son {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Dönem").color(tema::METIN_IKINCIL).size(12.5));
                            ui.add(
                                egui::DragValue::new(&mut self.fiyat_guncelle_ay)
                                    .range(1..=12)
                                    .prefix("Ay "),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.fiyat_guncelle_yil)
                                    .range(2000..=2100),
                            );
                        });
                        ui.label(
                            RichText::new(
                                "O tarihte geçerli (tarih ve öncesindeki en son) rayiç uygulanır.",
                            )
                            .color(tema::METIN_SOLUK)
                            .size(10.5),
                        );
                    }
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if tema::basari_buton(ui, "✓ Güncelle").clicked() {
                        uygula = true;
                    }
                    if ui.button("İptal").clicked() {
                        kapat = true;
                    }
                });
            });
        if uygula {
            self.fiyatlari_guncelle();
        }
        if kapat {
            self.fiyat_guncelle_acik = false;
        }
    }

    pub(crate) fn render_metraj_kalem_tablosu(&mut self, ui: &mut Ui) {
        if self.metraj_kalemleri.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("📋").size(32.0));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Bu grupta henüz kalem yok")
                        .color(tema::METIN_IKINCIL)
                        .size(14.0),
                );
                ui.label(
                    RichText::new("Soldan bir poz arayıp “Metraja Ekle” ile başlayın")
                        .color(tema::METIN_SOLUK)
                        .size(12.0),
                );
            });
            return;
        }
        let mut popup_acilacak: Option<usize> = None;
        let baslik = |ui: &mut egui::Ui, t: &str| {
            ui.label(
                RichText::new(t)
                    .strong()
                    .size(12.0)
                    .color(tema::METIN_IKINCIL),
            );
        };
        egui::Grid::new("metraj_grid")
            .num_columns(9)
            .min_col_width(40.0)
            .spacing(egui::vec2(10.0, 8.0))
            .striped(true)
            .show(ui, |ui: &mut egui::Ui| {
                baslik(ui, "#");
                baslik(ui, "Poz No");
                baslik(ui, "Açıklama");
                baslik(ui, "Kitap");
                baslik(ui, "Birim");
                baslik(ui, "B.Fiyat");
                baslik(ui, "Miktar");
                baslik(ui, "Tutar");
                baslik(ui, "");
                ui.end_row();

                let mut sil: Option<usize> = None;
                for (idx, kalem) in self.metraj_kalemleri.iter().enumerate() {
                    ui.label(
                        RichText::new(format!("{}", idx + 1))
                            .color(tema::METIN_SOLUK)
                            .size(11.0),
                    );
                    let poz_response = ui.label(
                        RichText::new(&kalem.poz_no)
                            .size(11.5)
                            .monospace()
                            .color(tema::METIN),
                    );
                    let imalatli = !kalem.imalat_cinsi.trim().is_empty();
                    let birincil = if imalatli {
                        metni_kisalt(&kalem.imalat_cinsi, 46)
                    } else {
                        metni_kisalt(&kalem.tanim, 46)
                    };
                    let hover = if imalatli {
                        format!("İmalat cinsi: {}\nPoz: {}", kalem.imalat_cinsi, kalem.tanim)
                    } else {
                        kalem.tanim.clone()
                    };
                    let aciklama_response = ui
                        .label(
                            RichText::new(birincil)
                                .size(11.5)
                                .color(tema::METIN_IKINCIL),
                        )
                        .on_hover_text(hover);
                    let kitap_kisa = metni_kisalt(&kalem.kitap_adi, 18);
                    ui.label(
                        RichText::new(kitap_kisa)
                            .size(10.5)
                            .color(tema::METIN_SOLUK),
                    )
                    .on_hover_text(&kalem.kitap_adi);
                    ui.label(
                        RichText::new(&kalem.birim)
                            .size(11.0)
                            .color(tema::METIN_IKINCIL),
                    );
                    ui.label(
                        RichText::new(para_formatla(kalem.birim_fiyat))
                            .size(11.5)
                            .color(tema::METIN_IKINCIL),
                    );
                    let miktar_renk = if kalem.miktar > 0.0 {
                        tema::METIN
                    } else {
                        tema::UYARI
                    };
                    let miktar_metni = if kalem.detaylar.is_empty() {
                        format!("{:.2}", kalem.miktar)
                    } else {
                        format!("📐 {:.2}", kalem.miktar)
                    };
                    let miktar_response = ui
                        .label(
                            RichText::new(miktar_metni)
                                .size(11.5)
                                .strong()
                                .color(miktar_renk),
                        )
                        .on_hover_text(if kalem.detaylar.is_empty() {
                            "Ölçü detayı yok — düzenlemek için tıkla"
                        } else {
                            "Ölçü kırılımı var — düzenlemek için tıkla"
                        });
                    ui.label(
                        RichText::new(para_formatla(kalem.tutar))
                            .size(11.5)
                            .strong()
                            .color(tema::BASARI),
                    );
                    if ui
                        .add(
                            egui::Button::new(RichText::new("✕").color(tema::TEHLIKE).size(11.0))
                                .fill(Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE),
                        )
                        .on_hover_text("Kalemi sil")
                        .clicked()
                    {
                        sil = Some(idx);
                    }
                    let satir_response =
                        poz_response.union(aciklama_response).union(miktar_response);
                    if satir_response
                        .on_hover_text("Miktar detaylarını düzenle")
                        .clicked()
                    {
                        popup_acilacak = Some(idx);
                    }
                    ui.end_row();
                }
                if let Some(idx) = sil {
                    self.anlik_goruntu_al();
                    self.metraj_kalemleri.remove(idx);
                    self.aktif_grubu_senkronize();
                    self.degisiklik_var = true;
                }
            });
        if let Some(idx) = popup_acilacak {
            self.popup_kalem_indeks = Some(idx);
            self.popup_detaylar = self.metraj_kalemleri[idx]
                .detaylar
                .iter()
                .map(detay_to_satir)
                .collect();
            self.popup_imalat_cinsi = self.metraj_kalemleri[idx].imalat_cinsi.clone();
            self.popup_yeni = PopupDetaySatiri::default();
            self.miktar_popup_acik = true;
        }
    }

    pub(crate) fn render_miktar_popup(&mut self, ctx: &egui::Context) {
        if !self.miktar_popup_acik {
            return;
        }
        let idx = match self.popup_kalem_indeks {
            Some(i) if i < self.metraj_kalemleri.len() => i,
            _ => {
                self.miktar_popup_acik = false;
                return;
            }
        };
        let kalem = &self.metraj_kalemleri[idx];
        let poz_no = kalem.poz_no.clone();
        let tanim = kalem.tanim.clone();
        let birim = kalem.birim.clone();
        let birim_fiyat = kalem.birim_fiyat;
        let kitap_adi = kalem.kitap_adi.clone();

        egui::Window::new("📐 Miktar Detayları")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&poz_no).monospace().strong().size(16.0));
                    ui.label(RichText::new(&tanim).size(14.0));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Birim: {}", birim)).color(tema::METIN_IKINCIL));
                    ui.label(RichText::new(format!("Birim Fiyat: {} TL", para_formatla(birim_fiyat))).color(tema::BASARI));
                    ui.label(RichText::new(format!("· 📅 {}", kitap_adi)).color(tema::METIN_SOLUK).size(12.0));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("İmalat cinsi").size(12.0).color(tema::METIN_IKINCIL));
                    ui.add(TextEdit::singleline(&mut self.popup_imalat_cinsi).hint_text("ör. Zemin kat perde duvarları").desired_width(380.0));
                });
                ui.separator();

                ui.label(RichText::new("Ölçü detayları  ·  boş bırakılan boyut 1 sayılır  ·  “çıkan” işaretli satırlar düşülür").color(tema::METIN_SOLUK).size(11.5));
                ui.add_space(3.0);
                let bsl = |ui: &mut egui::Ui, t: &str| { ui.label(RichText::new(t).strong().size(11.5).color(tema::METIN_IKINCIL)); };
                egui::Grid::new("popup_detay_grid").num_columns(9).spacing(egui::vec2(7.0, 6.0)).striped(true).show(ui, |ui| {
                    bsl(ui, "#"); bsl(ui, "Açıklama"); bsl(ui, "Adet"); bsl(ui, "En"); bsl(ui, "Boy"); bsl(ui, "Yük."); bsl(ui, "Çıkan"); bsl(ui, "= Miktar"); bsl(ui, "");
                    ui.end_row();

                    let mut silinecek_satir: Option<usize> = None;
                    for (d_idx, satir) in self.popup_detaylar.iter_mut().enumerate() {
                        ui.label(RichText::new(format!("{}", d_idx + 1)).color(tema::METIN_SOLUK).size(11.0));
                        ui.add(TextEdit::singleline(&mut satir.aciklama).desired_width(170.0).hint_text("açıklama"));
                        ui.add(TextEdit::singleline(&mut satir.adet).desired_width(48.0));
                        ui.add(TextEdit::singleline(&mut satir.en).desired_width(48.0));
                        ui.add(TextEdit::singleline(&mut satir.boy).desired_width(48.0));
                        ui.add(TextEdit::singleline(&mut satir.yukseklik).desired_width(48.0));
                        ui.checkbox(&mut satir.cikan, "");
                        let m = satir_miktar(satir).unwrap_or(0.0);
                        let renk = if m < 0.0 { tema::UYARI } else { tema::BASARI };
                        ui.label(RichText::new(format!("{:.3}", m)).size(11.5).strong().color(renk));
                        if ui.add(egui::Button::new(RichText::new("🗑").color(tema::TEHLIKE).size(11.0)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::NONE)).clicked() {
                            silinecek_satir = Some(d_idx);
                        }
                        ui.end_row();
                    }
                    if let Some(s) = silinecek_satir { self.popup_detaylar.remove(s); }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Yeni satır").color(tema::METIN_IKINCIL).size(11.5));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.aciklama).hint_text("açıklama").desired_width(160.0));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.adet).hint_text("adet").desired_width(48.0));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.en).hint_text("en").desired_width(48.0));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.boy).hint_text("boy").desired_width(48.0));
                    ui.add(TextEdit::singleline(&mut self.popup_yeni.yukseklik).hint_text("yük.").desired_width(48.0));
                    ui.checkbox(&mut self.popup_yeni.cikan, "çıkan");
                    let ekle = tema::birincil_buton(ui, "＋ Ekle").clicked();
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (ekle || enter) && satir_miktar(&self.popup_yeni).is_some() {
                        self.popup_detaylar.push(std::mem::take(&mut self.popup_yeni));
                    }
                });
                ui.separator();

                let toplam_miktar: f64 = self.popup_detaylar.iter().filter_map(satir_miktar).sum();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Toplam Miktar: {:.3} {}", toplam_miktar, birim)).size(14.0).strong().color(tema::BASARI));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("≈ {} TL", para_formatla(toplam_miktar * birim_fiyat))).size(13.0).color(tema::METIN_IKINCIL));
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if tema::basari_buton(ui, "✓ Tamam").clicked() {
                        let detaylar: Vec<MiktarDetay> = self.popup_detaylar.iter()
                            .filter_map(satir_to_detay)
                            .collect();
                        if detaylar.len() != self.popup_detaylar.len() {
                            self.hata_mesaji = "Ölçü satırlarında geçersiz sayı var. Lütfen kırmızı/bozuk alanları düzeltin.".into();
                            return;
                        }
                        let imalat = self.popup_imalat_cinsi.clone();
                        self.anlik_goruntu_al();
                        if let Some(kalem) = self.metraj_kalemleri.get_mut(idx) {
                            kalem.detaylar = detaylar;
                            kalem.imalat_cinsi = imalat;
                            kalem.detaylardan_miktar_hesapla();
                            self.degisiklik_var = true;
                        }
                        self.aktif_grubu_senkronize();
                        self.miktar_popup_acik = false;
                    }
                    if ui.button("❌ İptal").clicked() {
                        self.miktar_popup_acik = false;
                    }
                });
            });
    }

    pub(crate) fn nakliye_popup_ac(&mut self) {
        if let Some(poz) = self.secili_poz.clone() {
            self.nakliye_poz = Some(poz);
            self.nakliye_miktar.clear();
            self.nakliye_mesafe.clear();
            self.nakliye_popup_acik = true;
        }
    }

    pub(crate) fn render_nakliye_popup(&mut self, ctx: &egui::Context) {
        if !self.nakliye_popup_acik {
            return;
        }
        let poz = match self.nakliye_poz.clone() {
            Some(p) => p,
            None => {
                self.nakliye_popup_acik = false;
                return;
            }
        };
        let birim_fiyat = poz.fiyat.unwrap_or(0.0);
        let mut ekle = false;
        let mut kapat = false;
        egui::Window::new("🚚 Nakliye Hesabı")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&poz.poz_no).monospace().strong().size(15.0));
                    ui.label(
                        RichText::new(metni_kisalt(&poz.tanim, 50))
                            .size(13.0)
                            .color(tema::METIN_IKINCIL),
                    )
                    .on_hover_text(&poz.tanim);
                });
                ui.label(
                    RichText::new(format!(
                        "Taşıma birim fiyatı: {} TL / {}·km",
                        para_formatla(birim_fiyat),
                        poz.birim
                    ))
                    .size(12.0)
                    .color(tema::BASARI),
                );
                ui.label(
                    RichText::new("Taşıma bedeli = birim fiyat × miktar × mesafe")
                        .size(11.0)
                        .color(tema::METIN_SOLUK),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Miktar")
                            .color(tema::METIN_IKINCIL)
                            .size(12.0),
                    );
                    ui.add(
                        TextEdit::singleline(&mut self.nakliye_miktar)
                            .hint_text("120 (ton/m³)")
                            .desired_width(100.0),
                    );
                    ui.label(
                        RichText::new("×  Mesafe (km)")
                            .color(tema::METIN_IKINCIL)
                            .size(12.0),
                    );
                    ui.add(
                        TextEdit::singleline(&mut self.nakliye_mesafe)
                            .hint_text("25")
                            .desired_width(80.0),
                    );
                });
                let m = sayi_oku(&self.nakliye_miktar).unwrap_or(0.0);
                let km = sayi_oku(&self.nakliye_mesafe).unwrap_or(0.0);
                let tasima_miktari = m * km;
                let bedel = birim_fiyat * tasima_miktari;
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "Taşıma miktarı: {:.2} × {:.2} = {:.2}",
                        m, km, tasima_miktari
                    ))
                    .size(12.0)
                    .color(tema::METIN_IKINCIL),
                );
                ui.label(
                    RichText::new(format!("Nakliye bedeli ≈ {} TL", para_formatla(bedel)))
                        .size(15.0)
                        .strong()
                        .color(tema::BASARI),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if tema::basari_buton(ui, "＋ Metraja Ekle").clicked() {
                        ekle = true;
                    }
                    if ui.button("❌ İptal").clicked() {
                        kapat = true;
                    }
                });
            });
        if ekle {
            let m = sayi_oku(&self.nakliye_miktar).unwrap_or(0.0);
            let km = sayi_oku(&self.nakliye_mesafe).unwrap_or(0.0);
            if m > 0.0 && km > 0.0 {
                self.nakliye_kalem_ekle(&poz, m * km, km);
                self.nakliye_popup_acik = false;
            } else {
                self.hata_mesaji = "Miktar ve mesafe (km) girin.".into();
            }
        }
        if kapat {
            self.nakliye_popup_acik = false;
        }
    }

    pub(crate) fn render_metraj_ozetleri(&self, ui: &mut Ui) {
        let toplam_kalem = self.metraj_kalemleri.len();
        let fiyatsiz = self
            .metraj_kalemleri
            .iter()
            .filter(|k| k.birim_fiyat <= 0.0)
            .count();
        let secili_kitap_tutari = self
            .secili_kitap
            .as_ref()
            .map(|kitap| {
                self.metraj_kalemleri
                    .iter()
                    .filter(|k| k.kitap_adi.starts_with(&kitap.ad))
                    .map(|k| k.tutar)
                    .sum::<f64>()
            })
            .unwrap_or(0.0);
        let mut kitap_sayisi: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for kalem in &self.metraj_kalemleri {
            kitap_sayisi.insert(&kalem.kitap_adi);
        }

        ui.horizontal_wrapped(|ui| {
            tema::rozet(
                ui,
                &format!("📋 {} kalem", toplam_kalem),
                tema::METIN_IKINCIL,
            );
            tema::rozet(
                ui,
                &format!("📚 {} kitap", kitap_sayisi.len()),
                tema::METIN_IKINCIL,
            );
            tema::rozet(
                ui,
                &format!("⚠ {} fiyatsız", fiyatsiz),
                if fiyatsiz > 0 {
                    tema::UYARI
                } else {
                    tema::METIN_SOLUK
                },
            );
            if self.secili_kitap.is_some() {
                tema::rozet(
                    ui,
                    &format!("Seçili kitap: {} TL", para_formatla(secili_kitap_tutari)),
                    tema::BASARI,
                );
            }
        });

        if !self.metraj_kalemleri.is_empty() {
            ui.add_space(4.0);
            egui::CollapsingHeader::new(RichText::new("Özet döküm").color(tema::METIN_IKINCIL))
                .default_open(false)
                .show(ui, |ui| {
                    let mut kitap_toplamlari: std::collections::BTreeMap<String, f64> =
                        std::collections::BTreeMap::new();
                    let mut birim_toplamlari: std::collections::BTreeMap<String, f64> =
                        std::collections::BTreeMap::new();
                    for kalem in &self.metraj_kalemleri {
                        *kitap_toplamlari
                            .entry(kalem.kitap_adi.clone())
                            .or_insert(0.0) += kalem.tutar;
                        *birim_toplamlari.entry(kalem.birim.clone()).or_insert(0.0) += kalem.tutar;
                    }
                    ui.columns(2, |cols| {
                        cols[0].label(RichText::new("Kitap").strong());
                        for (kitap, toplam) in kitap_toplamlari.iter().take(6) {
                            cols[0].label(format!(
                                "{}: {} TL",
                                metni_kisalt(kitap, 28),
                                para_formatla(*toplam)
                            ));
                        }
                        cols[1].label(RichText::new("Birim").strong());
                        for (birim, toplam) in birim_toplamlari.iter().take(6) {
                            cols[1].label(format!("{}: {} TL", birim, para_formatla(*toplam)));
                        }
                    });
                });
        }
    }
}

// ==================== İŞ GRUBU AĞAÇ ÇİZİMİ ====================
// İş grupları ağacını çizer; tıklanan grubun id'sini secilen_out'a yazar.
fn is_grubu_agac_ciz(
    ui: &mut Ui,
    gruplar: &[IsGrubu],
    secili_id: Option<&str>,
    aktif_kalemler: &[MetrajKalemi],
    secilen_out: &mut Option<String>,
) {
    for g in gruplar {
        let secili = secili_id == Some(g.id.as_str());
        let toplam = grup_canli_toplam(g, secili_id, aktif_kalemler);
        let yaprak = g.alt_gruplar.is_empty();
        let ikon = if yaprak { "📄" } else { "📁" };
        let ad_rengi = if secili { Color32::WHITE } else { tema::METIN };
        let tutar_rengi = if toplam > 0.0 {
            tema::BASARI
        } else {
            tema::METIN_SOLUK
        };

        let mut job = egui::text::LayoutJob::default();
        job.append(
            &format!("{} ", ikon),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(13.5),
                color: ad_rengi,
                ..Default::default()
            },
        );
        job.append(
            &g.ad,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(13.5),
                color: ad_rengi,
                ..Default::default()
            },
        );
        job.append(
            &format!("   {} TL", para_formatla(toplam)),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(11.5),
                color: tutar_rengi,
                ..Default::default()
            },
        );

        if ui.add(egui::SelectableLabel::new(secili, job)).clicked() {
            *secilen_out = Some(g.id.clone());
        }
        if !yaprak {
            ui.indent(g.id.clone(), |ui| {
                is_grubu_agac_ciz(ui, &g.alt_gruplar, secili_id, aktif_kalemler, secilen_out);
            });
        }
    }
}

// ==================== MİKTAR DETAY (BOYUT) YARDIMCILARI ====================
fn opt_str(o: Option<f64>) -> String {
    o.map(|v| format!("{}", v).replace('.', ","))
        .unwrap_or_default()
}

// Bir popup satırının boyutlarından (işaretli) miktarı hesaplar; hiç boyut yoksa None.
// `cikan` işaretliyse sonuç negatiftir (metrajdan düşülür).
pub(crate) fn satir_miktar(s: &PopupDetaySatiri) -> Option<f64> {
    let a = sayi_oku(&s.adet);
    let e = sayi_oku(&s.en);
    let b = sayi_oku(&s.boy);
    let y = sayi_oku(&s.yukseklik);
    if a.is_none() && e.is_none() && b.is_none() && y.is_none() {
        return None;
    }
    let m = a.unwrap_or(1.0) * e.unwrap_or(1.0) * b.unwrap_or(1.0) * y.unwrap_or(1.0);
    Some(if s.cikan { -m.abs() } else { m })
}

pub(crate) fn satir_to_detay(s: &PopupDetaySatiri) -> Option<MiktarDetay> {
    let m = satir_miktar(s)?;
    Some(MiktarDetay {
        aciklama: s.aciklama.clone(),
        miktar: m,
        adet: sayi_oku(&s.adet),
        en: sayi_oku(&s.en),
        boy: sayi_oku(&s.boy),
        yukseklik: sayi_oku(&s.yukseklik),
        cikan: s.cikan,
    })
}

pub(crate) fn detay_to_satir(d: &MiktarDetay) -> PopupDetaySatiri {
    if d.boyutlu_mu() {
        PopupDetaySatiri {
            aciklama: d.aciklama.clone(),
            adet: opt_str(d.adet),
            en: opt_str(d.en),
            boy: opt_str(d.boy),
            yukseklik: opt_str(d.yukseklik),
            cikan: d.cikan,
        }
    } else {
        // Eski/elle girilmiş detay: miktarı "Adet" sütununa koy
        PopupDetaySatiri {
            aciklama: d.aciklama.clone(),
            adet: if d.miktar != 0.0 {
                opt_str(Some(d.miktar.abs()))
            } else {
                String::new()
            },
            cikan: d.cikan,
            ..Default::default()
        }
    }
}
