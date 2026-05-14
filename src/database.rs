use rusqlite::{params, Connection, Result};
use std::path::Path;

use crate::models::{Kitap, Poz};

pub struct Veritabani {
    conn: Connection,
}

impl Veritabani {
    pub fn ac(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Veritabani { conn };
        db.tablolari_olustur()?;
        Ok(db)
    }

    fn tablolari_olustur(&self) -> Result<()> {
        // Migration: eski semayi kontrol et
        let eski_sema = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('pozlar') WHERE name='kitap_id'",
            [],
            |row| row.get::<_, u32>(0),
        ).unwrap_or(0);

        if eski_sema == 0 {
            // Eski sema - tablolari yeniden olustur
            log::info!("Eski veritabani semasi tespit edildi, migration yapiliyor...");
            self.conn.execute_batch(
                "DROP TABLE IF EXISTS pozlar_fts;
                 DROP TABLE IF EXISTS pozlar;
                 DROP TABLE IF EXISTS kitaplar;",
            )?;
        }

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kitaplar (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ad TEXT NOT NULL,
                tarih TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS pozlar (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                poz_no TEXT NOT NULL,
                tanim TEXT NOT NULL,
                birim TEXT NOT NULL,
                fiyat REAL,
                kategori TEXT NOT NULL,
                kitap_id INTEGER NOT NULL,
                kitap_adi TEXT NOT NULL DEFAULT '',
                UNIQUE(poz_no, kitap_id),
                FOREIGN KEY(kitap_id) REFERENCES kitaplar(id) ON DELETE CASCADE
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS pozlar_fts USING fts5(
                poz_no, tanim, birim, kategori, kitap_adi,
                content='pozlar',
                content_rowid='id'
            );
            CREATE TRIGGER IF NOT EXISTS pozlar_ai AFTER INSERT ON pozlar BEGIN
                INSERT INTO pozlar_fts(rowid, poz_no, tanim, birim, kategori, kitap_adi)
                VALUES (new.id, new.poz_no, new.tanim, new.birim, new.kategori, new.kitap_adi);
            END;
            CREATE TRIGGER IF NOT EXISTS pozlar_ad AFTER DELETE ON pozlar BEGIN
                INSERT INTO pozlar_fts(pozlar_fts, rowid, poz_no, tanim, birim, kategori, kitap_adi)
                VALUES('delete', old.id, old.poz_no, old.tanim, old.birim, old.kategori, old.kitap_adi);
            END;
            CREATE TRIGGER IF NOT EXISTS pozlar_au AFTER UPDATE ON pozlar BEGIN
                INSERT INTO pozlar_fts(pozlar_fts, rowid, poz_no, tanim, birim, kategori, kitap_adi)
                VALUES('delete', old.id, old.poz_no, old.tanim, old.birim, old.kategori, old.kitap_adi);
                INSERT INTO pozlar_fts(rowid, poz_no, tanim, birim, kategori, kitap_adi)
                VALUES (new.id, new.poz_no, new.tanim, new.birim, new.kategori, new.kitap_adi);
            END;",
        )?;

        // Pragmalar
        self.conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode=WAL;")?;
        Ok(())
    }

    // ==================== KITAP ISLEMLERI ====================

    pub fn kitap_ekle(&self, ad: &str) -> Result<i64> {
        let tarih = krono_tarih();
        self.conn.execute(
            "INSERT INTO kitaplar (ad, tarih) VALUES (?1, ?2)",
            params![ad, tarih],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn kitaplari_listele(&self) -> Result<Vec<Kitap>> {
        let mut stmt = self.conn.prepare(
            "SELECT k.id, k.ad, COUNT(p.id) as poz_sayisi, k.tarih
             FROM kitaplar k LEFT JOIN pozlar p ON p.kitap_id = k.id
             GROUP BY k.id ORDER BY k.id",
        )?;
        let sonuc = stmt
            .query_map([], |row| {
                Ok(Kitap {
                    id: row.get(0)?,
                    ad: row.get(1)?,
                    poz_sayisi: row.get(2)?,
                    tarih: row.get(3)?,
                })
            })?
            .filter_map(|k| k.ok())
            .collect();
        Ok(sonuc)
    }

    pub fn kitap_sil(&self, kitap_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM pozlar WHERE kitap_id = ?1", params![kitap_id])?;
        self.conn
            .execute("DELETE FROM kitaplar WHERE id = ?1", params![kitap_id])?;
        Ok(())
    }

    pub fn kitap_getir(&self, kitap_id: i64) -> Result<Option<Kitap>> {
        let mut stmt = self.conn.prepare(
            "SELECT k.id, k.ad, COUNT(p.id), k.tarih FROM kitaplar k LEFT JOIN pozlar p ON p.kitap_id = k.id WHERE k.id = ?1 GROUP BY k.id",
        )?;
        let mut sonuc = stmt
            .query_map(params![kitap_id], |row| {
                Ok(Kitap {
                    id: row.get(0)?,
                    ad: row.get(1)?,
                    poz_sayisi: row.get(2)?,
                    tarih: row.get(3)?,
                })
            })?
            .filter_map(|k| k.ok());
        Ok(sonuc.next())
    }

    // ==================== POZ ISLEMLERI ====================

    pub fn pozlari_yukle(&self, kitap_id: i64, kitap_adi: &str, pozlar: &[Poz]) -> Result<usize> {
        // Bu kitabın eski pozlarını sil
        self.conn
            .execute("DELETE FROM pozlar WHERE kitap_id = ?1", params![kitap_id])?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO pozlar (poz_no, tanim, birim, fiyat, kategori, kitap_id, kitap_adi)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;

        let mut eklenen = 0;
        for poz in pozlar {
            stmt.execute(params![
                poz.poz_no,
                poz.tanim,
                poz.birim,
                poz.fiyat,
                poz.kategori,
                kitap_id,
                kitap_adi,
            ])?;
            eklenen += 1;
        }

        Ok(eklenen)
    }

    pub fn poz_no_ara(&self, poz_no: &str, kitap_id: Option<i64>) -> Result<Vec<Poz>> {
        let sql = if let Some(kid) = kitap_id {
            format!(
                "SELECT poz_no, tanim, birim, fiyat, kategori, kitap_id, kitap_adi FROM pozlar
                 WHERE poz_no LIKE ?1 AND kitap_id = {}
                 ORDER BY poz_no LIMIT 50",
                kid
            )
        } else {
            "SELECT poz_no, tanim, birim, fiyat, kategori, kitap_id, kitap_adi FROM pozlar
             WHERE poz_no LIKE ?1 ORDER BY poz_no LIMIT 50"
            .to_string()
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let sonuc = stmt
            .query_map(params![format!("{}%", poz_no)], |row| {
                Ok(Poz {
                    poz_no: row.get(0)?,
                    tanim: row.get(1)?,
                    birim: row.get(2)?,
                    fiyat: row.get(3)?,
                    kategori: row.get(4)?,
                    kitap_id: row.get(5)?,
                    kitap_adi: row.get(6)?,
                })
            })?
            .filter_map(|p| p.ok())
            .collect();
        Ok(sonuc)
    }

    pub fn tam_metin_ara(&self, sorgu: &str, kitap_id: Option<i64>) -> Result<Vec<Poz>> {
        let terimler: Vec<String> = sorgu
            .split_whitespace()
            .map(|t| format!("\"{}\"*", t.replace('"', "")))
            .collect();
        let fts_sorgu = terimler.join(" AND ");

        let kitap_filtre = if let Some(kid) = kitap_id {
            format!(" AND p.kitap_id = {}", kid)
        } else {
            String::new()
        };

        let sql = format!(
            "SELECT p.poz_no, p.tanim, p.birim, p.fiyat, p.kategori, p.kitap_id, p.kitap_adi
             FROM pozlar_fts f JOIN pozlar p ON f.rowid = p.id
             WHERE pozlar_fts MATCH ?1 {}
             ORDER BY rank LIMIT 100",
            kitap_filtre
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let sonuc = stmt
            .query_map(params![fts_sorgu], |row| {
                Ok(Poz {
                    poz_no: row.get(0)?,
                    tanim: row.get(1)?,
                    birim: row.get(2)?,
                    fiyat: row.get(3)?,
                    kategori: row.get(4)?,
                    kitap_id: row.get(5)?,
                    kitap_adi: row.get(6)?,
                })
            })?
            .filter_map(|p| p.ok())
            .collect();
        Ok(sonuc)
    }

    pub fn poz_getir(&self, poz_no: &str, kitap_id: Option<i64>) -> Result<Option<Poz>> {
        let sql = if let Some(kid) = kitap_id {
            format!(
                "SELECT poz_no, tanim, birim, fiyat, kategori, kitap_id, kitap_adi FROM pozlar
                 WHERE poz_no = ?1 AND kitap_id = {}",
                kid
            )
        } else {
            "SELECT poz_no, tanim, birim, fiyat, kategori, kitap_id, kitap_adi FROM pozlar WHERE poz_no = ?1 LIMIT 1".to_string()
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let mut sonuc = stmt
            .query_map(params![poz_no], |row| {
                Ok(Poz {
                    poz_no: row.get(0)?,
                    tanim: row.get(1)?,
                    birim: row.get(2)?,
                    fiyat: row.get(3)?,
                    kategori: row.get(4)?,
                    kitap_id: row.get(5)?,
                    kitap_adi: row.get(6)?,
                })
            })?
            .filter_map(|p| p.ok());
        Ok(sonuc.next())
    }

    pub fn tum_pozlar(&self, kitap_id: Option<i64>) -> Result<Vec<Poz>> {
        let sql = if let Some(kid) = kitap_id {
            format!(
                "SELECT poz_no, tanim, birim, fiyat, kategori, kitap_id, kitap_adi FROM pozlar
                 WHERE kitap_id = {} ORDER BY poz_no",
                kid
            )
        } else {
            "SELECT poz_no, tanim, birim, fiyat, kategori, kitap_id, kitap_adi FROM pozlar ORDER BY poz_no".to_string()
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let sonuc = stmt
            .query_map([], |row| {
                Ok(Poz {
                    poz_no: row.get(0)?,
                    tanim: row.get(1)?,
                    birim: row.get(2)?,
                    fiyat: row.get(3)?,
                    kategori: row.get(4)?,
                    kitap_id: row.get(5)?,
                    kitap_adi: row.get(6)?,
                })
            })?
            .filter_map(|p| p.ok())
            .collect();
        Ok(sonuc)
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
        let sonuc = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|k| k.ok())
            .collect();
        Ok(sonuc)
    }

    pub fn poz_sayisi(&self) -> Result<u32> {
        self.conn
            .query_row("SELECT COUNT(*) FROM pozlar", [], |row| row.get(0))
    }
}

fn krono_tarih() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let rem = days % 365;
    let months = rem / 30;
    let days = rem % 30;
    format!("{:04}-{:02}-{:02}", years, months + 1, days + 1)
}