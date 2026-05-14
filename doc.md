# 🏗 Metrajmatik - Yaklaşık Maliyet / Metraj Programı

**Versiyon:** 0.1.0  
**Tarih:** Mayıs 2026  
**Geliştirme:** Rust (egui/eframe GUI)

---

## 📖 İçindekiler

1. [Genel Bakış](#genel-bakış)
2. [Sistem Gereksinimleri](#sistem-gereksinimleri)
3. [Kurulum ve Çalıştırma](#kurulum-ve-çalıştırma)
4. [Kullanım Kılavuzu](#kullanım-kılavuzu)
   - [PDF Yükleme](#1-pdf-birim-fiyat-listesini-yükleme)
   - [Poz Arama](#2-poz-arama)
   - [Metraj Oluşturma](#3-metraj-oluşturma)
   - [Metraj Yönetimi](#4-metraj-yönetimi)
   - [Dışa Aktarma](#5-dışa-aktarma)
5. [Proje Yapısı](#proje-yapısı)
6. [Teknik Detaylar](#teknik-detaylar)
7. [Derleme Talimatları](#derleme-talimatları)

---

## Genel Bakış

Metrajmatik, Çevre ve Şehircilik Bakanlığı güncel birim fiyat listelerini PDF'ten otomatik olarak okuyup veritabanına kaydeden, poz numarası ve açıklama ile hızlı arama yapabilen, kullanıcının metraj kalemlerini girip toplam yaklaşık maliyeti hesaplayabildiği bir masaüstü uygulamasıdır.

**Ana Özellikler:**
- 📄 PDF birim fiyat listesini otomatik ayrıştırma
- 🔍 Poz numarası ve açıklama ile anlık arama
- 📋 Çoklu kalemli metraj tablosu
- 💰 Otomatik tutar hesaplama
- 💾 JSON ile metraj kaydetme/yükleme
- 📊 Excel (.xlsx) dışa aktarma
- 🗂 Kategori bazlı filtreleme

---

## Sistem Gereksinimleri

- **İşletim Sistemi:** Windows 10/11 (64-bit)
- **RAM:** En az 512 MB
- **Disk:** ~50 MB boş alan
- **Bağımlılık:** Yok (tek .exe dosyası, portable)

---

## Kurulum ve Çalıştırma

### Hazır Exe ile (Önerilen)

1. `target/release/metrajmatik.exe` dosyasını istediğiniz klasöre kopyalayın
2. `20206-05-BF.pdf` (veya kendi PDF'inizi) aynı klasöre veya üst klasöre koyun
3. `metrajmatik.exe` dosyasına çift tıklayarak çalıştırın

### Cargo ile (Geliştiriciler)

```powershell
cd metrajmatik
cargo run --release
```

---

## Kullanım Kılavuzu

Uygulama açıldığında iki sekme göreceksiniz: **📋 Metraj Tablosu** ve **📄 PDF Yükle**. Alt kısımda sürekli görünen bir durum çubuğu bulunur.

### 1. PDF Birim Fiyat Listesini Yükleme

Bu adım, birim fiyat verilerini programa tanıtmak için gereklidir.

**Adımlar:**
1. **📄 PDF Yükle** sekmesine tıklayın
2. İki seçeneğiniz var:
   - **Hızlı Yükle:** Eğer `20206-05-BF.pdf` dosyası programla aynı veya üst klasördeyse, yeşil butona tıklayarak doğrudan yükleyebilirsiniz
   - **Dosya Seç:** Farklı bir PDF için "📂 PDF Dosyası Seç ve Yükle" butonuna tıklayın ve PDF'inizi seçin
3. Yükleme sırasında bir spinner animasyonu ve durum mesajları göreceksiniz
4. Başarılı yükleme sonrası "✅ Başarıyla X poz yüklendi!" mesajını alacaksınız

**Desteklenen PDF Formatı:**
- Çevre ve Şehircilik Bakanlığı güncel birim fiyat listeleri
- Poz No: `XX.XXX.XXXX` formatında
- Fiyat: `X.XXX,XX` veya `XXX,XX` formatında TL

> ⚠️ **Not:** Farklı formatlardaki PDF'lerde ayrıştırma hataları olabilir. Bu durumda PDF parser regex desenleri güncellenmelidir.

---

### 2. Poz Arama

PDF yüklendikten sonra **📋 Metraj Tablosu** sekmesine geçin. Sol panelde arama bölümü bulunur.

#### Poz No ile Arama

1. **"Poz No"** alanına poz numarasını yazmaya başlayın (örn: `15.100`)
2. Yazdıkça anlık olarak eşleşen pozlar listelenecektir
3. Listeden bir poza tıklayarak seçin
4. Seçili pozun detayları (açıklama, birim, fiyat, kategori) altta görünecektir

#### Açıklama ile Arama

1. **"Açıklama"** alanına anahtar kelime yazın (örn: `beton`, `tuğla`, `kazı`)
2. FTS5 tam metin arama motoru ilgili tüm pozları bulur
3. Sonuçlara tıklayarak poz seçebilirsiniz

#### Kategori Filtresi

1. **"Kategori"** dropdown menüsünden bir kategori seçin
2. O kategorideki tüm pozlar listelenecektir
3. "TÜMÜ" seçeneği ile filtreyi kaldırabilirsiniz

> 💡 **İpucu:** Her pozun üzerine fare ile gelerek (hover) tam açıklamasını tooltip olarak görebilirsiniz.

> ⚠️ **Önemli:** Fiyatı `---` (formül) olarak görünen pozlar metraja eklenemez. Bunlar genellikle derinlik zammı gibi formül içeren özel pozlardır.

---

### 3. Metraj Oluşturma

Poz seçtikten sonra sağ panelde metraj tablosuna kalem ekleyebilirsiniz.

**Adımlar:**
1. Sol panelden bir poz seçin (veya doğrudan **"Poz No"** alanına yazın)
2. Seçili pozun bilgileri otomatik olarak "Poz No" alanına ve alt kısma gelecektir
3. **"Miktar"** alanına miktarı girin (örn: `150.5`)
4. **"➕ Kalem Ekle"** butonuna tıklayın
5. Kalem metraj tablosuna eklenecek ve toplam tutar güncellenecektir

**Metraj Tablosu Özellikleri:**
- Her kalem için: Sıra No, Poz No, Açıklama, Birim, Birim Fiyat, **Miktar** (düzenlenebilir), Tutar
- Miktar hücresine tıklayıp değiştirebilirsiniz, tutar otomatik güncellenir
- **✕** butonu ile kalem silebilirsiniz
- En altta **GENEL TOPLAM** canlı olarak görünür

---

### 4. Metraj Yönetimi

Sağ paneldeki butonlarla metrajınızı yönetebilirsiniz:

| Buton | İşlev |
|-------|-------|
| 📂 **Metraj Yükle** | Daha önce kaydettiğiniz bir JSON metraj dosyasını açar |
| 💾 **Kaydet** | Mevcut metrajı JSON dosyası olarak kaydeder |
| 📊 **Excel'e Aktar** | Metrajı formatlı Excel (.xlsx) dosyasına aktarır |
| 🗑 **Temizle** | Tüm metraj kalemlerini siler (onay ister) |

**Metraj Adı:** Tablonun üst kısmındaki metraj adını değiştirebilirsiniz. Bu isim kaydetme ve Excel aktarma sırasında dosya adı olarak kullanılır.

---

### 5. Dışa Aktarma

#### Excel Aktarımı (.xlsx)

Excel dosyası şu formatta oluşturulur:

| Sıra No | Poz No | Açıklama | Birim | Birim Fiyat (TL) | Miktar | Tutar (TL) |
|---------|--------|----------|-------|------------------|--------|------------|
| 1 | 15.100.1001 | ... | Ton | 280,21 | 10,00 | 2.802,10 |
| ... | ... | ... | ... | ... | ... | ... |
| | | | | | **GENEL TOPLAM** | **X.XXX,XX** |

- Başlık ve sütunlar renklendirilmiş ve formatlanmıştır
- Sayısal değerler binlik ayraçlı ve 2 ondalıklıdır
- Genel toplam yeşil zeminli, beyaz yazılıdır

#### JSON Kaydetme

Metrajınızı `.json` formatında kaydedip daha sonra yükleyebilirsiniz:
```json
{
  "ad": "Ornek Metraj",
  "tarih": "2026-05-14",
  "kalemler": [
    {
      "poz_no": "15.100.1001",
      "tanim": "1 ton her cins çimento...",
      "birim": "Ton",
      "birim_fiyat": 280.21,
      "miktar": 10.0,
      "tutar": 2802.10
    }
  ]
}
```

---

## Proje Yapısı

```
metrajmatik/
├── Cargo.toml              # Rust bağımlılıkları
├── doc.md                  # Bu doküman
├── src/
│   ├── main.rs             # Uygulama giriş noktası
│   ├── app.rs              # egui/eframe UI ve uygulama durumu
│   ├── models.rs           # Veri modelleri (Poz, MetrajKalemi, KayitliMetraj)
│   ├── pdf_parser.rs       # PDF metin çıkarma ve poz ayrıştırma motoru
│   ├── database.rs         # SQLite + FTS5 veritabanı işlemleri
│   └── export.rs           # Excel (.xlsx) ve JSON dışa/içe aktarma
└── target/release/
    └── metrajmatik.exe     # Derlenmiş çalıştırılabilir dosya
```

---

## Teknik Detaylar

### Kullanılan Teknolojiler

| Teknoloji | Sürüm | Amaç |
|-----------|-------|------|
| **Rust** | 1.85+ | Programlama dili |
| **egui/eframe** | 0.31 | Modern GUI framework |
| **rusqlite** | 0.33 | SQLite veritabanı (gömülü) |
| **pdf-extract** | 0.7 | PDF metin çıkarma |
| **regex** | 1 | Poz/fiyat ayrıştırma (regex) |
| **rust_xlsxwriter** | 0.83 | Excel dosyası oluşturma |
| **serde/serde_json** | 1 | JSON serileştirme |
| **rfd** | 0.15 | Dosya seçim diyalogları |

### Veritabanı Şeması

```sql
-- Ana poz tablosu
CREATE TABLE pozlar (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    poz_no TEXT UNIQUE NOT NULL,
    tanim TEXT NOT NULL,
    birim TEXT NOT NULL,
    fiyat REAL,
    kategori TEXT NOT NULL
);

-- Tam metin arama (FTS5)
CREATE VIRTUAL TABLE pozlar_fts USING fts5(
    poz_no, tanim, birim, kategori,
    content='pozlar',
    content_rowid='id'
);
```

### PDF Ayrıştırma Algoritması

1. `pdf-extract` ile PDF'ten ham metin çıkarılır
2. Regex ile `XX.XXX.XXXX` formatında poz numaraları tespit edilir
3. Çok satırlı tanımlar toplanır (fiyat bulunana kadar)
4. Satır sonunda `X.XXX,XX` formatında fiyat aranır
5. Birim (m³, m², Ton, Kg, Ad, m) tanım ve fiyat arasından ayrıştırılır
6. Kategori başlıkları anahtar kelime eşleştirme ile tespit edilir
7. Sayfa numaraları ve başlık satırları filtrelenir

---

## Derleme Talimatları

### Gereksinimler

- Rust toolchain (https://rustup.rs)
- Windows 10/11

### Geliştirme Derlemesi

```powershell
cd d:\metrajmatik\metrajmatik
cargo build
```

### Release Derlemesi

```powershell
cargo build --release
```

Çıktı: `target/release/metrajmatik.exe`

### Test

```powershell
cargo test
```

### Bağımlılıkları Güncelleme

```powershell
cargo update
```

---

## Sık Sorulan Sorular

### PDF yüklenirken hata alıyorum
PDF'in Çevre ve Şehircilik Bakanlığı formatında olduğundan emin olun. Farklı formatlardaki PDF'ler için `pdf_parser.rs` dosyasındaki regex desenleri güncellenmelidir.

### Poz bulunamıyor
- PDF'i yüklediğinizden emin olun (durum çubuğunda poz sayısı görünmelidir)
- Poz numarasını doğru formatta yazın: `XX.XXX.XXXX`
- Tam numarayı bilmiyorsanız ilk birkaç haneyi yazıp LIKE araması yapabilirsiniz (örn: `15.100`)
- Alternatif olarak açıklama aramasını kullanın

### Excel aktarımında Türkçe karakter sorunu
Uygulama çıktıları UTF-8 kodlamalıdır. Excel'de açarken UTF-8 olarak içe aktarın.

---

## Lisans

Bu proje özel kullanım için geliştirilmiştir.

---

*Metrajmatik - Mayıs 2026*