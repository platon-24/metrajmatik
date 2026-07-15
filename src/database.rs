use rusqlite::{params, Connection, OpenFlags, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::bicim::krono_tarih;
use crate::models::{AnalizGirdisi, Donem, Kitap, PaketPoz, Poz, VeriPaketi};

pub struct Veritabani {
    conn: Connection,
}

const UYGULAMA_KIMLIGI: i64 = 0x4D54524A; // "MTRJ"

type EskiPozSatiri = (String, String, String, Option<f64>, String, i64, u32, u32);

// v2 şema: kitap = KURUM (dönem yok), poz = KİMLİK (kurum içinde bir kez),
// fiyat = (yıl/ay) indeksli ayrı tablo. Bir poz'un birden çok aylık fiyatı olur.
const YENI_SEMA: &str = "
    CREATE TABLE IF NOT EXISTS kitaplar (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ad TEXT NOT NULL UNIQUE,
        tarih TEXT NOT NULL DEFAULT ''
    );
    CREATE TABLE IF NOT EXISTS pozlar (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        kitap_id INTEGER NOT NULL,
        poz_no TEXT NOT NULL,
        tanim TEXT NOT NULL,
        birim TEXT NOT NULL,
        kategori TEXT NOT NULL DEFAULT 'DİĞER',
        tur TEXT NOT NULL DEFAULT 'poz',
        UNIQUE(kitap_id, poz_no),
        FOREIGN KEY(kitap_id) REFERENCES kitaplar(id) ON DELETE CASCADE
    );
    CREATE TABLE IF NOT EXISTS poz_fiyatlari (
        poz_id INTEGER NOT NULL,
        yil INTEGER NOT NULL,
        ay INTEGER NOT NULL,
        fiyat REAL,
        PRIMARY KEY(poz_id, yil, ay),
        FOREIGN KEY(poz_id) REFERENCES pozlar(id) ON DELETE CASCADE
    );
    CREATE TABLE IF NOT EXISTS varsayilan_is_gruplari (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ad TEXT NOT NULL,
        ust_grup_id INTEGER,
        sira INTEGER NOT NULL,
        FOREIGN KEY(ust_grup_id) REFERENCES varsayilan_is_gruplari(id) ON DELETE CASCADE
    );
    CREATE TABLE IF NOT EXISTS analiz_girdileri (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        kitap_id INTEGER NOT NULL,
        poz_no TEXT NOT NULL,
        girdi_no TEXT NOT NULL,
        tanim TEXT NOT NULL,
        birim TEXT NOT NULL,
        birim_fiyat REAL NOT NULL,
        miktar REAL NOT NULL,
        tur TEXT NOT NULL DEFAULT 'Malzeme',
        sira INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX IF NOT EXISTS idx_analiz_poz ON analiz_girdileri(kitap_id, poz_no);
    CREATE INDEX IF NOT EXISTS idx_fiyat_poz ON poz_fiyatlari(poz_id);
    CREATE VIRTUAL TABLE IF NOT EXISTS pozlar_fts USING fts5(
        poz_no, tanim, birim, kategori, content='pozlar', content_rowid='id'
    );
    CREATE TRIGGER IF NOT EXISTS pozlar_ai AFTER INSERT ON pozlar BEGIN
        INSERT INTO pozlar_fts(rowid, poz_no, tanim, birim, kategori)
        VALUES (new.id, new.poz_no, new.tanim, new.birim, new.kategori);
    END;
    CREATE TRIGGER IF NOT EXISTS pozlar_ad AFTER DELETE ON pozlar BEGIN
        INSERT INTO pozlar_fts(pozlar_fts, rowid, poz_no, tanim, birim, kategori)
        VALUES('delete', old.id, old.poz_no, old.tanim, old.birim, old.kategori);
    END;
    CREATE TRIGGER IF NOT EXISTS pozlar_au AFTER UPDATE ON pozlar BEGIN
        INSERT INTO pozlar_fts(pozlar_fts, rowid, poz_no, tanim, birim, kategori)
        VALUES('delete', old.id, old.poz_no, old.tanim, old.birim, old.kategori);
        INSERT INTO pozlar_fts(rowid, poz_no, tanim, birim, kategori)
        VALUES (new.id, new.poz_no, new.tanim, new.birim, new.kategori);
    END;
";

// Bir poz'u EN SON dönem fiyatıyla getiren temel SELECT (arama sonuçları için).
const POZ_SECIM_BASE: &str = "SELECT p.poz_no, p.tanim, p.birim, f.fiyat, p.kategori, p.kitap_id, k.ad, f.yil, f.ay \
     FROM pozlar p \
     JOIN kitaplar k ON k.id = p.kitap_id \
     JOIN poz_fiyatlari f ON f.poz_id = p.id \
       AND f.yil * 100 + f.ay = (SELECT MAX(f2.yil * 100 + f2.ay) FROM poz_fiyatlari f2 WHERE f2.poz_id = p.id)";

fn poz_secim_sql(kitap_id: Option<i64>) -> String {
    match kitap_id {
        Some(kid) => format!("{} WHERE p.kitap_id = {}", POZ_SECIM_BASE, kid),
        None => format!("{} WHERE 1 = 1", POZ_SECIM_BASE),
    }
}

impl Veritabani {
    pub fn ac(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Veritabani { conn };
        db.tablolari_olustur()?;
        Ok(db)
    }

    fn tablolari_olustur(&self) -> Result<()> {
        // v1 (kurum+dönem tek tabloda, pozlar.yil kolonu) tespit edilirse v2'ye göç.
        // FK zorlaması bu noktada henüz KAPALI (varsayılan) — göç güvenli.
        let v1 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('pozlar') WHERE name = 'yil'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if v1 {
            self.eski_semadan_goc()?;
        }

        self.conn.execute_batch(YENI_SEMA)?;

        // Tohumlama (yalnızca boşsa)
        let count = self
            .conn
            .query_row("SELECT COUNT(*) FROM varsayilan_is_gruplari", [], |row| {
                row.get::<_, u32>(0)
            })
            .unwrap_or(0);
        if count == 0 {
            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('İnşaat', NULL, 1)", [])?;
            let insaat_id = self.conn.last_insert_rowid();
            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('Kaba İnşaat', ?1, 1)", params![insaat_id])?;
            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('İnce İnşaat', ?1, 2)", params![insaat_id])?;

            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('Mekanik Tesisat', NULL, 2)", [])?;
            let mekanik_id = self.conn.last_insert_rowid();
            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('Sıhhi Tesisat', ?1, 1)", params![mekanik_id])?;
            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('Isıtma Tesisatı', ?1, 2)", params![mekanik_id])?;

            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('Elektrik Tesisatı', NULL, 3)", [])?;
            let elektrik_id = self.conn.last_insert_rowid();
            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('Kuvvetli Akım', ?1, 1)", params![elektrik_id])?;
            self.conn.execute("INSERT INTO varsayilan_is_gruplari (ad, ust_grup_id, sira) VALUES ('Zayıf Akım', ?1, 2)", params![elektrik_id])?;
        }

        self.conn.execute_batch(
            "PRAGMA application_id = 0x4D54524A;
             PRAGMA foreign_keys = ON;
             PRAGMA journal_mode=WAL;",
        )?;
        Ok(())
    }

    /// Bir geri yükleme adayını hiçbir şekilde değiştirmeden doğrular. Metrajmatik
    /// uygulama kimliği varsa eşleşmesini, SQLite bütünlüğünü ve v1/v2 şema
    /// imzasını denetler. `ac()` burada kullanılamaz; eksik tabloları oluşturur.
    pub fn yedek_dogrula(yol: &Path) -> Result<()> {
        let conn = Connection::open_with_flags(yol, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let sonuc: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if sonuc != "ok" {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let uygulama_id: i64 = conn.query_row("PRAGMA application_id", [], |r| r.get(0))?;
        if uygulama_id != 0 && uygulama_id != UYGULAMA_KIMLIGI {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let temel_tablo: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('kitaplar','pozlar')",
            [],
            |r| r.get(0),
        )?;
        let v2_fiyat: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='poz_fiyatlari'",
            [],
            |r| r.get(0),
        )?;
        let v1_yil: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('pozlar') WHERE name='yil'",
            [],
            |r| r.get(0),
        )?;
        if temel_tablo != 2 || (v2_fiyat == 0 && v1_yil == 0) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(())
    }

    /// v1 (kurum+dönem = ayrı kitap) verisini v2'ye (kurum + dönem-indeksli fiyat) taşır.
    /// Kitaplar ada göre gruplanır; poz kimlikleri tekilleştirilir; her (yıl/ay) fiyatı
    /// poz_fiyatlari'na aktarılır; özel pozlar ve analizler korunur (kitap_id yeniden eşlenir).
    fn eski_semadan_goc(&self) -> Result<()> {
        log::info!("v1→v2 şema göçü (kurum/dönem modeli) başlıyor...");
        let tx = self.conn.unchecked_transaction()?;

        // 1. Eski FTS/trigger'ları temizle, eski tabloları yeniden adlandır.
        //    analiz_girdileri'ni FK'siz yeniden kur (kitaplar rename'i FK'yi bozmasın).
        tx.execute_batch(
            "DROP TABLE IF EXISTS pozlar_fts;
             DROP TRIGGER IF EXISTS pozlar_ai;
             DROP TRIGGER IF EXISTS pozlar_ad;
             DROP TRIGGER IF EXISTS pozlar_au;
             ALTER TABLE kitaplar RENAME TO kitaplar_eski_goc;
             ALTER TABLE pozlar RENAME TO pozlar_eski_goc;
             CREATE TABLE analiz_goc (
                 id INTEGER PRIMARY KEY AUTOINCREMENT, kitap_id INTEGER NOT NULL, poz_no TEXT NOT NULL,
                 girdi_no TEXT NOT NULL, tanim TEXT NOT NULL, birim TEXT NOT NULL, birim_fiyat REAL NOT NULL,
                 miktar REAL NOT NULL, tur TEXT NOT NULL DEFAULT 'Malzeme', sira INTEGER NOT NULL DEFAULT 0);
             INSERT INTO analiz_goc SELECT id, kitap_id, poz_no, girdi_no, tanim, birim, birim_fiyat, miktar, tur, sira FROM analiz_girdileri;
             DROP TABLE analiz_girdileri;
             ALTER TABLE analiz_goc RENAME TO analiz_girdileri;",
        )?;

        // 2. Yeni şemayı kur.
        tx.execute_batch(YENI_SEMA)?;

        // 3. Kurumları (distinct ad) oluştur + eski→yeni kitap eşlemesi.
        tx.execute("INSERT OR IGNORE INTO kitaplar (ad, tarih) SELECT DISTINCT ad, '' FROM kitaplar_eski_goc", [])?;
        tx.execute(
            "CREATE TEMP TABLE kitap_map AS
             SELECT ek.id AS eski, k.id AS yeni FROM kitaplar_eski_goc ek JOIN kitaplar k ON k.ad = ek.ad", [],
        )?;
        let mut map: HashMap<i64, i64> = HashMap::new();
        {
            let mut s = tx.prepare("SELECT eski, yeni FROM kitap_map")?;
            let rows = s.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
            for row in rows {
                let (o, n) = row?;
                map.insert(o, n);
            }
        }

        // 4. Eski pozları oku; kimlikleri tekilleştir; fiyatları dönem tablosuna yaz.
        let mut eski_pozlar: Vec<EskiPozSatiri> = Vec::new();
        {
            let mut s = tx.prepare("SELECT poz_no, tanim, birim, fiyat, kategori, kitap_id, yil, ay FROM pozlar_eski_goc")?;
            let rows = s.query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            })?;
            for row in rows {
                eski_pozlar.push(row?);
            }
        }
        for (poz_no, tanim, birim, fiyat, kategori, eski_kid, yil, ay) in eski_pozlar {
            let kurum = match map.get(&eski_kid) {
                Some(k) => *k,
                None => continue,
            };
            tx.execute(
                "INSERT OR IGNORE INTO pozlar (kitap_id, poz_no, tanim, birim, kategori, tur) VALUES (?1, ?2, ?3, ?4, ?5, 'poz')",
                params![kurum, poz_no, tanim, birim, kategori],
            )?;
            let poz_id: i64 = tx.query_row(
                "SELECT id FROM pozlar WHERE kitap_id = ?1 AND poz_no = ?2",
                params![kurum, poz_no],
                |r| r.get(0),
            )?;
            tx.execute("INSERT OR REPLACE INTO poz_fiyatlari (poz_id, yil, ay, fiyat) VALUES (?1, ?2, ?3, ?4)", params![poz_id, yil, ay, fiyat])?;
        }

        // 5. Analizlerin kitap_id'sini yeni kurum id'lerine eşle (çakışmasız, tek UPDATE).
        tx.execute(
            "UPDATE analiz_girdileri SET kitap_id = (SELECT yeni FROM kitap_map WHERE eski = analiz_girdileri.kitap_id)
             WHERE kitap_id IN (SELECT eski FROM kitap_map)", [],
        )?;

        // 6. Eski tabloları düşür.
        tx.execute_batch(
            "DROP TABLE kitaplar_eski_goc; DROP TABLE pozlar_eski_goc; DROP TABLE kitap_map;",
        )?;
        tx.commit()?;
        log::info!("Göç tamamlandı.");
        Ok(())
    }

    // ==================== KİTAP (KURUM) ====================
    /// Kurum ekler (varsa mevcut id'yi döndürür).
    pub fn kitap_ekle(&self, ad: &str) -> Result<i64> {
        let tarih = krono_tarih();
        self.conn.execute(
            "INSERT OR IGNORE INTO kitaplar (ad, tarih) VALUES (?1, ?2)",
            params![ad, tarih],
        )?;
        self.conn
            .query_row("SELECT id FROM kitaplar WHERE ad = ?1", params![ad], |r| {
                r.get(0)
            })
    }

    /// Her kurum için: en son dönem (görüntü) ve toplam (tekil) poz sayısı.
    pub fn kitaplari_listele(&self) -> Result<Vec<Kitap>> {
        let mut stmt = self.conn.prepare(
            "SELECT k.id, k.ad,
                COALESCE((SELECT f.yil FROM poz_fiyatlari f JOIN pozlar p ON p.id = f.poz_id
                          WHERE p.kitap_id = k.id ORDER BY f.yil DESC, f.ay DESC LIMIT 1), 0),
                COALESCE((SELECT f.ay FROM poz_fiyatlari f JOIN pozlar p ON p.id = f.poz_id
                          WHERE p.kitap_id = k.id ORDER BY f.yil DESC, f.ay DESC LIMIT 1), 0),
                (SELECT COUNT(*) FROM pozlar p WHERE p.kitap_id = k.id),
                k.tarih
             FROM kitaplar k ORDER BY k.ad",
        )?;
        let sonuc = stmt
            .query_map([], |row| {
                Ok(Kitap {
                    id: row.get(0)?,
                    ad: row.get(1)?,
                    yil: row.get(2)?,
                    ay: row.get(3)?,
                    poz_sayisi: row.get(4)?,
                    tarih: row.get(5)?,
                })
            })?
            .filter_map(|k| k.ok())
            .collect();
        Ok(sonuc)
    }

    pub fn kitap_getir(&self, kitap_id: i64) -> Result<Option<Kitap>> {
        let mut stmt = self.conn.prepare(
            "SELECT k.id, k.ad,
                COALESCE((SELECT f.yil FROM poz_fiyatlari f JOIN pozlar p ON p.id = f.poz_id
                          WHERE p.kitap_id = k.id ORDER BY f.yil DESC, f.ay DESC LIMIT 1), 0),
                COALESCE((SELECT f.ay FROM poz_fiyatlari f JOIN pozlar p ON p.id = f.poz_id
                          WHERE p.kitap_id = k.id ORDER BY f.yil DESC, f.ay DESC LIMIT 1), 0),
                (SELECT COUNT(*) FROM pozlar p WHERE p.kitap_id = k.id),
                k.tarih
             FROM kitaplar k WHERE k.id = ?1",
        )?;
        let mut sonuc = stmt
            .query_map(params![kitap_id], |row| {
                Ok(Kitap {
                    id: row.get(0)?,
                    ad: row.get(1)?,
                    yil: row.get(2)?,
                    ay: row.get(3)?,
                    poz_sayisi: row.get(4)?,
                    tarih: row.get(5)?,
                })
            })?
            .filter_map(|k| k.ok());
        Ok(sonuc.next())
    }

    /// Bir kurumun sahip olduğu dönemler (yıl/ay) ve her dönemdeki poz sayısı.
    pub fn donemler(&self, kitap_id: i64) -> Result<Vec<Donem>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.yil, f.ay, COUNT(*) FROM poz_fiyatlari f JOIN pozlar p ON p.id = f.poz_id
             WHERE p.kitap_id = ?1 GROUP BY f.yil, f.ay ORDER BY f.yil DESC, f.ay DESC",
        )?;
        let rows = stmt.query_map(params![kitap_id], |row| {
            Ok(Donem {
                yil: row.get(0)?,
                ay: row.get(1)?,
                poz_sayisi: row.get(2)?,
            })
        })?;
        Ok(rows.filter_map(|d| d.ok()).collect())
    }

    pub fn kitap_sil(&self, kitap_id: i64) -> Result<()> {
        // FK cascade pozlar+fiyatları siler; analiz'i (FK yok) elle sil.
        self.conn.execute(
            "DELETE FROM analiz_girdileri WHERE kitap_id = ?1",
            params![kitap_id],
        )?;
        self.conn
            .execute("DELETE FROM kitaplar WHERE id = ?1", params![kitap_id])?;
        Ok(())
    }

    pub fn kitap_guncelle(&self, kitap_id: i64, ad: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE kitaplar SET ad = ?1 WHERE id = ?2",
            params![ad, kitap_id],
        )?;
        Ok(())
    }

    /// Belirli bir dönemin fiyatlarını temizler (o dönemi tamamen kaldırmak için).
    pub fn donem_sil(&self, kitap_id: i64, yil: u32, ay: u32) -> Result<()> {
        self.conn.execute(
            "DELETE FROM poz_fiyatlari WHERE yil = ?1 AND ay = ?2 AND poz_id IN (SELECT id FROM pozlar WHERE kitap_id = ?3)",
            params![yil, ay, kitap_id],
        )?;
        // Artık hiç fiyatı kalmayan poz kimliklerini de temizle
        self.conn.execute(
            "DELETE FROM pozlar WHERE kitap_id = ?1 AND id NOT IN (SELECT DISTINCT poz_id FROM poz_fiyatlari)",
            params![kitap_id],
        )?;
        Ok(())
    }

    // ==================== POZ ====================
    /// Bir kuruma, verilen dönem için PDF'ten çıkan pozları yükler. Poz kimliği
    /// (kurum + poz_no) yoksa oluşturulur; o dönemin fiyatı yazılır (yeniden import
    /// aynı dönemi değiştirir).
    pub fn pozlari_yukle(&self, kitap_id: i64, yil: u32, ay: u32, pozlar: &[Poz]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM poz_fiyatlari WHERE yil = ?1 AND ay = ?2 AND poz_id IN (SELECT id FROM pozlar WHERE kitap_id = ?3)",
            params![yil, ay, kitap_id],
        )?;
        let mut eklenen = 0usize;
        for poz in pozlar {
            tx.execute(
                "INSERT OR IGNORE INTO pozlar (kitap_id, poz_no, tanim, birim, kategori, tur) VALUES (?1, ?2, ?3, ?4, ?5, 'poz')",
                params![kitap_id, poz.poz_no, poz.tanim, poz.birim, poz.kategori],
            )?;
            let poz_id: i64 = tx.query_row(
                "SELECT id FROM pozlar WHERE kitap_id = ?1 AND poz_no = ?2",
                params![kitap_id, poz.poz_no],
                |r| r.get(0),
            )?;
            tx.execute("INSERT OR REPLACE INTO poz_fiyatlari (poz_id, yil, ay, fiyat) VALUES (?1, ?2, ?3, ?4)", params![poz_id, yil, ay, poz.fiyat])?;
            eklenen += 1;
        }
        tx.commit()?;
        Ok(eklenen)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn poz_ekle(
        &self,
        kitap_id: i64,
        yil: u32,
        ay: u32,
        poz_no: &str,
        tanim: &str,
        birim: &str,
        fiyat: Option<f64>,
        kategori: &str,
    ) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO pozlar (kitap_id, poz_no, tanim, birim, kategori, tur) VALUES (?1, ?2, ?3, ?4, ?5, 'poz')",
            params![kitap_id, poz_no, tanim, birim, kategori],
        )?;
        tx.execute(
            "UPDATE pozlar SET tanim = ?1, birim = ?2, kategori = ?3 WHERE kitap_id = ?4 AND poz_no = ?5",
            params![tanim, birim, kategori, kitap_id, poz_no],
        )?;
        let poz_id: i64 = tx.query_row(
            "SELECT id FROM pozlar WHERE kitap_id = ?1 AND poz_no = ?2",
            params![kitap_id, poz_no],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO poz_fiyatlari (poz_id, yil, ay, fiyat) VALUES (?1, ?2, ?3, ?4)",
            params![poz_id, yil, ay, fiyat],
        )?;
        tx.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn poz_guncelle(
        &self,
        kitap_id: i64,
        yil: u32,
        ay: u32,
        eski_poz_no: &str,
        poz_no: &str,
        tanim: &str,
        birim: &str,
        fiyat: Option<f64>,
        kategori: &str,
    ) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE pozlar SET poz_no = ?1, tanim = ?2, birim = ?3, kategori = ?4 WHERE kitap_id = ?5 AND poz_no = ?6",
            params![poz_no, tanim, birim, kategori, kitap_id, eski_poz_no],
        )?;
        let poz_id: i64 = tx.query_row(
            "SELECT id FROM pozlar WHERE kitap_id = ?1 AND poz_no = ?2",
            params![kitap_id, poz_no],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO poz_fiyatlari (poz_id, yil, ay, fiyat) VALUES (?1, ?2, ?3, ?4)",
            params![poz_id, yil, ay, fiyat],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn poz_sil(&self, kitap_id: i64, poz_no: &str) -> Result<()> {
        // FK cascade fiyatları siler; analiz'i (FK yok) elle sil.
        self.conn.execute(
            "DELETE FROM analiz_girdileri WHERE kitap_id = ?1 AND poz_no = ?2",
            params![kitap_id, poz_no],
        )?;
        self.conn.execute(
            "DELETE FROM pozlar WHERE kitap_id = ?1 AND poz_no = ?2",
            params![kitap_id, poz_no],
        )?;
        Ok(())
    }

    /// Analiz sonucunu pozun BELİRLİ dönem fiyatı yapar (yoksa o dönemi ekler).
    pub fn poz_fiyat_guncelle(
        &self,
        kitap_id: i64,
        poz_no: &str,
        yil: u32,
        ay: u32,
        fiyat: f64,
    ) -> Result<()> {
        let poz_id: i64 = self.conn.query_row(
            "SELECT id FROM pozlar WHERE kitap_id = ?1 AND poz_no = ?2",
            params![kitap_id, poz_no],
            |r| r.get(0),
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO poz_fiyatlari (poz_id, yil, ay, fiyat) VALUES (?1, ?2, ?3, ?4)",
            params![poz_id, yil, ay, fiyat],
        )?;
        Ok(())
    }

    // ==================== ANALİZ ====================
    pub fn analiz_getir(&self, kitap_id: i64, poz_no: &str) -> Result<Vec<AnalizGirdisi>> {
        let mut stmt = self.conn.prepare(
            "SELECT girdi_no, tanim, birim, birim_fiyat, miktar, tur
             FROM analiz_girdileri WHERE kitap_id = ?1 AND poz_no = ?2 ORDER BY sira, id",
        )?;
        let rows = stmt.query_map(params![kitap_id, poz_no], |row| {
            Ok(AnalizGirdisi {
                girdi_no: row.get(0)?,
                tanim: row.get(1)?,
                birim: row.get(2)?,
                birim_fiyat: row.get(3)?,
                miktar: row.get(4)?,
                tur: row.get(5)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Bir pozun analiz girdilerini (varsa eskisini silip) atomik olarak kaydeder.
    pub fn analiz_kaydet(
        &self,
        kitap_id: i64,
        poz_no: &str,
        girdiler: &[AnalizGirdisi],
    ) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM analiz_girdileri WHERE kitap_id = ?1 AND poz_no = ?2",
            params![kitap_id, poz_no],
        )?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO analiz_girdileri (kitap_id, poz_no, girdi_no, tanim, birim, birim_fiyat, miktar, tur, sira)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for (i, g) in girdiler.iter().enumerate() {
                stmt.execute(params![
                    kitap_id,
                    poz_no,
                    g.girdi_no,
                    g.tanim,
                    g.birim,
                    g.birim_fiyat,
                    g.miktar,
                    g.tur,
                    i as i64
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    // ==================== VERİ PAKETİ ====================
    /// Bir kurumun tüm pozlarını + dönem fiyatlarını taşınabilir pakete çıkarır.
    pub fn kurum_disa_aktar(&self, kitap_id: i64) -> Result<VeriPaketi> {
        let kurum: String = self.conn.query_row(
            "SELECT ad FROM kitaplar WHERE id = ?1",
            params![kitap_id],
            |r| r.get(0),
        )?;
        let mut stmt = self.conn.prepare("SELECT id, poz_no, tanim, birim, kategori FROM pozlar WHERE kitap_id = ?1 ORDER BY poz_no")?;
        let ham: Vec<(i64, String, String, String, String)> = stmt
            .query_map(params![kitap_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .filter_map(|x| x.ok())
            .collect();
        let mut pozlar = Vec::with_capacity(ham.len());
        for (pid, poz_no, tanim, birim, kategori) in ham {
            let mut fs = self.conn.prepare(
                "SELECT yil, ay, fiyat FROM poz_fiyatlari WHERE poz_id = ?1 ORDER BY yil, ay",
            )?;
            let fiyatlar: Vec<(u32, u32, Option<f64>)> = fs
                .query_map(params![pid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .filter_map(|x| x.ok())
                .collect();
            pozlar.push(PaketPoz {
                poz_no,
                tanim,
                birim,
                kategori,
                fiyatlar,
            });
        }
        Ok(VeriPaketi { kurum, pozlar })
    }

    /// Veri paketini içe alır: kurumu (yoksa) oluşturur, poz + dönem fiyatlarını ekler.
    pub fn kurum_ice_aktar(&self, paket: &VeriPaketi) -> Result<(i64, usize)> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO kitaplar (ad, tarih) VALUES (?1, ?2)",
            params![paket.kurum, krono_tarih()],
        )?;
        let kitap_id: i64 = tx.query_row(
            "SELECT id FROM kitaplar WHERE ad = ?1",
            params![paket.kurum],
            |r| r.get(0),
        )?;
        for p in &paket.pozlar {
            tx.execute(
                "INSERT OR IGNORE INTO pozlar (kitap_id, poz_no, tanim, birim, kategori, tur) VALUES (?1, ?2, ?3, ?4, ?5, 'poz')",
                params![kitap_id, p.poz_no, p.tanim, p.birim, p.kategori],
            )?;
            let poz_id: i64 = tx.query_row(
                "SELECT id FROM pozlar WHERE kitap_id = ?1 AND poz_no = ?2",
                params![kitap_id, p.poz_no],
                |r| r.get(0),
            )?;
            for (yil, ay, fiyat) in &p.fiyatlar {
                tx.execute("INSERT OR REPLACE INTO poz_fiyatlari (poz_id, yil, ay, fiyat) VALUES (?1, ?2, ?3, ?4)", params![poz_id, yil, ay, fiyat])?;
            }
        }
        tx.commit()?;
        Ok((kitap_id, paket.pozlar.len()))
    }

    /// Bir kurumda analizi olan poz numaraları (rozet için).
    pub fn analizli_poz_nolari(&self, kitap_id: i64) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT poz_no FROM analiz_girdileri WHERE kitap_id = ?1")?;
        let rows = stmt.query_map(params![kitap_id], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    // ==================== ARAMA (hep EN SON dönem fiyatı) ====================
    fn poz_map(row: &rusqlite::Row) -> rusqlite::Result<Poz> {
        Ok(Poz {
            poz_no: row.get(0)?,
            tanim: row.get(1)?,
            birim: row.get(2)?,
            fiyat: row.get(3)?,
            kategori: row.get(4)?,
            kitap_id: row.get(5)?,
            kitap_adi: row.get(6)?,
            yil: row.get(7)?,
            ay: row.get(8)?,
        })
    }

    pub fn poz_no_ara(&self, poz_no: &str, kitap_id: Option<i64>) -> Result<Vec<Poz>> {
        let sql = format!(
            "{} AND p.poz_no LIKE ?1 ORDER BY p.poz_no LIMIT 50",
            poz_secim_sql(kitap_id)
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![format!("{}%", poz_no)], Self::poz_map)?;
        Ok(rows.filter_map(|p| p.ok()).collect())
    }

    pub fn tam_metin_ara(&self, sorgu: &str, kitap_id: Option<i64>) -> Result<Vec<Poz>> {
        let terimler: Vec<String> = sorgu
            .split_whitespace()
            .map(|t| format!("\"{}\"*", t.replace('"', "")))
            .collect();
        let kitap_filtre = if let Some(kid) = kitap_id {
            format!(" AND p.kitap_id = {}", kid)
        } else {
            String::new()
        };
        let sql = format!(
            "SELECT p.poz_no, p.tanim, p.birim, f.fiyat, p.kategori, p.kitap_id, k.ad, f.yil, f.ay
             FROM pozlar_fts ft
             JOIN pozlar p ON ft.rowid = p.id
             JOIN kitaplar k ON k.id = p.kitap_id
             JOIN poz_fiyatlari f ON f.poz_id = p.id AND f.yil * 100 + f.ay = (SELECT MAX(f2.yil * 100 + f2.ay) FROM poz_fiyatlari f2 WHERE f2.poz_id = p.id)
             WHERE pozlar_fts MATCH ?1 {} ORDER BY rank LIMIT 100", kitap_filtre);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![terimler.join(" AND ")], Self::poz_map)?;
        Ok(rows.filter_map(|p| p.ok()).collect())
    }

    /// Tüm veritabanını (kurumlar + pozlar + fiyatlar + analizler) tek temiz dosyaya
    /// yedekler. `VACUUM INTO` WAL'ı birleştirip bütünlüklü tek dosya üretir; hedef
    /// dosya varsa önce silinir (VACUUM INTO mevcut dosyaya yazmaz).
    pub fn yedekle(&self, hedef: &Path) -> Result<()> {
        if hedef.exists() {
            std::fs::remove_file(hedef).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                    e.to_string(),
                )))
            })?;
        }
        let yol = hedef.to_string_lossy().replace('\'', "''");
        self.conn.execute_batch(&format!("VACUUM INTO '{}'", yol))
    }

    pub fn poz_getir(&self, poz_no: &str, kitap_id: Option<i64>) -> Result<Option<Poz>> {
        let sql = format!("{} AND p.poz_no = ?1 LIMIT 1", poz_secim_sql(kitap_id));
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![poz_no], Self::poz_map)?;
        rows.next().transpose()
    }

    /// Bir pozun (yıl, ay) tarihinde GEÇERLİ fiyatı: o tarih ve öncesinde yayımlanmış
    /// en son fiyat. Rayiçleri ihale tarihine güncellerken kullanılır (kurum kitabı
    /// her ay yeniden yayımlanmayabilir; "o tarihteki geçerli rayiç" mantığı). Yoksa None.
    pub fn poz_fiyat_asof(
        &self,
        kitap_id: i64,
        poz_no: &str,
        yil: u32,
        ay: u32,
    ) -> Result<Option<f64>> {
        let hedef = (yil * 100 + ay) as i64;
        let sql = "SELECT f.fiyat FROM pozlar p
                   JOIN poz_fiyatlari f ON f.poz_id = p.id
                   WHERE p.kitap_id = ?1 AND p.poz_no = ?2 AND (f.yil * 100 + f.ay) <= ?3 AND f.fiyat IS NOT NULL
                   ORDER BY (f.yil * 100 + f.ay) DESC LIMIT 1";
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query_map(params![kitap_id, poz_no, hedef], |r| r.get::<_, f64>(0))?;
        rows.next().transpose()
    }

    pub fn tum_pozlar(&self, kitap_id: Option<i64>) -> Result<Vec<Poz>> {
        let sql = format!("{} ORDER BY p.poz_no", poz_secim_sql(kitap_id));
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::poz_map)?;
        Ok(rows.filter_map(|p| p.ok()).collect())
    }

    pub fn pozlari_listele(&self, kitap_id: i64, arama: &str) -> Result<Vec<Poz>> {
        let arama = arama.trim();
        if arama.is_empty() {
            let sql = format!("{} ORDER BY p.poz_no", poz_secim_sql(Some(kitap_id)));
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map([], Self::poz_map)?;
            return Ok(rows.filter_map(|p| p.ok()).collect());
        }
        let sql = format!(
            "{} AND (p.poz_no LIKE ?1 OR p.tanim LIKE ?1 OR p.birim LIKE ?1 OR p.kategori LIKE ?1) ORDER BY p.poz_no",
            poz_secim_sql(Some(kitap_id)),
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![format!("%{}%", arama)], Self::poz_map)?;
        Ok(rows.filter_map(|p| p.ok()).collect())
    }

    /// Belirli bir DÖNEMİN pozlarını (o dönemki fiyatlarıyla) listeler — eski fiyat
    /// görüntüleme için. O dönemde fiyatı olmayan pozlar listede olmaz.
    pub fn pozlari_listele_donem(
        &self,
        kitap_id: i64,
        yil: u32,
        ay: u32,
        arama: &str,
    ) -> Result<Vec<Poz>> {
        let arama = arama.trim();
        let base = format!(
            "SELECT p.poz_no, p.tanim, p.birim, f.fiyat, p.kategori, p.kitap_id, k.ad, f.yil, f.ay \
             FROM pozlar p JOIN kitaplar k ON k.id = p.kitap_id \
             JOIN poz_fiyatlari f ON f.poz_id = p.id AND f.yil = {} AND f.ay = {} \
             WHERE p.kitap_id = {}",
            yil, ay, kitap_id,
        );
        if arama.is_empty() {
            let sql = format!("{} ORDER BY p.poz_no", base);
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map([], Self::poz_map)?;
            return Ok(rows.filter_map(|p| p.ok()).collect());
        }
        let sql = format!("{} AND (p.poz_no LIKE ?1 OR p.tanim LIKE ?1 OR p.birim LIKE ?1 OR p.kategori LIKE ?1) ORDER BY p.poz_no", base);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![format!("%{}%", arama)], Self::poz_map)?;
        Ok(rows.filter_map(|p| p.ok()).collect())
    }

    pub fn kategoriler(&self, kitap_id: Option<i64>) -> Result<Vec<String>> {
        let sql = if let Some(kid) = kitap_id {
            format!(
                "SELECT DISTINCT kategori FROM pozlar WHERE kitap_id = {} ORDER BY kategori",
                kid
            )
        } else {
            "SELECT DISTINCT kategori FROM pozlar ORDER BY kategori".to_string()
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.filter_map(|k| k.ok()).collect())
    }

    /// Tekil poz kimliği sayısı.
    pub fn poz_sayisi(&self) -> Result<u32> {
        self.conn
            .query_row("SELECT COUNT(*) FROM pozlar", [], |row| row.get(0))
    }

    pub fn varsayilan_gruplari_getir(&self) -> Result<Vec<crate::models::IsGrubu>> {
        struct DbGrup {
            id: i64,
            ad: String,
            ust_grup_id: Option<i64>,
        }
        let mut stmt = self.conn.prepare("SELECT id, ad, ust_grup_id FROM varsayilan_is_gruplari ORDER BY ust_grup_id ASC, sira ASC")?;
        let db_gruplar: Vec<DbGrup> = stmt
            .query_map([], |row| {
                Ok(DbGrup {
                    id: row.get(0)?,
                    ad: row.get(1)?,
                    ust_grup_id: row.get(2)?,
                })
            })?
            .filter_map(|x| x.ok())
            .collect();

        fn build_tree(
            db_gruplar: &[DbGrup],
            parent_id: Option<i64>,
        ) -> Vec<crate::models::IsGrubu> {
            let mut node_list = Vec::new();
            for g in db_gruplar {
                if g.ust_grup_id == parent_id {
                    let children = build_tree(db_gruplar, Some(g.id));
                    let frontend_id = format!("db_{}", g.id);
                    node_list.push(crate::models::IsGrubu {
                        id: frontend_id,
                        ad: g.ad.clone(),
                        alt_gruplar: children,
                        kalemler: Vec::new(),
                    });
                }
            }
            node_list
        }

        Ok(build_tree(&db_gruplar, None))
    }
}

#[cfg(test)]
mod testler {
    use super::Veritabani;
    use crate::models::AnalizGirdisi;
    use rusqlite::Connection;
    use std::sync::atomic::{AtomicU32, Ordering};

    static SAYAC: AtomicU32 = AtomicU32::new(0);

    fn gecici_yol() -> std::path::PathBuf {
        let n = SAYAC.fetch_add(1, Ordering::SeqCst);
        let mut yol = std::env::temp_dir();
        yol.push(format!("mm_db_{}_{}.db", std::process::id(), n));
        let _ = std::fs::remove_file(&yol);
        yol
    }

    #[test]
    fn en_son_donem_fiyati_gelir() {
        let yol = gecici_yol();
        let db = Veritabani::ac(&yol).unwrap();
        let kid = db.kitap_ekle("ÇŞB").unwrap();
        // Aynı poz, iki dönem
        db.poz_ekle(
            kid,
            2026,
            5,
            "15.150.1001",
            "Beton",
            "m³",
            Some(800.0),
            "Beton",
        )
        .unwrap();
        db.poz_ekle(
            kid,
            2026,
            6,
            "15.150.1001",
            "Beton",
            "m³",
            Some(900.0),
            "Beton",
        )
        .unwrap();
        // Arama en son (6/2026) fiyatı vermeli
        let p = db.poz_getir("15.150.1001", Some(kid)).unwrap().unwrap();
        assert_eq!(p.fiyat, Some(900.0));
        assert_eq!((p.yil, p.ay), (2026, 6));
        // Tek kimlik (kurum içinde poz bir kez)
        assert_eq!(db.poz_sayisi().unwrap(), 1);
        // İki dönem görünür
        let d = db.donemler(kid).unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!((d[0].yil, d[0].ay), (2026, 6));
        // Belirli dönem seçimi: 5/2026 ESKİ fiyatı (800) vermeli
        let eski = db.pozlari_listele_donem(kid, 2026, 5, "").unwrap();
        assert_eq!(eski.len(), 1);
        assert_eq!(eski[0].fiyat, Some(800.0));
        assert_eq!((eski[0].yil, eski[0].ay), (2026, 5));
        drop(db);
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn analiz_kaydet_getir_ve_fiyat_uygula() {
        let yol = gecici_yol();
        let db = Veritabani::ac(&yol).unwrap();
        let kid = db.kitap_ekle("Test").unwrap();
        db.poz_ekle(kid, 2026, 5, "15.100.1001", "Test poz", "m³", None, "Beton")
            .unwrap();
        let girdiler = vec![
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
        ];
        db.analiz_kaydet(kid, "15.100.1001", &girdiler).unwrap();
        assert_eq!(db.analiz_getir(kid, "15.100.1001").unwrap().len(), 2);
        assert!(db
            .analizli_poz_nolari(kid)
            .unwrap()
            .contains(&"15.100.1001".to_string()));
        // Analiz sonucunu poz fiyatı yap (5/2026)
        db.poz_fiyat_guncelle(kid, "15.100.1001", 2026, 5, 562.5)
            .unwrap();
        assert_eq!(
            db.poz_getir("15.100.1001", Some(kid))
                .unwrap()
                .unwrap()
                .fiyat,
            Some(562.5)
        );
        drop(db);
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn veri_paketi_disa_ice_roundtrip() {
        let yol = gecici_yol();
        let db = Veritabani::ac(&yol).unwrap();
        let kid = db.kitap_ekle("ÇŞB").unwrap();
        db.poz_ekle(
            kid,
            2026,
            5,
            "15.150.1001",
            "Beton",
            "m³",
            Some(800.0),
            "Beton",
        )
        .unwrap();
        db.poz_ekle(
            kid,
            2026,
            6,
            "15.150.1001",
            "Beton",
            "m³",
            Some(900.0),
            "Beton",
        )
        .unwrap();
        let paket = db.kurum_disa_aktar(kid).unwrap();
        assert_eq!(paket.kurum, "ÇŞB");
        assert_eq!(paket.pozlar.len(), 1);
        assert_eq!(paket.pozlar[0].fiyatlar.len(), 2);

        let yol2 = gecici_yol();
        let db2 = Veritabani::ac(&yol2).unwrap();
        let (yeni_kid, n) = db2.kurum_ice_aktar(&paket).unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            db2.poz_getir("15.150.1001", Some(yeni_kid))
                .unwrap()
                .unwrap()
                .fiyat,
            Some(900.0)
        );
        assert_eq!(db2.donemler(yeni_kid).unwrap().len(), 2);
        drop(db);
        drop(db2);
        let _ = std::fs::remove_file(&yol);
        let _ = std::fs::remove_file(&yol2);
    }

    #[test]
    fn poz_fiyat_asof_o_tarihte_gecerli_rayici_verir() {
        let yol = gecici_yol();
        let db = Veritabani::ac(&yol).unwrap();
        let kid = db.kitap_ekle("ÇŞB").unwrap();
        db.poz_ekle(
            kid,
            2026,
            3,
            "15.150.1001",
            "Beton",
            "m³",
            Some(800.0),
            "Beton",
        )
        .unwrap();
        db.poz_ekle(
            kid,
            2026,
            6,
            "15.150.1001",
            "Beton",
            "m³",
            Some(900.0),
            "Beton",
        )
        .unwrap();
        // 2026/05: Haziran henüz yok → Mart rayici (800)
        assert_eq!(
            db.poz_fiyat_asof(kid, "15.150.1001", 2026, 5).unwrap(),
            Some(800.0)
        );
        // Tam Haziran ve sonrası → 900
        assert_eq!(
            db.poz_fiyat_asof(kid, "15.150.1001", 2026, 6).unwrap(),
            Some(900.0)
        );
        assert_eq!(
            db.poz_fiyat_asof(kid, "15.150.1001", 2027, 1).unwrap(),
            Some(900.0)
        );
        // İlk dönemden önce → yok
        assert_eq!(
            db.poz_fiyat_asof(kid, "15.150.1001", 2026, 1).unwrap(),
            None
        );
        drop(db);
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn yedek_al_ve_gecerli_dosya_uretir() {
        let yol = gecici_yol();
        let db = Veritabani::ac(&yol).unwrap();
        let kid = db.kitap_ekle("ÇŞB").unwrap();
        db.poz_ekle(
            kid,
            2026,
            6,
            "15.150.1001",
            "Beton",
            "m³",
            Some(900.0),
            "Beton",
        )
        .unwrap();

        let yedek = gecici_yol();
        db.yedekle(&yedek).unwrap();
        assert!(yedek.exists());
        Veritabani::yedek_dogrula(&yedek).expect("Metrajmatik yedeği doğrulanmalı");
        // Var olan hedefin üzerine de yazabilmeli (önce siler)
        db.yedekle(&yedek).unwrap();

        // Yedek açılıp aynı veriyi vermeli
        let db2 = Veritabani::ac(&yedek).unwrap();
        assert_eq!(db2.poz_sayisi().unwrap(), 1);
        let kitaplar = db2.kitaplari_listele().unwrap();
        assert_eq!(kitaplar.len(), 1);
        assert_eq!(kitaplar[0].ad, "ÇŞB");

        drop(db);
        drop(db2);
        let _ = std::fs::remove_file(&yol);
        let _ = std::fs::remove_file(&yedek);
    }

    #[test]
    fn bos_sqlite_dosyasi_yedek_sayilmaz_ve_degistirilmez() {
        let yol = gecici_yol();
        std::fs::File::create(&yol).unwrap();
        let onceki_boyut = std::fs::metadata(&yol).unwrap().len();

        assert!(Veritabani::yedek_dogrula(&yol).is_err());
        assert_eq!(std::fs::metadata(&yol).unwrap().len(), onceki_boyut);

        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn v1_semasindan_goc() {
        let yol = gecici_yol();
        // Elle v1 şema oluştur (kurum+dönem ayrı kitap, pozlar.yil)
        {
            let c = Connection::open(&yol).unwrap();
            c.execute_batch(
                "CREATE TABLE kitaplar (id INTEGER PRIMARY KEY AUTOINCREMENT, ad TEXT NOT NULL, yil INTEGER, ay INTEGER, tarih TEXT DEFAULT '');
                 CREATE TABLE pozlar (id INTEGER PRIMARY KEY AUTOINCREMENT, poz_no TEXT, tanim TEXT, birim TEXT, fiyat REAL, kategori TEXT, kitap_id INTEGER, kitap_adi TEXT, yil INTEGER, ay INTEGER);
                 CREATE TABLE analiz_girdileri (id INTEGER PRIMARY KEY AUTOINCREMENT, kitap_id INTEGER, poz_no TEXT, girdi_no TEXT, tanim TEXT, birim TEXT, birim_fiyat REAL, miktar REAL, tur TEXT, sira INTEGER);",
            ).unwrap();
            // Aynı kurum (ÇŞB) iki dönem = iki eski kitap
            c.execute(
                "INSERT INTO kitaplar (id, ad, yil, ay) VALUES (1,'ÇŞB',2026,5)",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO kitaplar (id, ad, yil, ay) VALUES (2,'ÇŞB',2026,6)",
                [],
            )
            .unwrap();
            c.execute("INSERT INTO pozlar (poz_no,tanim,birim,fiyat,kategori,kitap_id,kitap_adi,yil,ay) VALUES ('15.150.1001','Beton','m³',800.0,'Beton',1,'ÇŞB',2026,5)", []).unwrap();
            c.execute("INSERT INTO pozlar (poz_no,tanim,birim,fiyat,kategori,kitap_id,kitap_adi,yil,ay) VALUES ('15.150.1001','Beton','m³',900.0,'Beton',2,'ÇŞB',2026,6)", []).unwrap();
            // Özel poz (yalnız 6/2026) + analiz (eski kitap_id=2)
            c.execute("INSERT INTO pozlar (poz_no,tanim,birim,fiyat,kategori,kitap_id,kitap_adi,yil,ay) VALUES ('OZ.1','Özel','ad',10.0,'Özel',2,'ÇŞB',2026,6)", []).unwrap();
            c.execute("INSERT INTO analiz_girdileri (kitap_id,poz_no,girdi_no,tanim,birim,birim_fiyat,miktar,tur,sira) VALUES (2,'OZ.1','R1','rayic','ad',5.0,2.0,'Malzeme',0)", []).unwrap();
        }
        // Aç → göç tetiklenir
        let db = Veritabani::ac(&yol).unwrap();
        // Tek kurum, tek poz kimliği (Beton) + özel poz = 2 kimlik
        let kitaplar = db.kitaplari_listele().unwrap();
        assert_eq!(kitaplar.len(), 1);
        let kid = kitaplar[0].id;
        assert_eq!(kitaplar[0].ad, "ÇŞB");
        assert_eq!(db.poz_sayisi().unwrap(), 2);
        // Beton en son 6/2026 = 900
        let beton = db.poz_getir("15.150.1001", Some(kid)).unwrap().unwrap();
        assert_eq!(beton.fiyat, Some(900.0));
        // İki dönem korunmuş
        assert_eq!(db.donemler(kid).unwrap().len(), 2);
        // Analiz remap edilmiş (yeni kurum id ile erişilebilir)
        assert_eq!(db.analiz_getir(kid, "OZ.1").unwrap().len(), 1);
        drop(db);
        let _ = std::fs::remove_file(&yol);
    }
}
