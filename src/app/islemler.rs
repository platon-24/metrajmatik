//! UI-dışı iş mantığı: toplam hesabı, aktif grup senkronizasyonu, geri-al/yinele,
//! otomatik kayıt, arama, poz sorgulama, kalem ekleme, dosya (kaydet/yükle/Excel/PDF)
//! ve toplu fiyat güncelleme. Tümü `MetrajApp` üzerinde `pub(crate)` metotlardır.

use eframe::egui;
use std::path::{Path, PathBuf};

use crate::bicim::krono_tarih;
use crate::export::{metraj_csv_aktar, metraj_csv_oku, metraj_excel_aktar, metraj_json_kaydet, metraj_json_yukle, veri_paketi_kaydet, veri_paketi_yukle, AnalizFoyu};
use crate::is_grubu::{grup_bul_mut, grup_bul_ref, grup_canli_toplam, ilk_yaprak_grup_id};
use crate::models::{IsGrubu, KayitliMetraj, MetrajKalemi, Poz};
use crate::pdf_parser::{pdf_metin_cikar, pozlari_ayristir, profil_otomatik_sec, AyristirmaProfili};

use super::{Anlik, MetrajApp};

impl MetrajApp {
    // ==================== YARDIMCI ====================
    pub(crate) fn toplam_tutar(&self) -> f64 {
        if self.is_gruplari.is_empty() {
            return self.metraj_kalemleri.iter().map(|k| k.tutar).sum();
        }
        let secili = self.secili_grup_id.as_deref();
        self.is_gruplari
            .iter()
            .map(|g| grup_canli_toplam(g, secili, &self.metraj_kalemleri))
            .sum()
    }

    // Aktif (seçili) grubun kalemlerini düzenleme tamponundan (metraj_kalemleri) ağaca geri yazar.
    pub(crate) fn aktif_grubu_senkronize(&mut self) {
        if let Some(id) = self.secili_grup_id.clone() {
            if let Some(g) = grup_bul_mut(&mut self.is_gruplari, &id) {
                g.kalemler = self.metraj_kalemleri.clone();
            }
        }
    }

    // Bir grubu aktif yapar: önceki aktif grubu kaydeder, yeni grubun kalemlerini tampona yükler.
    pub(crate) fn grup_sec(&mut self, id: String) {
        if self.secili_grup_id.as_deref() == Some(id.as_str()) {
            return;
        }
        self.aktif_grubu_senkronize();
        let kalemler = grup_bul_ref(&self.is_gruplari, &id)
            .map(|g| g.kalemler.clone())
            .unwrap_or_default();
        self.secili_grup_id = Some(id);
        self.metraj_kalemleri = kalemler;
        self.secili_poz = None;
    }

    // ==================== GERİ AL / YİNELE ====================
    fn mevcut_anlik(&self) -> Anlik {
        Anlik {
            is_gruplari: self.is_gruplari.clone(),
            metraj_kalemleri: self.metraj_kalemleri.clone(),
            secili_grup_id: self.secili_grup_id.clone(),
            metraj_adi: self.metraj_adi.clone(),
        }
    }
    // Değiştiren bir işlemden HEMEN ÖNCE çağrılır: mevcut durumu geri-al yığınına koyar.
    pub(crate) fn anlik_goruntu_al(&mut self) {
        self.aktif_grubu_senkronize();
        let a = self.mevcut_anlik();
        self.geri_al_yigini.push(a);
        if self.geri_al_yigini.len() > 50 {
            self.geri_al_yigini.remove(0);
        }
        self.yinele_yigini.clear();
    }
    fn anlik_uygula(&mut self, a: Anlik) {
        self.is_gruplari = a.is_gruplari;
        self.metraj_kalemleri = a.metraj_kalemleri;
        self.secili_grup_id = a.secili_grup_id;
        self.metraj_adi = a.metraj_adi;
        self.secili_poz = None;
        self.degisiklik_var = true;
    }
    pub(crate) fn geri_al(&mut self) {
        if let Some(a) = self.geri_al_yigini.pop() {
            self.aktif_grubu_senkronize();
            let mevcut = self.mevcut_anlik();
            self.yinele_yigini.push(mevcut);
            self.anlik_uygula(a);
            self.basarili_mesaj = "↩ Geri alındı.".into();
            self.hata_mesaji.clear();
        }
    }
    pub(crate) fn yinele(&mut self) {
        if let Some(a) = self.yinele_yigini.pop() {
            self.aktif_grubu_senkronize();
            let mevcut = self.mevcut_anlik();
            self.geri_al_yigini.push(mevcut);
            self.anlik_uygula(a);
            self.basarili_mesaj = "↪ Yinelendi.".into();
            self.hata_mesaji.clear();
        }
    }

    // ==================== OTOMATİK KAYIT ====================
    pub(crate) fn autosave_kontrol(&mut self, ctx: &egui::Context) {
        if !self.degisiklik_var {
            return;
        }
        let now = ctx.input(|i| i.time);
        if self.son_autosave == 0.0 {
            self.son_autosave = now; // ilk işaretleme; hemen kaydetme
            return;
        }
        if now - self.son_autosave < 30.0 {
            return;
        }
        self.son_autosave = now;
        let yol = self.autosave_yolu.clone();
        let m = self.proje_olustur();
        let _ = metraj_json_kaydet(&m, &yol);
    }

    // Hiyerarşik is_gruplari yapısını düzleştirip (eski sürümler için) kalemler ile birlikte döndürür.
    fn kayit_yapisi_hazirla(&mut self) -> (Vec<IsGrubu>, Vec<MetrajKalemi>) {
        self.aktif_grubu_senkronize();
        if self.is_gruplari.is_empty() {
            (vec![], self.metraj_kalemleri.clone())
        } else {
            let mut flat = Vec::new();
            for g in &self.is_gruplari {
                flat.extend(g.tum_kalemler_duz());
            }
            (self.is_gruplari.clone(), flat)
        }
    }
    pub(crate) fn kitaplari_yenile(&mut self) { if let Some(ref db) = self.db { if let Ok(k) = db.kitaplari_listele() { self.kitaplar = k; } } }
    fn metraj_kalemlerini_tekillestir(&mut self) -> usize {
        let mut birlesen = 0;
        let mut tekil: Vec<MetrajKalemi> = Vec::with_capacity(self.metraj_kalemleri.len());
        for kalem in self.metraj_kalemleri.drain(..) {
            if let Some(mevcut) = tekil.iter_mut().find(|m| m.poz_no == kalem.poz_no) {
                mevcut.miktar += kalem.miktar;
                mevcut.detaylar.extend(kalem.detaylar);
                mevcut.tutar_guncelle();
                birlesen += 1;
            } else {
                tekil.push(kalem);
            }
        }
        tekil.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
        self.metraj_kalemleri = tekil;
        birlesen
    }
    pub(crate) fn pozlar_tablosu_yenile(&mut self) {
        self.pozlar_tablosu.clear();
        self.analizli_pozlar.clear();
        self.pozlar_donemler.clear();
        self.pozlar_yuklu_kitap_id = self.secili_kitap.as_ref().map(|k| k.id);
        let kitap_id = match self.pozlar_yuklu_kitap_id { Some(id) => id, None => return };
        let arama = self.pozlar_arama_metni.clone();
        if let Some(ref db) = self.db {
            self.pozlar_donemler = db.donemler(kitap_id).unwrap_or_default();
            // Seçili dönem artık geçerli değilse (kurum değişti) en sona dön
            if let Some((y, a)) = self.pozlar_donem {
                if !self.pozlar_donemler.iter().any(|d| d.yil == y && d.ay == a) {
                    self.pozlar_donem = None;
                }
            }
            let sonuc = match self.pozlar_donem {
                Some((y, a)) => db.pozlari_listele_donem(kitap_id, y, a, &arama),
                None => db.pozlari_listele(kitap_id, &arama),
            };
            match sonuc {
                Ok(pozlar) => self.pozlar_tablosu = pozlar,
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
            if let Ok(nolar) = db.analizli_poz_nolari(kitap_id) {
                self.analizli_pozlar = nolar.into_iter().collect();
            }
        }
    }
    pub(crate) fn akilli_ara(&mut self) {
        self.poz_arama_metni.clear();
        self.aciklama_arama_metni.clear();
        self.kategori_pozlar.clear();
        let sorgu = self.akilli_arama_metni.trim();
        if sorgu.is_empty() {
            self.arama_sonuclari.clear();
            return;
        }
        if let Some(ref db) = self.db {
            let kid = self.secili_kitap.as_ref().map(|k| k.id);
            let mut sonuc: Vec<Poz> = Vec::new();
            let poz_gibi = sorgu.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
            if poz_gibi {
                if let Ok(mut pozlar) = db.poz_no_ara(sorgu, kid) {
                    sonuc.append(&mut pozlar);
                }
            }
            if (!poz_gibi || sonuc.len() < 20) && sorgu.split_whitespace().all(|t| !t.is_empty()) {
                if let Ok(pozlar) = db.tam_metin_ara(sorgu, kid) {
                    for poz in pozlar {
                        if !sonuc.iter().any(|p| p.poz_no == poz.poz_no && p.kitap_id == poz.kitap_id) {
                            sonuc.push(poz);
                        }
                    }
                }
            }
            sonuc.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
            sonuc.truncate(100);
            self.arama_sonuclari = sonuc;
        }
    }
    pub(crate) fn poz_no_ara(&mut self) { if self.poz_arama_metni.is_empty() { self.arama_sonuclari.clear(); return; } if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(s) = db.poz_no_ara(&self.poz_arama_metni, kid) { self.arama_sonuclari = s; } } }
    pub(crate) fn aciklama_ara(&mut self) { if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(s) = db.tam_metin_ara(&self.aciklama_arama_metni, kid) { self.arama_sonuclari = s; } } }
    pub(crate) fn poz_sorgula(&mut self) {
        let poz_no = self.yeni_poz_no.trim().to_string();
        if poz_no.is_empty() { self.secili_poz = None; return; }
        if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id);
            match db.poz_getir(&poz_no, kid) {
                Ok(Some(p)) => {
                    self.secili_poz = Some(p);
                    self.yeni_poz_no = poz_no;
                }
                Ok(None) => {
                    if let Ok(s) = db.poz_no_ara(&poz_no, kid) {
                        if s.len() == 1 {
                            self.secili_poz = Some(s[0].clone());
                            self.yeni_poz_no = s[0].poz_no.clone();
                        } else {
                            self.secili_poz = None;
                            self.arama_sonuclari = s;
                        }
                    }
                }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }
    pub(crate) fn kalem_ekle(&mut self) {
        let poz = match self.secili_poz.clone() {
            Some(p) => p,
            None => { self.hata_mesaji = "Once bir poz secin.".into(); return; }
        };
        // Gruplar varsa kalem mutlaka bir aktif gruba eklenir.
        if !self.is_gruplari.is_empty() && self.secili_grup_id.is_none() {
            self.hata_mesaji = "Önce soldaki ağaçtan bir iş grubu seçin.".into();
            self.basarili_mesaj.clear();
            return;
        }
        // metraj_kalemleri aktif grubun düzenleme tamponudur; aynı poz tekrar eklenmez.
        if self.metraj_kalemleri.iter().any(|k| k.poz_no == poz.poz_no) {
            self.basarili_mesaj = format!("{} zaten listede var. Miktarını düzenlemek için satıra tıklayın.", poz.poz_no);
            self.hata_mesaji.clear();
            return;
        }
        self.anlik_goruntu_al();
        let kalem = MetrajKalemi::yeni(&poz, 0.0);
        self.metraj_kalemleri.push(kalem);
        self.metraj_kalemleri.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
        self.aktif_grubu_senkronize();
        self.degisiklik_var = true;
        self.basarili_mesaj = format!("{} eklendi.", poz.poz_no);
        self.hata_mesaji.clear();
    }
    /// Nakliye kalemi ekler: taşıma pozu, miktar = tonaj/hacim × mesafe.
    pub(crate) fn nakliye_kalem_ekle(&mut self, poz: &Poz, miktar: f64, mesafe: f64) {
        if !self.is_gruplari.is_empty() && self.secili_grup_id.is_none() {
            self.hata_mesaji = "Önce soldaki ağaçtan bir iş grubu seçin.".into();
            return;
        }
        self.anlik_goruntu_al();
        let mut kalem = MetrajKalemi::yeni(poz, miktar);
        kalem.imalat_cinsi = format!("Nakliye — {:.0} km", mesafe);
        self.metraj_kalemleri.push(kalem);
        self.metraj_kalemleri.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
        self.aktif_grubu_senkronize();
        self.degisiklik_var = true;
        self.hata_mesaji.clear();
        self.basarili_mesaj = format!("Nakliye eklendi: {} ({:.2} taşıma miktarı).", poz.poz_no, miktar);
    }

    /// Bir kurumu taşınabilir veri paketine (.mvp) aktarır.
    pub(crate) fn kurum_disa_aktar_diyalog(&mut self, kitap_id: i64) {
        let paket = match self.db.as_ref().map(|db| db.kurum_disa_aktar(kitap_id)) {
            Some(Ok(p)) => p,
            Some(Err(e)) => { self.hata_mesaji = format!("{}", e); return; }
            None => return,
        };
        if let Some(d) = rfd::FileDialog::new().add_filter("Metrajmatik Veri Paketi", &["mvp"]).set_file_name(&format!("{}.mvp", paket.kurum)).save_file() {
            match veri_paketi_kaydet(&paket, &d) {
                Ok(()) => self.basarili_mesaj = format!("Veri paketi: {} ({} poz)", d.display(), paket.pozlar.len()),
                Err(e) => self.hata_mesaji = e,
            }
        }
    }

    /// Bir veri paketini (.mvp) içe alır: kurumu + poz + dönem fiyatlarını ekler.
    pub(crate) fn kurum_ice_aktar_diyalog(&mut self) {
        let dosya = match rfd::FileDialog::new().add_filter("Veri Paketi", &["mvp", "json"]).pick_file() { Some(d) => d, None => return };
        let paket = match veri_paketi_yukle(&dosya) { Ok(p) => p, Err(e) => { self.hata_mesaji = e; return; } };
        let sonuc = self.db.as_ref().map(|db| db.kurum_ice_aktar(&paket));
        match sonuc {
            Some(Ok((_, n))) => self.basarili_mesaj = format!("'{}' paketi içe alındı ({} poz).", paket.kurum, n),
            Some(Err(e)) => { self.hata_mesaji = format!("{}", e); return; }
            None => return,
        }
        self.poz_sayisi = self.db.as_ref().and_then(|db| db.poz_sayisi().ok()).unwrap_or(self.poz_sayisi);
        self.kitaplari_yenile();
    }

    /// Tüm fiyat kitabı veritabanını tek dosyaya yedekler. Kullanıcı bu dosyayı
    /// kendi bulut klasörüne (OneDrive/Drive) koyarak "bulut yedek" elde eder.
    pub(crate) fn veritabani_yedekle_diyalog(&mut self) {
        let db = match self.db.as_ref() { Some(d) => d, None => { self.hata_mesaji = "Veritabanı açık değil!".into(); return; } };
        let damga: String = krono_tarih().chars().map(|c| if c.is_ascii_digit() { c } else { '-' }).collect();
        let varsayilan = format!("metrajmatik_yedek_{}.db", damga);
        if let Some(d) = rfd::FileDialog::new().add_filter("Metrajmatik Yedek", &["db"]).set_file_name(&varsayilan).save_file() {
            match db.yedekle(&d) {
                Ok(()) => self.basarili_mesaj = format!("Yedek alındı: {} — bu dosyayı bulut klasörünüze (OneDrive/Drive) kopyalayarak yedekleyebilirsiniz.", d.display()),
                Err(e) => self.hata_mesaji = format!("Yedek alınamadı: {}", e),
            }
        }
    }

    /// Bir yedek dosyasını (.db) geri yükler: canlı bağlantıyı kapatır, dosyayı
    /// mevcut veritabanının üzerine yazar ve yeniden açar. Fiyat kitaplarını değiştirir
    /// (projeler .mrj dosyalarında ayrı tutulur, etkilenmez).
    pub(crate) fn veritabani_geri_yukle_diyalog(&mut self) {
        let kaynak = match rfd::FileDialog::new().add_filter("Metrajmatik Yedek", &["db"]).pick_file() { Some(d) => d, None => return };
        // Bozuk dosyayı üzerine yazmamak için önce geçerliliğini doğrula.
        if let Err(e) = crate::database::Veritabani::ac(&kaynak) {
            self.hata_mesaji = format!("Yedek dosyası geçersiz: {}", e);
            return;
        }
        let hedef = super::veri_yolu("metrajmatik_veriler.db");
        // Canlı bağlantıyı kapat.
        self.db = None;
        // WAL/SHM yan dosyalarını temizle (üzerine yazılan dosyayla tutarsız kalmasın).
        let hedef_metin = hedef.to_string_lossy().to_string();
        for ek in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(PathBuf::from(format!("{}{}", hedef_metin, ek)));
        }
        if let Err(e) = std::fs::copy(&kaynak, &hedef) {
            self.hata_mesaji = format!("Geri yükleme başarısız: {}", e);
            self.db = crate::database::Veritabani::ac(&hedef).ok();
            return;
        }
        match crate::database::Veritabani::ac(&hedef) {
            Ok(vt) => {
                self.poz_sayisi = vt.poz_sayisi().unwrap_or(0);
                self.db = Some(vt);
                self.secili_kitap = None;
                self.kitaplari_yenile();
                self.basarili_mesaj = "Yedek geri yüklendi. Fiyat kitapları güncellendi.".into();
            }
            Err(e) => self.hata_mesaji = format!("Geri yüklendi ama veritabanı açılamadı: {}", e),
        }
    }

    pub(crate) fn kategorileri_yukle(&mut self) { if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(k) = db.kategoriler(kid) { self.kategoriler = k; } } }
    pub(crate) fn kategori_filtrele(&mut self) { if let Some(ref db) = self.db { let kid = self.secili_kitap.as_ref().map(|k| k.id); if let Ok(t) = db.tum_pozlar(kid) { self.kategori_pozlar = t.into_iter().filter(|p| p.kategori == self.secili_kategori).collect(); } } }

    // ==================== DOSYA ====================
    pub(crate) fn pdf_sec_ve_yukle(&mut self) { if let Some(y) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() { self.pdf_yukle(y); } }
    pub(crate) fn pdf_yukle(&mut self, pdf_yolu: PathBuf) {
        let kitap = match self.secili_kitap.clone() { Some(k) => k, None => { self.hata_mesaji = "Once hedef kurum secin!".into(); return; } };
        // Import dönemi: PDF Yükle ekranındaki yıl/ay seçimi.
        let (yil, ay) = (self.yeni_kitap_yil, self.yeni_kitap_ay);
        self.pdf_yukleniyor = true; self.pdf_durumu = format!("PDF okunuyor...");
        match pdf_metin_cikar(&pdf_yolu) {
            Ok(metin) => {
                let profil = match self.import_profili.as_str() {
                    "Çevre ve Şehircilik" => AyristirmaProfili::csb(),
                    "Vakıflar / Restorasyon" => AyristirmaProfili::vakiflar(),
                    "Karayolları (Ar-Ge)" => AyristirmaProfili::kgm(),
                    "Genel" => AyristirmaProfili::genel(),
                    _ => profil_otomatik_sec(&metin),
                };
                let pozlar = pozlari_ayristir(&metin, kitap.id, &kitap.ad, yil, ay, &profil);
                self.pdf_durumu = format!("{} profiliyle {} poz ayrıştırıldı.", profil.ad, pozlar.len());
                if let Some(ref db) = self.db { match db.pozlari_yukle(kitap.id, yil, ay, &pozlar) {
                    Ok(sayi) => { self.poz_sayisi = db.poz_sayisi().unwrap_or(0); self.basarili_mesaj = format!("✅ {} kurumuna {}/{} dönemi için {} poz yuklendi!", kitap.ad, ay, yil, sayi); self.pdf_durumu = format!("✅ {} poz yuklendi.", sayi); if let Ok(Some(yk)) = db.kitap_getir(kitap.id) { self.secili_kitap = Some(yk); } self.kitaplari_yenile(); self.pozlar_tablosu_yenile(); }
                    Err(e) => self.hata_mesaji = format!("{}", e),
                }}
            }
            Err(e) => self.hata_mesaji = format!("{}", e),
        }
        self.pdf_yukleniyor = false;
    }
    // Mevcut durumdan kaydedilebilir proje nesnesi oluşturur (oranlar dahil).
    pub(crate) fn proje_olustur(&mut self) -> KayitliMetraj {
        let (is_gruplari, kalemler) = self.kayit_yapisi_hazirla();
        KayitliMetraj {
            ad: self.metraj_adi.clone(),
            kalemler,
            is_gruplari,
            tarih: krono_tarih(),
            genel_gider_kar_orani: self.genel_gider_kar_orani,
            kdv_orani: self.kdv_orani,
            hesap_turu: self.hesap_turu,
            hakedisler: self.hakedisler.clone(),
            is_programi: self.is_programi.clone(),
            proje_bilgi: self.proje_bilgi.clone(),
        }
    }
    pub(crate) fn metraj_kaydet(&mut self) {
        // Ileri donuk uyumluluk: hem hiyerarsik is_gruplari hem de duzlestirilmis kalemler yazilir
        let m = self.proje_olustur();
        if let Some(ref y) = self.mevcut_dosya_yolu { match metraj_json_kaydet(&m, y) { Ok(()) => { self.degisiklik_var = false; self.basarili_mesaj = format!("Kaydedildi: {}", y.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } }
        else if let Some(d) = rfd::FileDialog::new().add_filter("Metrajmatik Projesi", &["mrj"]).set_file_name(&format!("{}.mrj", self.metraj_adi)).save_file() { match metraj_json_kaydet(&m, &d) { Ok(()) => { self.mevcut_dosya_yolu = Some(d.clone()); self.degisiklik_var = false; self.basarili_mesaj = format!("Kaydedildi: {}", d.display()); } Err(e) => self.hata_mesaji = format!("{}", e) } }
    }
    pub(crate) fn metraj_yukle_diyalog(&mut self) {
        if let Some(d) = rfd::FileDialog::new()
            .add_filter("Metrajmatik Projesi", &["mrj", "json"])
            .pick_file()
        {
            self.metraj_dosyadan_yukle(&d, true);
        }
    }
    // Bir dosyadan projeyi yükler. `dosya_olarak` true ise yol "mevcut dosya" olur (kurtarmada false).
    pub(crate) fn metraj_dosyadan_yukle(&mut self, d: &Path, dosya_olarak: bool) {
        match metraj_json_yukle(d) {
            Ok(m) => {
                let KayitliMetraj { ad, kalemler, is_gruplari, genel_gider_kar_orani, kdv_orani, hesap_turu, hakedisler, is_programi, proje_bilgi, .. } = m;
                self.hesap_turu = hesap_turu;
                self.genel_gider_kar_orani = genel_gider_kar_orani;
                self.kdv_orani = kdv_orani;
                self.hakedisler = hakedisler;
                self.is_programi = is_programi;
                self.proje_bilgi = proje_bilgi;
                self.secili_hakedis = None;
                self.geri_al_yigini.clear();
                self.yinele_yigini.clear();
                self.secili_grup_id = None;
                self.secili_poz = None;
                let mut birlesen = 0;

                if is_gruplari.is_empty() {
                    // Eski flat proje: kalemleri tekilleştir ve otomatik gruba aktar
                    self.is_gruplari = vec![];
                    self.metraj_kalemleri = kalemler;
                    birlesen = self.metraj_kalemlerini_tekillestir();
                    if !self.metraj_kalemleri.is_empty() {
                        self.is_gruplari = vec![
                            IsGrubu {
                                id: "otomatik_insaat".into(),
                                ad: "İnşaat".into(),
                                alt_gruplar: vec![
                                    IsGrubu {
                                        id: "otomatik_kaba_insaat".into(),
                                        ad: "Kaba İnşaat".into(),
                                        alt_gruplar: vec![],
                                        kalemler: std::mem::take(&mut self.metraj_kalemleri),
                                    },
                                ],
                                kalemler: vec![],
                            },
                        ];
                    }
                } else {
                    // Hiyerarşik proje: kalemler grupların içinde, tampon boş başlar
                    self.is_gruplari = is_gruplari;
                    self.metraj_kalemleri = vec![];
                }

                // İlk yaprak grubu aktif yap ve kalemlerini tampona yükle
                if let Some(id) = ilk_yaprak_grup_id(&self.is_gruplari) {
                    self.grup_sec(id);
                }

                self.metraj_adi = ad;
                if dosya_olarak {
                    self.mevcut_dosya_yolu = Some(d.to_path_buf());
                    self.degisiklik_var = birlesen > 0;
                    self.basarili_mesaj = if birlesen > 0 {
                        format!("Açıldı: {} ({} yinelenen poz birleştirildi)", d.display(), birlesen)
                    } else {
                        format!("Açıldı: {}", d.display())
                    };
                } else {
                    // Kurtarma: kaydedilmemiş sayılır
                    self.degisiklik_var = true;
                    self.basarili_mesaj = "Otomatik kayıttan kurtarıldı. Lütfen 'Kaydet' ile kalıcı hale getirin.".into();
                }
            }
            Err(e) => self.hata_mesaji = format!("{}", e),
        }
    }
    pub(crate) fn metraj_excel_diyalog(&mut self) {
        let m = self.proje_olustur();
        // Analiz föyleri: metrajda analizi olan (kitap_id bilinen) pozlar için girdiler.
        let mut analizler: Vec<AnalizFoyu> = Vec::new();
        if let Some(ref db) = self.db {
            let mut gorulen: std::collections::HashSet<(i64, String)> = std::collections::HashSet::new();
            for kalem in &m.kalemler {
                if kalem.kitap_id > 0 && gorulen.insert((kalem.kitap_id, kalem.poz_no.clone())) {
                    if let Ok(girdiler) = db.analiz_getir(kalem.kitap_id, &kalem.poz_no) {
                        if !girdiler.is_empty() {
                            analizler.push(AnalizFoyu {
                                poz_no: kalem.poz_no.clone(),
                                tanim: kalem.tanim.clone(),
                                birim: kalem.birim.clone(),
                                birim_fiyat: kalem.birim_fiyat,
                                girdiler,
                            });
                        }
                    }
                }
            }
        }
        if let Some(d) = rfd::FileDialog::new().add_filter("Excel", &["xlsx"]).set_file_name(&format!("{}.xlsx", self.metraj_adi)).save_file() {
            match metraj_excel_aktar(&m, &analizler, &d) {
                Ok(()) => { self.basarili_mesaj = format!("Excel: {}", d.display()); }
                Err(e) => self.hata_mesaji = format!("{}", e),
            }
        }
    }

    /// Birim fiyat teklif cetveli + teklif mektubu (Excel). `dolu`: proje fiyatlarıyla
    /// dolu (isteklinin çalışma kopyası) ya da boş (isteklilere dağıtılacak cetvel).
    pub(crate) fn teklif_cetveli_diyalog(&mut self, dolu: bool) {
        let m = self.proje_olustur();
        let ek = if dolu { "Teklif Cetveli" } else { "Bos Teklif Cetveli" };
        if let Some(d) = rfd::FileDialog::new().add_filter("Excel", &["xlsx"]).set_file_name(&format!("{} - {}.xlsx", self.metraj_adi, ek)).save_file() {
            match crate::export::teklif_cetveli_excel_aktar(&m, dolu, &d) {
                Ok(()) => self.basarili_mesaj = format!("Teklif cetveli: {}", d.display()),
                Err(e) => self.hata_mesaji = e,
            }
        }
    }

    /// Metrajı CSV'ye aktarır (Excel'de açılabilir).
    pub(crate) fn metraj_csv_diyalog(&mut self) {
        let m = self.proje_olustur();
        if let Some(d) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name(&format!("{}.csv", self.metraj_adi)).save_file() {
            match metraj_csv_aktar(&m, &d) {
                Ok(()) => self.basarili_mesaj = format!("CSV: {}", d.display()),
                Err(e) => self.hata_mesaji = e,
            }
        }
    }

    /// CSV'den (poz_no; miktar; [imalat]) aktif gruba kalem ekler. Pozlar seçili
    /// kurumdan (yoksa tüm kitaplardan) çözülür.
    pub(crate) fn metraj_csv_ice_aktar_diyalog(&mut self) {
        let dosya = match rfd::FileDialog::new().add_filter("CSV", &["csv"]).pick_file() { Some(d) => d, None => return };
        let satirlar = match metraj_csv_oku(&dosya) { Ok(s) => s, Err(e) => { self.hata_mesaji = e; return; } };
        if !self.is_gruplari.is_empty() && self.secili_grup_id.is_none() {
            self.hata_mesaji = "Önce soldaki ağaçtan bir iş grubu seçin.".into();
            return;
        }
        self.anlik_goruntu_al();
        let kid = self.secili_kitap.as_ref().map(|k| k.id);
        let mut eklenen = 0;
        let mut bulunamayan: Vec<String> = Vec::new();
        if let Some(ref db) = self.db {
            for (poz_no, miktar, imalat) in satirlar {
                if self.metraj_kalemleri.iter().any(|k| k.poz_no == poz_no) { continue; }
                match db.poz_getir(&poz_no, kid) {
                    Ok(Some(poz)) => {
                        let mut kalem = MetrajKalemi::yeni(&poz, miktar);
                        kalem.imalat_cinsi = imalat;
                        self.metraj_kalemleri.push(kalem);
                        eklenen += 1;
                    }
                    _ => bulunamayan.push(poz_no),
                }
            }
        }
        self.metraj_kalemleri.sort_by(|a, b| a.poz_no.cmp(&b.poz_no));
        self.aktif_grubu_senkronize();
        self.degisiklik_var = true;
        self.hata_mesaji.clear();
        self.basarili_mesaj = if bulunamayan.is_empty() {
            format!("CSV'den {} kalem eklendi.", eklenen)
        } else {
            let ornek: Vec<String> = bulunamayan.iter().take(5).cloned().collect();
            format!("CSV'den {} kalem eklendi. {} poz bulunamadı: {}", eklenen, bulunamayan.len(), ornek.join(", "))
        };
    }

    /// Rayiç/fiyat güncelleme. İki kip:
    /// - **Kuruma göre:** her kalemi hedef kurumun fiyatıyla (en son ya da seçilen
    ///   döneme göre "o tarihte geçerli" rayiçle) yeniden fiyatlandırır.
    /// - **Endekse göre (Yİ-ÜFE):** tüm birim fiyatları (güncel/temel) oranıyla çarpar.
    pub(crate) fn fiyatlari_guncelle(&mut self) {
        // Aktif grubun tampondaki kalemlerini ağaca yaz ki güncelleme tüm gruplara uygulansın
        self.anlik_goruntu_al();

        // Ağaçtaki (veya düz listedeki) her kaleme bir işlem uygular.
        fn agaci_gez(gruplar: &mut [IsGrubu], f: &mut dyn FnMut(&mut MetrajKalemi)) {
            for g in gruplar.iter_mut() {
                for kalem in g.kalemler.iter_mut() { f(kalem); }
                agaci_gez(&mut g.alt_gruplar, f);
            }
        }

        if self.fiyat_guncelle_endeks_mod {
            // ---- Endekse göre (Yİ-ÜFE) ----
            let (temel, guncel) = (self.fiyat_endeks_temel, self.fiyat_endeks_guncel);
            if temel <= 0.0 { self.hata_mesaji = "Temel endeks sıfırdan büyük olmalı.".into(); return; }
            let carpan = guncel / temel;
            let mut n = 0u32;
            let mut uygula = |kalem: &mut MetrajKalemi| {
                kalem.birim_fiyat = crate::bicim::kurus_yuvarla(kalem.birim_fiyat * carpan);
                kalem.tutar_guncelle();
                n += 1;
            };
            if self.is_gruplari.is_empty() {
                for kalem in self.metraj_kalemleri.iter_mut() { uygula(kalem); }
            } else {
                agaci_gez(&mut self.is_gruplari, &mut uygula);
            }
            self.degisiklik_var = true;
            self.fiyat_guncelle_acik = false;
            self.basarili_mesaj = format!("✅ {} kalem endeksle güncellendi (× {:.4}; Yİ-ÜFE {:.1} → {:.1}).", n, carpan, temel, guncel);
        } else {
            // ---- Kuruma göre (isteğe bağlı dönem) ----
            let hedef = match self.fiyat_guncelle_hedef.clone() {
                Some(k) => k,
                None => { self.hata_mesaji = "Lütfen hedef kurum seçin.".into(); return; }
            };
            let en_son = self.fiyat_guncelle_en_son;
            let (ty, ta) = (self.fiyat_guncelle_yil, self.fiyat_guncelle_ay);
            if let Some(ref db) = self.db {
                let mut guncellenen = 0u32;
                let mut bulunamayan = 0u32;
                let mut kalem_guncelle = |kalem: &mut MetrajKalemi| {
                    let yeni = if en_son {
                        db.poz_getir(&kalem.poz_no, Some(hedef.id)).ok().flatten().and_then(|p| p.fiyat)
                    } else {
                        db.poz_fiyat_asof(hedef.id, &kalem.poz_no, ty, ta).ok().flatten()
                    };
                    if let Some(f) = yeni {
                        kalem.birim_fiyat = f;
                        kalem.kitap_adi = if en_son { hedef.ad.clone() } else { format!("{} ({}/{})", hedef.ad, ta, ty) };
                        kalem.tutar_guncelle();
                        guncellenen += 1;
                    } else {
                        bulunamayan += 1;
                    }
                };
                if self.is_gruplari.is_empty() {
                    for kalem in self.metraj_kalemleri.iter_mut() { kalem_guncelle(kalem); }
                } else {
                    agaci_gez(&mut self.is_gruplari, &mut kalem_guncelle);
                }
                self.degisiklik_var = true;
                self.fiyat_guncelle_acik = false;
                let donem = if en_son { "en son".to_string() } else { format!("{}/{}", ta, ty) };
                self.basarili_mesaj = format!("✅ {} kalem güncellendi (→ {} / {} rayiçleri). {} kalem bulunamadı.", guncellenen, hedef.ad, donem, bulunamayan);
            }
        }

        // Aktif grubun tampondaki kalemlerini güncellenmiş ağaçtan tazele
        if let Some(id) = self.secili_grup_id.clone() {
            if let Some(g) = grup_bul_ref(&self.is_gruplari, &id) {
                self.metraj_kalemleri = g.kalemler.clone();
            }
        }
        self.fiyat_guncelle_hedef = None;
    }
}
