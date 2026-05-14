use rusqlite::{params, Connection, Result};
use std::path::Path;

use crate::models::Poz;

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
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pozlar (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                poz_no TEXT UNIQUE NOT NULL,
                tanim TEXT NOT NULL,
                birim TEXT NOT NULL,
                fiyat REAL,
                kategori TEXT NOT NULL
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS pozlar_fts USING fts5(
                poz_no, tanim, birim, kategori,
                content='pozlar',
                content_rowid='id'
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
            END;",
        )?;
        Ok(())
    }

    /// Toplu poz ekleme (önce tabloyu temizler)
    pub fn pozlari_yukle(&self, pozlar: &[Poz]) -> Result<usize> {
        self.conn.execute("DELETE FROM pozlar", [])?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO pozlar (poz_no, tanim, birim, fiyat, kategori) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;

        let mut eklenen = 0;
        for poz in pozlar {
            stmt.execute(params![
                poz.poz_no,
                poz.tanim,
                poz.birim,
                poz.fiyat,
                poz.kategori
            ])?;
            eklenen += 1;
        }

        Ok(eklenen)
    }

    /// Poz numarasına göre arama
    pub fn poz_no_ara(&self, poz_no: &str) -> Result<Vec<Poz>> {
        let mut stmt = self.conn.prepare(
            "SELECT poz_no, tanim, birim, fiyat, kategori FROM pozlar
             WHERE poz_no LIKE ?1
             ORDER BY poz_no
             LIMIT 50",
        )?;

        let sonuc = stmt
            .query_map(params![format!("{}%", poz_no)], |row| {
                Ok(Poz {
                    poz_no: row.get(0)?,
                    tanim: row.get(1)?,
                    birim: row.get(2)?,
                    fiyat: row.get(3)?,
                    kategori: row.get(4)?,
                })
            })?
            .filter_map(|p| p.ok())
            .collect();

        Ok(sonuc)
    }

    /// Tam metin araması (açıklama ve poz no üzerinde)
    pub fn tam_metin_ara(&self, sorgu: &str) -> Result<Vec<Poz>> {
        // FTS5 sorgusu için kelimeleri * ile birleştir (prefix search)
        let terimler: Vec<String> = sorgu
            .split_whitespace()
            .map(|t| format!("\"{}\"*", t.replace('"', "")))
            .collect();
        let fts_sorgu = terimler.join(" AND ");

        let sql = format!(
            "SELECT p.poz_no, p.tanim, p.birim, p.fiyat, p.kategori
             FROM pozlar_fts f
             JOIN pozlar p ON f.rowid = p.id
             WHERE pozlar_fts MATCH ?1
             ORDER BY rank
             LIMIT 100"
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
                })
            })?
            .filter_map(|p| p.ok())
            .collect();

        Ok(sonuc)
    }

    /// Belirli bir poz numarasını getir
    pub fn poz_getir(&self, poz_no: &str) -> Result<Option<Poz>> {
        let mut stmt = self.conn.prepare(
            "SELECT poz_no, tanim, birim, fiyat, kategori FROM pozlar WHERE poz_no = ?1",
        )?;

        let mut sonuc = stmt
            .query_map(params![poz_no], |row| {
                Ok(Poz {
                    poz_no: row.get(0)?,
                    tanim: row.get(1)?,
                    birim: row.get(2)?,
                    fiyat: row.get(3)?,
                    kategori: row.get(4)?,
                })
            })?
            .filter_map(|p| p.ok());

        Ok(sonuc.next())
    }

    /// Tüm pozları listele
    pub fn tum_pozlar(&self) -> Result<Vec<Poz>> {
        let mut stmt = self.conn.prepare(
            "SELECT poz_no, tanim, birim, fiyat, kategori FROM pozlar ORDER BY poz_no",
        )?;

        let sonuc = stmt
            .query_map([], |row| {
                Ok(Poz {
                    poz_no: row.get(0)?,
                    tanim: row.get(1)?,
                    birim: row.get(2)?,
                    fiyat: row.get(3)?,
                    kategori: row.get(4)?,
                })
            })?
            .filter_map(|p| p.ok())
            .collect();

        Ok(sonuc)
    }

    /// Kategorileri listele
    pub fn kategoriler(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT kategori FROM pozlar ORDER BY kategori",
        )?;

        let sonuc = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|k| k.ok())
            .collect();

        Ok(sonuc)
    }

    /// Toplam poz sayısı
    pub fn poz_sayisi(&self) -> Result<u32> {
        self.conn
            .query_row("SELECT COUNT(*) FROM pozlar", [], |row| row.get(0))
    }
}