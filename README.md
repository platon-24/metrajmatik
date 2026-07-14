<div align="center">

# 🏗️ Metrajmatik

### Modern, hızlı ve mevzuata uygun **yaklaşık maliyet · metraj · hakediş** programı

![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white)
![egui](https://img.shields.io/badge/egui-0.31-1f6feb)
![SQLite](https://img.shields.io/badge/SQLite-FTS5-003B57?logo=sqlite&logoColor=white)
![Testler](https://img.shields.io/badge/testler-46%20passing-2ea043)
![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows&logoColor=white)
![Durum](https://img.shields.io/badge/durum-aktif%20geliştirme-f5a623)

*Türk kamu ihale mevzuatına birebir uyumlu, tek dosyada çalışan masaüstü keşif/metraj/hakediş platformu.*

`📁 Proje` · `📋 Metraj` · `📊 İcmal` · `🧾 Hakediş` · `📅 İş Programı` · `🔎 Pozlar` · `📚 Kitaplar` · `📄 PDF Yükle`

</div>

---

## ✨ Nedir?

**Metrajmatik**, yapım işlerinde bir projenin **yaklaşık maliyetinden** başlayıp **metraj**, **birim fiyat analizi**, **keşif icmali**, **hakediş** ve **iş programına** kadar tüm yaşam döngüsünü tek uygulamada yürütür. Rust + [egui](https://github.com/emilk/egui) ile yazılmış, harici bağımlılık gerektirmeyen (SQLite gömülü) **tek yürütülebilir dosya**dır.

> **Tasarım ilkesi:** *Metraj bir kez girilir* — icmal, teklif cetveli, hakediş ve pursantaj hep aynı tek kaynağı yeniden kullanır.

---

## 🎯 Öne çıkanlar

- **📐 Doğru mevzuat.** Genel gider + müteahhit kârı (%25) **yalnızca analiz/rayiç ile fiyatlandırılan** özel pozlara uygulanır — kurum birim fiyatları bunu zaten içerdiği için çifte sayılmaz. Kamu yaklaşık maliyeti **KDV hariç** hesaplanır.
- **🗂️ Kurum/dönem veri modeli.** Kitap = *kurum*, poz = *kimlik*, fiyat = *(yıl/ay) indeksli*. Arama hep **en son dönem** fiyatını verir; eski dönemler korunur.
- **📄 6 kurum PDF profili.** ÇŞB · KGM · DSİ · Vakıflar · PTT · Altyapı birim fiyat kitaplarını otomatik tanıyıp ayrıştırır (~10.000 poz doğrulandı).
- **📊 Resmî Excel çıktıları.** Yaklaşık Maliyet Hesap Cetveli (GİZLİ damgası + imza blokları), Metraj Cetveli, Pursantaj, Analiz Föyleri, Hakediş Raporu, İş Programı — proje künyesi başlıklara akar.
- **🎨 Koyu, sade arayüz.** Otomatik kayıt + kurtarma, sınırsız geri al/yinele, canlı toplamlar.

---

## 🔄 Uçtan uca akış

| # | Adım | Kapsam | Durum |
|---|------|--------|:---:|
| 1 | **Proje Kur** | İdare adı, işin adı, İKN, iş yeri, sözleşme, Kamu/Özel kipi | ✅ |
| 2 | **Kitap / Veri** | Kurum + dönem fiyat kitapları, `.mvp` veri paketi içe/dışa | ✅ |
| 3 | **İş Ağacı** | Hiyerarşik iş grupları (İnşaat / Mekanik / Elektrik…) | ✅ |
| 4 | **Metraj** | Boyutlu miktar (adet/en/boy/yük + çıkan), imalat cinsi, otomatik icmal | ✅ |
| 5 | **Fiyatlandır** | Kurum fiyatı doğrudan; değilse **birim fiyat analizi** (rayiçlerden) | ✅ |
| 6 | **İcmal** | İş grubu bazlı, Kamu (KDV hariç) / Özel (KDV dahil) | ✅ |
| 7 | **Güncelle** | Rayiçleri ihale tarihine/döneme toplu güncelleme | ⚠️ Kısmi |
| 8 | **Çıktı** | Resmî Excel dossier + CSV | ✅ |
| 9 | **İhale** | Birim fiyat teklif cetveli + teklif mektubu | 🔜 Sırada |
| 10 | **Hakediş** | Yeşil defter → hakediş → fiyat farkı → kesin hesap | ✅ |

---

## 🧾 Modüller ne yapar?

**Hakediş** — Çoklu ara/kesin hakediş, önceki kümülatifi devralma, **yeşil defter** ölçü kırılımı, kesintiler (damga ‰9,48 · teminat · SGK · avans mahsubu), **fiyat farkı (Yİ-ÜFE)**, **KDV tevkifatı**, **kesin hesap** (sözleşme vs gerçekleşen) ve resmî Excel raporu.

**İş Programı** — Sözleşme bedelini süre boyunca aylara pursantaj olarak dağıtır; aylık + kümülatif tablo, **ilerleme (S) eğrisi grafiği** ve Excel pursantaj cetveli.

**Analiz** — Rayiçlerden birim fiyat analizi (işçilik + malzeme + nakliye), kaynak izlenir, %25 yalnız analize uygulanır.

**Veri paketi & yedek** — Kurum fiyat kitaplarını `.mvp` paketi olarak paylaşın; tüm veritabanını tek dosyaya **yedekleyip** kendi bulut klasörünüzde (OneDrive/Drive) saklayın.

---

## 🚀 Kurulum & çalıştırma

Gereksinim: [Rust](https://rustup.rs) (stable). SQLite **gömülüdür** — ayrıca kurmanıza gerek yok.

```bash
# Depoyu klonlayın
git clone <repo-url>
cd metrajmatik

# Geliştirme modunda çalıştır
cargo run

# Optimize edilmiş sürüm derle
cargo run --release
```

Veriler `%APPDATA%\Metrajmatik\` altında tutulur (fiyat kitabı veritabanı + otomatik kayıt). Projeler `.mrj` dosyalarında saklanır.

---

## 🛠️ Teknoloji yığını

| Katman | Araç |
|--------|------|
| Dil | **Rust** (2021) |
| Arayüz | **eframe / egui** 0.31 (immediate-mode GUI) |
| Veritabanı | **rusqlite** 0.33 (SQLite + FTS5, `bundled`) |
| PDF | pdf-extract + regex tabanlı ayrıştırma profilleri |
| Excel | rust_xlsxwriter 0.83 |
| Serileştirme | serde / serde_json (`.mrj`, `.mvp`) |
| Dosya diyalogları | rfd 0.15 |

---

## 📁 Proje yapısı

```
src/
├── main.rs               Giriş noktası + pencere kurulumu
├── models.rs             Veri modelleri (Metraj, Hakediş, İş Programı, Proje künyesi…)
├── database.rs           SQLite v2 şema + v1→v2 göç + FTS5 arama
├── export.rs             Excel / CSV / veri paketi çıktıları
├── pdf_parser.rs         6 kurum için PDF ayrıştırma profilleri
├── hakedis.rs            Hakediş hesap motoru (kümülatif → kesinti → net)
├── maliyet.rs            Yaklaşık maliyet özeti (kâr + KDV mantığı)
├── is_grubu.rs           Hiyerarşik iş grupları
├── bicim.rs              Biçimlendirme + kuruş yuvarlama yardımcıları
├── tema.rs               Koyu tema + UI bileşenleri
└── app/
    ├── mod.rs            Uygulama durumu (MetrajApp) + sekme dağıtımı
    ├── proje_ui.rs       📁 Proje künyesi sekmesi
    ├── gorunum_metraj.rs 📋 Metraj sekmesi (arama, iş grupları, miktar popup)
    ├── gorunum_diger.rs  📊 İcmal · 🔎 Pozlar · 📚 Kitaplar · 📄 PDF
    ├── analiz_ui.rs      Birim fiyat analizi popup'ı
    ├── hakedis_ui.rs     🧾 Hakediş sekmesi + yeşil defter
    ├── is_programi_ui.rs 📅 İş programı sekmesi + S-eğrisi
    └── islemler.rs       Dosya / arama / geri-al / yedek iş mantığı
```

---

## ✅ Testler

```bash
cargo test
```

**46 test** çekirdek mantığı doğrular: kâr/KDV hesabı, kurum/dönem fiyat çözümü, v1→v2 göç, hakediş icmali, iş programı dağılımı, veri paketi round-trip, yedekleme ve Excel üretimi. *(1 test — gerçek kurum PDF'leriyle doğrulama — `#[ignore]`; yerel örnek dosyalar gerektirir.)*

---

## 🗺️ Yol haritası

- [x] **P0** — Mevzuata doğru MVP (icmal doğruluğu, analiz, resmî Excel, kurum/dönem modeli)
- [x] **P1** — Piyasa paritesi (6 kurum PDF profili, pursantaj, CSV, nakliye, fiyat araştırması)
- [x] **P2** — Yaşam döngüsü (hakediş, fiyat farkı, KDV tevkifatı, kesin hesap, iş programı, veri paketi, yedek)
- [x] **Proje künyesi** — resmî çıktı başlıkları (idare / iş adı / İKN)
- [ ] **İhale tarafı** — birim fiyat teklif cetveli + teklif mektubu (sırada)
- [ ] Dönem/endeks toplu güncelleme cilası

Ayrıntılı strateji ve mevzuat notları için: [`YAKLASIK_MALIYET_RAPORU.md`](YAKLASIK_MALIYET_RAPORU.md)

---

## 📜 Lisans

Özel / ticari proje — **tüm hakları saklıdır.** (Lisans koşulları netleştirilecek.)

<div align="center">
<sub>Türk inşaat sektörü için ❤️ ve 🦀 ile yapıldı.</sub>
</div>
