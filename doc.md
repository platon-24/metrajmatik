# 🏗 Metrajmatik - Yaklaşık Maliyet / Metraj Programı

**Versiyon:** 0.2.0  
**Tarih:** Mayıs 2026  
**Teknoloji:** Rust (egui/eframe GUI)

---

## 📖 İçindekiler

1. [Genel Bakış](#genel-bakış)
2. [Sistem Gereksinimleri](#sistem-gereksinimleri)
3. [Kurulum ve Çalıştırma](#kurulum-ve-çalıştırma)
4. [Hızlı Başlangıç](#hızlı-başlangıç)
5. [Kullanım Kılavuzu](#kullanım-kılavuzu)
   - [Kitap Yöneticisi](#1-kitap-yöneticisi)
   - [PDF Yükleme](#2-pdf-yükleme)
   - [Metraj Oluşturma](#3-metraj-oluşturma)
   - [Poz Arama](#4-poz-arama)
   - [Metraj Yönetimi](#5-metraj-yönetimi)
   - [Dışa Aktarma](#6-dışa-aktarma)
6. [Kısayollar](#kısayollar)
7. [Proje Yapısı](#proje-yapısı)
8. [Teknik Detaylar](#teknik-detaylar)
9. [Sık Sorulan Sorular](#sık-sorulan-sorular)

---

## Genel Bakış

Metrajmatik, birden fazla kurumun (Çevre Bakanlığı, Kültür Bakanlığı, İller Bankası, PTT vb.) aylık güncel birim fiyat listelerini PDF'ten otomatik okuyup veritabanına kaydeden, poz numarası ve açıklama ile hızlı arama yapabilen, kullanıcının metraj kalemlerini girip toplam yaklaşık maliyeti hesaplayabildiği profesyonel bir masaüstü uygulamasıdır.

**Ana Özellikler:**
- 📚 **Çoklu kitap desteği**: Birden fazla kurumun birim fiyat listelerini ayrı ayrı yönetin
- 📅 **Yıl/Ay takibi**: Aylık güncellenen fiyatları dönemlere göre saklayın
- 📄 **PDF otomatik ayrıştırma**: PDF'ten poz, tanım, birim ve fiyat bilgilerini otomatik çıkarın
- 🔍 **Çift yönlü arama**: Poz numarası ve açıklama (tam metin) ile anlık arama
- 📋 **Çoklu kalemli metraj**: Farklı kitaplardan pozları tek metrajda birleştirin
- 💰 **Otomatik tutar**: Miktar girince anında hesaplama
- 💾 **Proje kaydetme**: `.mrj` uzantılı dosyalarla çalışmalarınızı saklayın
- 📊 **Excel çıktısı**: Formatlı Excel (.xlsx) raporu
- ⌨️ **Klavye kısayolları**: Ctrl+S kaydet, Ctrl+O aç

---

## Sistem Gereksinimleri

- **İşletim Sistemi:** Windows 10/11 (64-bit)
- **RAM:** En az 512 MB
- **Disk:** ~50 MB boş alan
- **Bağımlılık:** Yok - tek `.exe` dosyası, taşınabilir

---

## Kurulum ve Çalıştırma

### Hazır Exe ile (Önerilen)

1. `target/release/metrajmatik.exe` dosyasını istediğiniz klasöre kopyalayın
2. PDF dosyalarınızı aynı klasöre koyun (veya sonradan dosya seçici ile seçin)
3. `metrajmatik.exe` dosyasına çift tıklayarak çalıştırın

İlk çalıştırmada `metrajmatik_veriler.db` otomatik oluşturulur.

### Cargo ile (Geliştiriciler)

```powershell
cd metrajmatik
cargo run --release
```

---

## Hızlı Başlangıç

Uygulamayı ilk kez açtığınızda yapmanız gerekenler:

1. **📚 Kitaplar** sekmesine tıklayın
2. Kitap adı girin (örn: `Çevre ve Şehircilik Bakanlığı`)
3. Yıl ve Ay seçin (örn: 2026 / 5)
4. **➕ Kitap Ekle** butonuna tıklayın
5. **📄 PDF Yükle** sekmesine geçin
6. Hedef kitabı seçin
7. PDF dosyasını seçip yükleyin
8. **📋 Metraj** sekmesine geçin
9. Kitap seçin, poz arayın, miktar girin, kalem ekleyin
10. **Ctrl+S** ile kaydedin

---

## Kullanım Kılavuzu

Uygulama üç sekmeden oluşur. Üst menü çubuğundan sekmeler arasında geçiş yapabilirsiniz. Alt durum çubuğunda dosya adı, kaydedilmemiş değişiklik göstergesi (●), aktif kitap bilgisi ve toplam tutar görünür.

### 1. Kitap Yöneticisi

**📚 Kitaplar** sekmesi, birim fiyat listelerini (kitapları) yönettiğiniz yerdir.

#### Yeni Kitap Ekleme

1. **Kitap Adı** alanına kurum adını yazın (örn: `Kültür Bakanlığı`)
2. **Yıl** dropdown'ından dönemi seçin (2024-2028)
3. **Ay** dropdown'ından ayı seçin (1-12)
4. **➕ Kitap Ekle** butonuna tıklayın

> 💡 **Neden Yıl/Ay?** Birim fiyatlar her ay güncellenir. Aynı kitabın farklı aylara ait fiyatlarını ayrı ayrı saklamak için yıl/ay seçimi zorunludur. Örneğin `Çevre Bakanlığı (5/2026)` ve `Çevre Bakanlığı (6/2026)` iki ayrı kitap girişidir.

#### Kitap Listesi

Eklediğiniz kitaplar tablo halinde listelenir:

| ID | Kitap Adı | Yıl | Ay | Poz | Tarih | İşlem |
|----|-----------|-----|----|-----|-------|-------|
| 1 | Çevre ve Şehircilik | 2026 | 5 | 0 | 2026-05-14 | 🗑 |

- **Kitap seçme**: Kitap adına tıklayarak aktif kitap yapabilirsiniz. Aktif kitap yeşil renkle vurgulanır.
- **Kitap silme**: 🗑 butonu ile kitabı tüm pozlarıyla birlikte silebilirsiniz.
- **Poz sayısı**: O kitaba yüklenen poz sayısını gösterir.

#### Kitap Seçme ve Kullanma

Bir kitabı seçtiğinizde:
- **📋 Metraj** sekmesinde sadece o kitabın pozları aranır
- **📄 PDF Yükle** sekmesinde PDF o kitaba yüklenir

"TÜM KİTAPLAR" seçeneği ile tüm kitaplarda arama yapabilirsiniz.

---

### 2. PDF Yükleme

**📄 PDF Yükle** sekmesi, birim fiyat PDF'lerini kitaplara yüklemek içindir.

**Adımlar:**
1. **Hedef Kitap** dropdown'ından yükleme yapacağınız kitabı seçin
   - Kitaplar `Ad (Ay/Yıl)` formatında listelenir
   - Kitap yoksa önce Kitap Yöneticisi'nden ekleme yapmalısınız
2. **📂 PDF Dosyası Seç ve Yükle** butonuna tıklayın
3. PDF dosyanızı seçin
4. Yükleme durumu ekranda gösterilir
5. Başarılı yükleme sonrası kitabın poz sayısı güncellenir

**Hızlı Yükleme:**
- Eğer `20206-05-BF.pdf` dosyası program klasöründeyse, doğrudan "Hızlı Yükle" butonu ile yükleyebilirsiniz

**Desteklenen PDF Formatı:**
- Çevre ve Şehircilik Bakanlığı güncel birim fiyat listeleri
- Poz No: `XX.XXX.XXXX` formatında
- Fiyat: `X.XXX,XX` veya `XXX,XX` formatında TL

> ⚠️ Aynı kitap+ay+yıl kombinasyonuna tekrar PDF yüklerseniz, eski pozlar silinip yenileri eklenir.

---

### 3. Metraj Oluşturma

**📋 Metraj** sekmesi, asıl çalışma alanınızdır. İki panele ayrılır:

#### Sol Panel - Poz Arama

Üst kısımda **Kitap seçici** bulunur. Buradan hangi kitapta arama yapacağınızı seçersiniz.

**Poz No ile Arama:**
1. **Poz No** alanına poz numarasının başlangıcını yazın (örn: `15.100`)
2. Anlık olarak eşleşen pozlar listelenir
3. Listeden bir poza tıklayarak seçin
4. Seçili pozun detayları altta görünür

**Açıklama ile Arama:**
1. **Açıklama** alanına anahtar kelime yazın (örn: `beton`, `tuğla`, `kazı`)
2. FTS5 tam metin arama motoru ilgili tüm pozları bulur

**Kategori Filtresi:**
- Kategori dropdown'ından belirli bir kategorideki pozları listeleyin

> 💡 Arama sonuçlarında her pozun hangi **Ay/Yıl** dönemine ait olduğu gösterilir: `15.100.1001 | m³ | 280.21 | 5/2026`

#### Sağ Panel - Metraj Tablosu

**Kalem Ekleme:**
1. Sol panelden bir poz seçin
2. **Miktar** alanına miktar girin (veya boş bırakın)
3. **➕ Kalem Ekle** butonuna tıklayın
4. Kalem metraj tablosuna eklenir

> 💡 Miktarı boş bırakırsanız kalem 0.00 miktar ile eklenir. Tablodaki miktar hücresine tıklayarak sonradan girebilirsiniz.

**Metraj Tablosu Sütunları:**

| # | Kitap | Poz No | Açıklama | Birim | B.Fiyat | Miktar | Tutar | ✕ |
|---|-------|--------|----------|-------|---------|--------|-------|---|

- **Kitap**: Kalemin hangi kitaptan alındığını gösterir
- **Miktar**: Düzenlenebilir hücre - tıklayıp değiştirebilirsiniz, tutar otomatik güncellenir
- **Tutar**: Otomatik hesaplanır (Birim Fiyat × Miktar)
- **✕**: Kalemi siler

---

### 4. Poz Arama (Detaylı)

**Seçili Poz Bilgileri:**
Poz seçtiğinizde altta detaylı bilgi kartı görünür:
- **Poz No**: Poz numarası
- **Kitap**: Hangi kitaptan geldiği (Ay/Yıl ile)
- **Açıklama**: Tam tanım metni
- **Birim**: m³, m², Ton, Kg, Ad, m
- **Birim Fiyat**: TL cinsinden (formül içeren pozlar için uyarı)
- **Kategori**: Pozun ait olduğu iş kategorisi

> ⚠️ Fiyatı `---` (formül) olarak görünen pozlar metraja eklenemez. Bunlar derinlik zammı gibi formül içeren özel pozlardır.

**Hover (üzerine gelme) Bilgisi:**
Arama sonuçlarında pozun üzerine fare ile geldiğinizde tam açıklama metni tooltip olarak görünür.

---

### 5. Metraj Yönetimi

Sağ paneldeki butonlar:

| Buton | İşlev |
|-------|-------|
| 📂 **Aç (.mrj)** | Kayıtlı metraj dosyasını açar |
| 💾 **Kaydet (Ctrl+S)** | Mevcut dosyaya kaydeder veya ilk kayıtta "Farklı Kaydet" olarak çalışır |
| ● | Sarı nokta - kaydedilmemiş değişiklik göstergesi |
| 📊 **Excel** | Metrajı Excel (.xlsx) dosyasına aktarır |
| 🗑 **Temizle** | Tüm metraj kalemlerini siler |

**Metraj Adı:** Tablonun üst kısmındaki metraj adını değiştirebilirsiniz. Bu isim kaydetme ve Excel aktarma sırasında dosya adı olarak kullanılır.

**Kaydedilmemiş Değişiklik Takibi:**
- Kalem ekleme, silme, miktar değiştirme veya metraj adı değişikliğinde sarı ● işareti çıkar
- Kaydettikten sonra ● kaybolur
- Durum çubuğunda ve butonların yanında görünür

---

### 6. Dışa Aktarma

#### Excel (.xlsx)

Excel dosyası şu formatta oluşturulur:

| Sıra No | Poz No | Açıklama | Birim | Birim Fiyat (TL) | Miktar | Tutar (TL) |
|---------|--------|----------|-------|------------------|--------|------------|
| 1 | 15.100.1001 | ... | Ton | 280,21 | 10,00 | 2.802,10 |
| | | | | | **GENEL TOPLAM** | **X.XXX,XX** |

- Başlık ve sütunlar renklendirilmiş ve formatlanmıştır
- Sayısal değerler binlik ayraçlı ve 2 ondalıklıdır
- Genel toplam yeşil zeminli, beyaz yazılıdır

#### Proje Kaydetme (.mrj)

Metrajınızı `.mrj` uzantılı JSON formatında kaydedebilirsiniz. Dosya içeriği:
```json
{
  "ad": "Ornek Metraj",
  "tarih": "2026-05-14",
  "kalemler": [
    {
      "poz_no": "15.100.1001",
      "tanim": "1 ton her cins cimento...",
      "birim": "Ton",
      "birim_fiyat": 280.21,
      "miktar": 10.0,
      "tutar": 2802.10,
      "kitap_adi": "Cevre ve Sehircilik"
    }
  ]
}
```

> 💡 `.mrj` dosyaları aslında JSON formatındadır. Eski `.json` dosyalarınızı da açabilirsiniz.

---

## Kısayollar

| Kısayol | İşlev |
|---------|-------|
| **Ctrl+S** | Metrajı kaydet (mevcut dosyaya veya farklı kaydet) |
| **Ctrl+O** | Metraj dosyası aç |

---

## Proje Yapısı

```
metrajmatik/
├── Cargo.toml              # Rust bağımlılıkları
├── doc.md                  # Kullanım kılavuzu (bu dosya)
├── metrajmatik_veriler.db  # SQLite veritabanı (otomatik oluşur)
├── src/
│   ├── main.rs             # Uygulama giriş noktası (1400x800 pencere)
│   ├── app/                # UI katmanı (sorumluluğa göre bölünmüş)
│   │   ├── mod.rs          # Durum (MetrajApp) + update() akışı, menü/durum çubuğu
│   │   ├── islemler.rs     # UI-dışı mantık: arama, dosya, geri-al/yinele, fiyat güncelleme
│   │   ├── gorunum_metraj.rs  # Metraj sekmesi çizimleri + miktar popup + ağaç çizimi
│   │   └── gorunum_diger.rs   # Kitap / İcmal / Pozlar / PDF çizimleri
│   ├── models.rs           # Veri modelleri (Poz, Kitap, MetrajKalemi, IsGrubu)
│   ├── bicim.rs            # Biçim/ayrıştırma: para, tarih, metin, sayı (tek kaynak)
│   ├── maliyet.rs          # Yaklaşık maliyet özeti hesabı (tek kaynak)
│   ├── is_grubu.rs         # İş grubu ağaç işlemleri (saf, egui'siz)
│   ├── tema.rs             # Tema ve yeniden kullanılabilir bileşen yardımcıları
│   ├── pdf_parser.rs       # PDF metin çıkarma ve poz ayrıştırma
│   ├── database.rs         # SQLite + FTS5 veritabanı (kitaplar, pozlar)
│   └── export.rs           # Excel (.xlsx) ve JSON dışa/içe aktarma
└── target/release/
    └── metrajmatik.exe     # Tek .exe, kurulum gerektirmez
```

---

## Teknik Detaylar

### Kullanılan Teknolojiler

| Teknoloji | Sürüm | Amaç |
|-----------|-------|------|
| **Rust** | 1.85+ | Programlama dili |
| **egui/eframe** | 0.31 | Modern anlık GUI framework |
| **rusqlite** | 0.33 | Gömülü SQLite veritabanı |
| **pdf-extract** | 0.7 | PDF metin çıkarma |
| **regex** | 1 | Poz/fiyat ayrıştırma (regex) |
| **rust_xlsxwriter** | 0.83 | Excel dosyası oluşturma |
| **serde_json** | 1 | JSON serileştirme (.mrj dosyaları) |
| **rfd** | 0.15 | Dosya seçim diyalogları |

### Veritabanı Şeması

```sql
-- Kitap tablosu
CREATE TABLE kitaplar (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ad TEXT NOT NULL,
    yil INTEGER NOT NULL DEFAULT 2026,
    ay INTEGER NOT NULL DEFAULT 1,
    tarih TEXT NOT NULL DEFAULT ''
);

-- Poz tablosu
CREATE TABLE pozlar (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    poz_no TEXT NOT NULL,
    tanim TEXT NOT NULL,
    birim TEXT NOT NULL,
    fiyat REAL,
    kategori TEXT NOT NULL,
    kitap_id INTEGER NOT NULL,
    kitap_adi TEXT NOT NULL DEFAULT '',
    yil INTEGER NOT NULL DEFAULT 2026,
    ay INTEGER NOT NULL DEFAULT 1,
    UNIQUE(poz_no, kitap_id, yil, ay),
    FOREIGN KEY(kitap_id) REFERENCES kitaplar(id) ON DELETE CASCADE
);

-- Tam metin arama için FTS5 sanal tablosu
CREATE VIRTUAL TABLE pozlar_fts USING fts5(
    poz_no, tanim, birim, kategori, kitap_adi,
    content='pozlar', content_rowid='id'
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
8. Tüm pozlara `kitap_id`, `kitap_adi`, `yil`, `ay` bilgileri eklenir

---

## Sık Sorulan Sorular

### PDF yüklenirken hata alıyorum
PDF'in Çevre ve Şehircilik Bakanlığı formatında olduğundan emin olun. Farklı formatlardaki PDF'ler için `pdf_parser.rs` dosyasındaki regex desenleri güncellenmelidir.

### Kitap ekle butonu çalışmıyor
Kitap adı alanının boş olmadığından emin olun. Ayrıca programın yazma izni olduğundan ve `metrajmatik_veriler.db` dosyasının bulunduğu klasörde disk alanı olduğundan emin olun.

### Aynı pozu farklı kitaplarda kullanabilir miyim?
Evet. `UNIQUE(poz_no, kitap_id, yil, ay)` kısıtı sayesinde aynı poz numarası farklı kitaplarda veya farklı ay/yıl dönemlerinde bulunabilir.

### Metrajda hangi kitaptan poz eklediğimi nasıl anlarım?
Metraj tablosunda her kalemin **Kitap** sütunu vardır. Ayrıca arama sonuçlarında ve seçili poz detayında kitap bilgisi gösterilir.

### Excel aktarımında Türkçe karakter sorunu
Uygulama çıktıları UTF-8 kodlamalıdır. Excel'de açarken UTF-8 olarak içe aktarın.

### Eski .json dosyalarımı açabilir miyim?
Evet. **📂 Aç (.mrj)** butonu hem `.mrj` hem de `.json` uzantılı dosyaları açar.

---

## Lisans

Bu proje özel kullanım için geliştirilmiştir.

---

*Metrajmatik v0.2.0 - Mayıs 2026*