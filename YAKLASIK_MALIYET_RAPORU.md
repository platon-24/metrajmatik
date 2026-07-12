# 🏗 Metrajmatik — Tam Yaklaşık Maliyet Yol Haritası ve Rekabet Raporu

**Amaç:** Metrajmatik'i "çalışan bir metraj aracı"ndan, **OSKA ve AMP'yi piyasadan silecek** eksiksiz bir yaklaşık maliyet / keşif / hakediş platformuna dönüştürmek.
**Tarih:** 12 Temmuz 2026
**Kapsam:** Mevcut kod denetimi · mevzuat gereksinimleri · alan modeli · özellik yol haritası · workflow · UI/UX · kullanım kolaylığı · sağlamlık · veri stratejisi.

> Bu rapor mevcut kaynak kodun tamamı okunarak yazılmıştır (`src/app.rs`, `models.rs`, `database.rs`, `pdf_parser.rs`, `export.rs`, `tema.rs`). Somut kod referansları `dosya:satır` biçiminde verilmiştir.

---

## 0. Yönetici Özeti

Metrajmatik bugün **sağlam bir çekirdeğe** sahip: çoklu kurum kitabı, dönem (yıl/ay) takibi, FTS5 hızlı arama, hiyerarşik iş grupları, boyutlu miktar (en×boy×yükseklik×adet), 50 adım geri-al, 30 sn otomatik kayıt ve formatlı Excel çıktısı. Bu, birçok "excel + el emeği" iş akışını şimdiden geçecek bir temeldir.

Ancak bir **yaklaşık maliyet programı** olarak henüz **tamamlanmış değil**. OSKA/AMP'yi tehdit etmek için üç şey zorunlu:

1. **Doğruluk (mevzuata uygunluk).** Bugün İcmal iki yerde matematiksel/mevzuat hatası içeriyor: (a) kurum birim fiyatları zaten kâr+genel gider dahil olduğu halde tüm ara toplama **tekrar %25** ekleniyor (çifte sayım), (b) kamu yaklaşık maliyeti **KDV hariç** hesaplanması gerekirken KDV ekleniyor. Bunlar bir Sayıştay denetiminde ilk yakalanacak şeylerdir. → **Bölüm 1 & 3.**
2. **Analiz (birim fiyat analizi).** OSKA/AMP'nin asıl işi budur; Metrajmatik'te **hiç yok**. Rayiç → girdi → analiz → poz fiyatı zinciri kurulmadan "özel/yeni fiyat" üretilemez, bu da programı yalnızca "hazır fiyatlı poz seçici" seviyesinde tutar. → **Bölüm 4 & 5.**
3. **Veri (gerçek hendek).** OSKA'nın asıl gücü yazılım değil, **güncel poz/rayiç/analiz kitaplarını** hazır paket olarak satmasıdır. PDF ayrıştırma zekice ama tek formata bağlı ve kırılgan. Sürdürülebilir bir **veri hattı** olmadan pazar alınamaz. → **Bölüm 10.**

**Tez:** OSKA/AMP eski, pahalı, masaüstü-kilitli ve öğrenmesi zor. Metrajmatik'in kazanma yolu → **mevzuata birebir doğru + öğrenmesi 10 dakika + güncel veriyi tek tıkla kuran + fiyatın yarısı** bir ürün olmaktır. Aşağıdaki yol haritası bunu 3 fazda kuruyor.

---

## 1. Mevcut Durum — Dürüst Kod Denetimi

### 1.1 Güçlü yönler (korunmalı)

| Alan | Durum | Not |
|---|---|---|
| Mimari | ✅ Temiz | Modüller ayrık (`models`/`database`/`pdf_parser`/`export`/`app`/`tema`), tek `.exe`, bağımlılıksız |
| Arama | ✅ Çok iyi | SQLite **FTS5** + poz-no LIKE + "akıllı arama" birleşimi (`app.rs:1495`) hızlı ve doğru |
| Veri modeli çekirdeği | ✅ İyi | Hiyerarşik `IsGrubu` ağacı, `MiktarDetay` boyut kırılımı, geriye uyumlu JSON (`models.rs`) |
| Geri al / yinele | ✅ | 50 adımlık anlık görüntü yığını (`app.rs:1397`) |
| Otomatik kayıt + kurtarma | ✅ | 30 sn'de bir `.mrj` (`app.rs:1436`), açılışta kurtarma şeridi |
| Tema/UX tutarlılığı | ✅ | Tek tasarım dili (`tema.rs`), profesyonel koyu tema |
| Excel çıktısı | ✅ | Gruplu, alt toplamlı, formatlı (`export.rs`) |
| Test kültürü | ✅ başlangıç | `models.rs` ve `pdf_parser.rs` içinde birim testleri var |

### 1.2 Kritik doğruluk hataları (ACİL — mevzuat)

> Bunlar "eksik özellik" değil, **yanlış sonuç** üreten kusurlardır. Bir yaklaşık maliyet programında yanlış rakam = itibar kaybı.

**H1 — Kâr + genel giderde çifte sayım.**
`app.rs:1015` ve `export.rs:236` tüm ara toplama **%25 kâr+genel gider** ekliyor. Ancak kurumların (ÇŞB, KGM, DSİ…) yayımladığı **birim fiyatlar zaten kâr ve genel gider dahildir.** Bu fiyatlar üzerine tekrar %25 eklemek maliyeti şişirir.
→ Doğru kural: %25 **yalnızca analizle bulunan** (rayiç bazlı) özel/yeni fiyatlara uygulanır; hazır kurum birim fiyatlarına uygulanmaz. Poz başına "kâr dahil mi?" bayrağı gerekli.

**H2 — KDV, kamu yaklaşık maliyetine ekleniyor.**
`app.rs:1017` ve `export.rs:238` KDV'yi genel toplama dahil ediyor. Kamu İhale mevzuatında **yaklaşık maliyet KDV hariç** hesaplanır. → "Kamu / Özel" kip anahtarı: Kamu kipinde KDV = 0 ve etiket "KDV Hariç Yaklaşık Maliyet".

**H3 — Naïve tarih fonksiyonu.**
`database.rs:330` ve `app.rs:1912` tarihi `gün/365`, `gün%365/30` ile hesaplıyor — artık yıl yok, ay = 30 gün. Üretilen tarih (kitap tarihi, `.mrj` tarihi, Excel başlığı) **yanlış**. Resmî bir çıktıda kabul edilemez. → `time`/`chrono` benzeri doğru bir takvim, ya da en azından doğru Gregoryen hesap.

### 1.3 Yapısal eksikler (rekabet için zorunlu)

| # | Eksik | Etki | Ref |
|---|---|---|---|
| E1 | **Birim fiyat analizi yok** | Özel/yeni fiyat üretilemez; kurum fiyatına bağımlı | tüm proje |
| E2 | **Rayiç (girdi) kitabı kavramı yok** | İşçilik/malzeme/makine girdileri tutulamaz | `models.rs` |
| E3 | **Tek PDF formatına bağımlılık** | Poz regex `\d{2}\.\d{3}\.\d{4}` sadece ÇŞB yeni format; KGM/DSİ/Kültür pozları okunmaz | `pdf_parser.rs:47` |
| E4 | **Kategori/başlık ÇŞB'ye gömülü** | `"2026 MAYIS"` gibi sabit filtreler, elle yazılmış anahtar listesi | `pdf_parser.rs:12,64` |
| E5 | **Metraj = poz'a kilitli** | Poz'suz metraj, formüllü/çıkan (negatif) miktar, "benzer" adedi yok | `models.rs:42` |
| E6 | **Miktar hep 4 boyut çarpımı** | Alan (en×boy) vs hacim ayrımı yok; parametre boşsa 1 sayılıp çarpılıyor | `models.rs:42-51` |
| E7 | **Para birimi `f64`** | Kuruş yuvarlama hataları büyük keşiflerde birikir; denetimde tutmaz | `models.rs` |
| E8 | **DB ve autosave çalışma dizinine yazılıyor** | `PathBuf::from("metrajmatik_veriler.db")` — farklı klasörden açılınca farklı/boş DB, veri kaybı riski | `app.rs:105,119` |
| E9 | **Resmî form çıktıları yok** | Sadece serbest formatlı Excel; KİK hesap cetveli/metraj cetveli/analiz föyü yok | `export.rs` |
| E10 | **PDF çıktı yok** | Piyasa imzalı/mühürlü PDF bekler | — |
| E11 | **Nakliye (taşıma) analizi yok** | Mesafe bazlı taşıma bedeli hesabı OSKA'da standart | — |
| E12 | **Pursantaj / iş programı / hakediş yok** | Yaşam döngüsünün geri kalanı | — |

---

## 2. Rakip Analizi — OSKA & AMP

### 2.1 Ne yapıyorlar (kapsam)
OSKA ve AMP sadece yaklaşık maliyet değil, **tüm yapım işi yaşam döngüsünü** kapsar:

- Yaklaşık maliyet + **birim fiyat analizleri** + **rayiç** yönetimi
- **Metraj cetvelleri** (benzer/formül/çıkan) + metraj icmali
- **Nakliye/taşıma** analizleri (mesafe, formül)
- **Pursantaj tablosu** ve **iş programı**
- **Hakediş** (yeşil defter, ataşman, ara/kesin hakediş, kesintiler, tahakkuk)
- **Fiyat farkı** (TÜİK Yİ-ÜFE endeksleri, sabit ağırlık oranları)
- **Kesin hesap**, geçici/kesin kabul
- Kurum kitaplarının **hazır, güncel veri paketi** olarak satışı (asıl gelir)

### 2.2 Gerçek hendekleri
1. **Veri.** Her yıl/dönem tüm kurumların poz + analiz + rayiç kitaplarını hazır, doğrulanmış paket olarak sunmaları. Yazılım kopyalanır; **güncel ve doğru veri** kopyalanamaz.
2. **Alışkanlık.** On yıllardır bu ekranlara alışmış teknik personel; muscle-memory.
3. **Mevzuat güveni.** Çıktıların denetimde "tuttuğu" bilinir.

### 2.3 Zayıf noktaları = bizim fırsatımız

| OSKA/AMP zayıflığı | Metrajmatik fırsatı |
|---|---|
| Eski, kalabalık, öğrenmesi zor arayüz | Modern, sade, 10 dakikada öğrenilen UX (zaten yolda) |
| Pahalı lisans + yıllık veri ücreti | Yarı fiyat / şeffaf abonelik |
| Masaüstü-kilitli, dosya paylaşımı zor | Bulut yedek + paylaşım + çok kullanıcı |
| Excel ile zayıf gidiş-geliş | Excel'i birinci sınıf içe/dışa aktarım yap |
| Kapalı format | Açık `.mrj` (JSON) — dışa bağımlılık yok |
| Kurulum/donanım kilidi/dongle | Tek `.exe`, taşınabilir (zaten var) |

**Sonuç:** Onları yazılımda değil **kullanım kolaylığı + fiyat + veri tazeliği + bulut** ekseninde yenebiliriz. Ama önce **doğruluk ve analiz** paritesini yakalamak şart.

---

## 3. Mevzuat Çerçevesi (Yönetmelikler)

> Aşağıdaki çerçeve geneldir; **madde numaraları güncel Resmî Gazete metniyle doğrulanmalıdır** (yönetmelikler sık değişir). Program içine "mevzuat sürümü" alanı koyup çıktının hangi tarihli mevzuata göre üretildiğini yazmak denetim açısından değerlidir.

### 3.1 Ana kaynaklar
- **4734 sayılı Kamu İhale Kanunu** — yaklaşık maliyet ilkesi (idarece, ihaleden önce, gizli).
- **Yapım İşleri İhaleleri Uygulama Yönetmeliği** — yaklaşık maliyetin ayrıntılı hesap kuralları (metraj, rayiç sıralaması, analiz, kâr+genel gider, belgeler).
- **Yapım İşleri Genel Şartnamesi (YİGŞ)** — hakediş, metraj, fiyat farkı uygulaması.
- **Fiyat Farkı Kararnamesi** (yapım işleri fiyat farkı esasları) — Yİ-ÜFE endeksleri.
- Kurum tebliğleri: ÇŞB birim fiyat/rayiç/analiz tebliğleri, KGM, DSİ, İLBANK, Kültür/Vakıflar (restorasyon) vb.

### 3.2 Yaklaşık maliyetin olmazsa-olmaz ilkeleri (programa yansıması)

1. **Miktar = metraj.** Her poz için metraj cetveline dayalı miktar. → Metraj cetveli modeli (Bölüm 4).
2. **Fiyat/rayiç sıralaması (öncelik):**
   1) İdarenin belirlediği fiyat → 2) Kamu kurumu birim fiyatları (ÇŞB/KGM/DSİ…) → 3) Piyasa rayici (analiz + fiyat araştırması).
   → Programda her poz fiyatının **kaynağı** izlenebilmeli (hangi kitap/dönem ya da "analiz" / "piyasa").
3. **Analiz.** Yayımlanmış fiyatı olmayan iş kalemleri için işçilik+malzeme+makine girdilerinden **birim fiyat analizi** yapılır. → Bölüm 4.
4. **Kâr + genel gider %25.** **Yalnızca analizle** bulunan fiyatlara eklenir; kurum birim fiyatları bunu zaten içerir (H1).
5. **KDV hariç.** Yaklaşık maliyet KDV'siz hesaplanır (H2).
6. **Gizlilik.** Yaklaşık maliyet isteklilere açıklanmaz; onay belgesine eklenir. → Program "gizli/onay" damgalı çıktı üretebilmeli.
7. **Güncelleme.** Eski tarihli rayiçler, ihale tarihine güncellenebilir (endeks/oran). → Toplu fiyat güncelleme zaten var (`app.rs:1695`), buna "endeksle güncelle" eklenebilir.

### 3.3 Zorunlu belgeler (çıktı seti)
Tam bir yaklaşık maliyet dosyası şunları içerir — **hepsi program çıktısı olmalı:**

1. **Yaklaşık Maliyet Hesap Cetveli** (icmal)
2. **Metraj Cetvelleri** (her poz için, boyut kırılımlı)
3. **Metraj İcmali** (poz bazında toplam miktarlar)
4. **Birim Fiyat Analizleri** (analiz föyleri)
5. **Rayiç / Mahal listesi**
6. **Kâr+genel gider ve (özel sektörde) KDV gösterimi**
7. **Pursantaj tablosu** (istenen işlerde)

Bugün sadece #1 ve kısmen #2/#3 tek bir Excel'de var. Diğerleri eksik (E9).

---

## 4. Tam Bir Yaklaşık Maliyetin Alan Modeli

OSKA/AMP paritesi için veri modeli **poz-merkezli**den **rayiç→analiz→poz→metraj** zincirine genişlemeli.

### 4.1 Kavram katmanları

```
RAYİÇ (girdi)         ANALİZ                     POZ (iş kalemi)          METRAJ
─────────────         ──────                     ───────────────          ──────
10.100.1001 Düz işçi  15.150.1001 analizi:       15.150.1001              Poz + boyutlar
  saat · 250 TL   →     • 2.5 sa düz işçi     →   "Beton dökülmesi"   →     (benzer/formül)
10.130.... çimento      • 0.3 m³ çimento          birim: m³                 → miktar
04.xxx makine           • 0.1 sa vibratör         fiyat: analizden          → tutar
                        + %25 kâr/genel gider     VEYA kurum kitabından
                        = birim fiyat
```

- **Rayiç kitabı:** girdi no, ad, birim, fiyat, dönem. (Şema olarak `pozlar` tablosuna çok benzer — `tur` alanı eklenerek ayrılabilir: `rayic | poz | analiz_basligi`.)
- **Analiz:** başlık poz + girdi satırları `(girdi_no | alt_poz_no, miktar/katsayı, birim, tutar)`, montaj/nakliye ayrımı, %25 satırı, sonuç birim fiyat.
- **Poz fiyat kaynağı:** `Doğrudan(kurum kitabı, kâr dahil)` **veya** `Analiz(kâr eklenecek)` — bu ayrım H1'i çözer.
- **Metraj satırı:** poz + `benzer_adedi` + boyutlar + **işaret (+/−)** + serbest formül; poz'suz (yalnız ölçü) satıra da izin.

### 4.2 Önerilen şema genişlemesi (mevcut SQLite üzerine)

```sql
-- pozlar tablosuna:
ALTER TABLE pozlar ADD COLUMN tur TEXT NOT NULL DEFAULT 'poz';      -- 'poz' | 'rayic' | 'analiz'
ALTER TABLE pozlar ADD COLUMN kar_dahil INTEGER NOT NULL DEFAULT 1; -- kurum fiyatı=1, analiz sonucu=0

-- analiz girdileri:
CREATE TABLE analiz_girdileri (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  poz_no TEXT NOT NULL,          -- analizi yapılan iş kalemi
  kitap_id INTEGER NOT NULL,
  girdi_no TEXT NOT NULL,        -- rayiç ya da alt-poz numarası
  girdi_tur TEXT NOT NULL,       -- 'iscilik'|'malzeme'|'makine'|'nakliye'|'alt_poz'
  miktar REAL NOT NULL,          -- katsayı
  birim TEXT NOT NULL,
  FOREIGN KEY(kitap_id) REFERENCES kitaplar(id) ON DELETE CASCADE
);

-- metraj satırında işaret ve benzer:
-- MiktarDetay { ..., isaret: +1/-1, benzer: Option<f64>, formul: Option<String> }
```

`MiktarDetay::hesaplanan_miktar()` (`models.rs:42`) buna göre: `işaret × benzer × (boş olmayan boyutların çarpımı)` — **boş boyut 1 değil, "çarpmaya dahil değil"** olmalı (E6). Alan/hacim ayrımı formülle netleşir.

### 4.3 "İmalat cinsi" ayrımı
Gerçek metrajda satırın **imalatın cinsi** açıklaması, poz açıklamasından ayrıdır (ör. "1. kat perde duvarları"). `MetrajKalemi`e `imalat_cinsi: String` alanı eklenmeli; Excel/PDF metraj cetvelinde bu sütun beklenir.

---

## 5. Özellik Yol Haritası (Öncelikli)

### P0 — "Yaklaşık maliyeti TAMAMLA" (mevzuata doğru MVP)
> Bunlar bitmeden ürün "yaklaşık maliyet programı" sayılmaz.

- [ ] **H1 düzelt:** poz bazında `kar_dahil` bayrağı; %25 sadece analiz fiyatlarına.
- [ ] **H2 düzelt:** "Kamu/Özel" kipi; kamuda KDV hariç.
- [ ] **H3 düzelt:** doğru tarih.
- [ ] **Birim fiyat analizi (E1/E2):** rayiç kitabı + analiz föyü + analizden poz fiyatı üretimi.
- [ ] **Metraj cetveli olgunlaşması (E5/E6):** benzer adedi, +/− çıkan, formül, imalat cinsi.
- [ ] **Resmî çıktılar (E9/E10):** Yaklaşık Maliyet Hesap Cetveli + Metraj Cetveli + Metraj İcmali + Analiz Föyü → Excel **ve** PDF; "Gizli / Onaya esas" damgası.
- [ ] **Veri konumu (E8):** DB/autosave `%APPDATA%\Metrajmatik\` altına; taşınırlık için "veri klasörü" ayarı.

### P1 — Piyasa paritesi (OSKA/AMP ile aynı masada)
- [ ] **Çok formatlı PDF/Excel içe aktarma (E3/E4):** kurum bazlı ayrıştırma profilleri (ÇŞB, KGM, DSİ, İLBANK, Kültür/Vakıflar). Regex'ler profil dosyasına taşınır.
- [ ] **Nakliye/taşıma analizi (E11):** mesafe + formül bazlı taşıma bedeli.
- [ ] **Para birimi doğruluğu (E7):** kuruş bazlı tamsayı ya da sabit ondalık; yuvarlama kuralı tek yerde.
- [ ] **Pursantaj tablosu:** iş grubu/poz ağırlıkları (%).
- [ ] **Excel gidiş-geliş:** metrajı Excel'den içe aktar (şablonla).
- [ ] **Fiyat araştırması / piyasa rayici** kaydı (3 teklif ortalaması vb.).

### P2 — "OSKA/AMP'yi göm" (yaşam döngüsü + bulut)
- [ ] **Hakediş:** yeşil defter, ataşman, ara/kesin hakediş, kesintiler (SGK, damga, teminat, avans mahsubu), tahakkuk.
- [ ] **Fiyat farkı:** Yİ-ÜFE endeksleri, temel/güncel endeks, sabit ağırlık oranları (a, b₁…bₙ).
- [ ] **Kesin hesap**, geçici/kesin kabul.
- [ ] **İş programı / zaman planı** (bar-chart).
- [ ] **Bulut yedek + paylaşım + çok kullanıcı** (asıl farklılaşma).
- [ ] **Güncel veri paketleri** (Bölüm 10).

---

## 6. Uçtan Uca Workflow (hedef)

```
1. PROJE KUR        → Proje adı, işveren idare, iş adı, ihale kaydı, Kamu/Özel kipi
2. KİTAP/VERİ       → İlgili dönem poz+rayiç+analiz kitaplarını tek tıkla kur (veri paketi)
3. İŞ AĞACI         → Şablon iş grupları (İnşaat/Mekanik/Elektrik…) yükle veya özelleştir
4. METRAJ           → Her imalat için poz seç → metraj cetveli (benzer/boyut/çıkan/formül)
                      → miktar otomatik → metraj icmali otomatik
5. FİYATLANDIR      → Poz kurum fiyatlıysa doğrudan; değilse ANALİZ yap (rayiçlerden)
                      → %25 yalnız analizlere; kaynak izlenir
6. İCMAL            → İş grubu bazlı icmal, oranlar, KDV hariç yaklaşık maliyet
7. GÜNCELLE         → Rayiçleri ihale tarihine güncelle (dönem/endeks)
8. ÇIKTI            → Hesap cetveli + metrajlar + analizler + icmal → Excel & PDF (Gizli)
9. (SONRA) İHALE    → Birim fiyat teklif cetveli, teklif mektubu
10.(SONRA) HAKEDİŞ  → Yeşil defter → hakediş → fiyat farkı → kesin hesap
```

Kritik prensip: **metraj bir kez girilir**, her yerde (icmal, teklif cetveli, hakediş, pursantaj) yeniden kullanılır. Tek gerçek kaynak.

---

## 7. UI/UX Fikirleri (egui'ye uygun, somut)

### 7.1 Metraj ekranı — asıl çalışma alanı
- **Klavyeyle akış:** poz ara → Enter ile seç → miktar hücresine odak → Tab ile boyutlar → Enter sonraki satır. (Şu an çift tıklama var `app.rs:629`; klavye-öncelikli akış hızı 3×'ler.)
- **Excel-benzeri grid:** metraj tablosunda hücre içi düzenleme (miktar/boyut satır içinde), ayrı popup'a girmeden. Popup (`render_miktar_popup`) "detay/kırılım" için kalsın, hızlı giriş satır içi olsun.
- **Yapıştır (paste) desteği:** Excel'den boyut bloğu kopyalayıp metraja yapıştırma.
- **Formül çubuğu:** miktar hücresine `4*0.30*5.00-2*1.0` yazılabilsin; sonuç ve kırılım saklansın.
- **"Fiyatsız/formül" uyarı rozetleri** zaten var (`app.rs:936`) — icmalde de kırmızı bilgi kartıyla "N kalem fiyatlandırılmadı" göster, çıktıyı engelle.

### 7.2 Analiz ekranı (yeni)
- Sol: rayiç arama (mevcut poz arama bileşeni yeniden kullanılır).
- Sağ: analiz föyü grid'i — girdi | miktar | birim | tutar; altta işçilik/malzeme/makine alt toplamları + %25 + sonuç.
- "Bu analizi poz fiyatı yap" butonu.

### 7.3 Navigasyon & genel
- **Komut paleti (Ctrl+P):** "poz ekle", "analiz aç", "Excel çıktısı", "kitap değiştir"… Öğrenme eğrisini düşürür.
- **Son projeler** açılış ekranı (dosya yolunu hatırla).
- **Global arama (Ctrl+F):** proje içi kalem/poz.
- **Breadcrumb:** aktif kitap + dönem + iş grubu üstte sabit (kısmen status bar'da var `app.rs:287`).
- **Boş durum kartları** zaten iyi (`app.rs:770`) — analiz/hakediş ekranlarına da ekle.
- **Tema:** koyu tema güçlü; **açık tema** seçeneği kamu kurumu masaüstlerinde beklenir. `tema.rs` tek yerden yönettiği için eklenmesi kolay.
- **Yazdırma önizleme:** çıktı öncesi WYSIWYG sayfa önizleme (A4, başlık/altbilgi, idare logosu).

### 7.4 Görsel çıktı kalitesi
- İdare adı/logo, iş adı, ihale kayıt no, tarih, "Gizlidir" filigranı.
- Sayfa numarası, "X/Y sayfa", imza blokları (düzenleyen/kontrol eden/onaylayan).

---

## 8. Kullanım Kolaylığı (Ease of Use) — Kazandıran Eksen

1. **10 dakikada üretkenlik.** Kur → veri paketi yükle → örnek proje ile rehberli tur. İlk açılışta "Örnek proje aç" butonu.
2. **Tek tıkla güncel veri.** OSKA'nın en büyük sürtünmesi veri güncelleme. "Kitaplar" ekranına **"Güncel dönemi indir/kur"** akışı (Bölüm 10).
3. **Excel'i kucakla.** Mühendisler Excel'de yaşıyor: metrajı Excel'den içe, her çıktıyı Excel'e. `.mrj` (JSON) açık format — kilitlenme yok.
4. **Klavye-öncelikli.** Kısayollar görünür (tuş ipuçları), komut paleti.
5. **Hata yerine rehberlik.** "Fiyatsız kalem var → analiz yap" gibi eyleme dönük mesajlar (mevcut mesaj kültürü iyi, `app.rs:1556`).
6. **Sıfır kurulum.** Tek `.exe` (zaten var) — kurumsal BT onayı gerektirmez; büyük avantaj.
7. **Göç kolaylığı.** OSKA/AMP'den gelenler için: onların Excel çıktısını içe aktaran "içe aktarma sihirbazı" → müşteri kapma silahı.

---

## 9. Sağlamlık (Robustness) & Doğruluk

### 9.1 Veri güvenliği
- **DB'yi `%APPDATA%\Metrajmatik\`e taşı** (E8). Çalışma diziniyle bağı kes.
- **Otomatik yedek rotasyonu:** son N autosave'i sürümle (tek `.mrj`'in üzerine yazma riski var, `app.rs:119`).
- **Kayıt bütünlüğü:** yaz-önce-geçici-dosya + atomik `rename` (yarıda kalan yazımda bozulmayı önler).
- **SQLite WAL** açık (iyi, `database.rs:106`); düzenli `VACUUM`/bütünlük kontrolü ekle.

### 9.2 Sayısal doğruluk
- **Para = kuruş (i64) veya sabit ondalık** (E7). Tek bir `yuvarla()` ve tek bir `para_formatla()` (mevcut `app.rs:1926` iyi) kuralı.
- **Miktar yuvarlama politikası** mevzuata göre (genelde 2–3 hane) tek yerden.
- **Toplama sırası:** icmal alt toplamlar → grup → genel; her seviyede yuvarla-sonra-topla vs topla-sonra-yuvarla kararı sabitlensin (denetimde tutması için).

### 9.3 Ayrıştırma sağlamlığı
- PDF profilleri + **her profil için altın örnek testleri** (mevcut `pdf_parser.rs` test deseni genişletilir; gerçek PDF'lerle regresyon).
- İçe aktarımda **doğrulama raporu:** "1240 poz okundu, 12 satır fiyatsız, 3 birim tanınmadı" → kullanıcı görsün.

### 9.4 Test & CI
- Analiz hesabı, %25/KDV kip matrisi, metraj formül/çıkan, para yuvarlama için birim testleri.
- Altın-dosya (golden) testleri: örnek proje → beklenen Excel/PDF sayısal değerleri.
- `cargo test` + `cargo clippy` CI; sürüm başına regresyon.
- **Mevzuat kip testi:** aynı projede Kamu vs Özel çıktısının farkı doğrulansın (KDV hariç/dahil).

### 9.5 Ölçek
- 50–100k poz/rayiç (çok kurum × çok dönem) FTS5 ile sorun değil; ama **kitap silme** (`database.rs:138`) ve toplu içe aktarım (`pozlari_yukle`) işlemleri **transaction** içine alınmalı (yarıda kalırsa tutarlılık).

---

## 10. Veri Stratejisi & İş Modeli (Asıl Hendek)

Yazılım paritesi gerekli ama **yeterli değil**. OSKA'yı OSKA yapan güncel veridir.

1. **Veri hattı kur.** Her dönem (ÇŞB, KGM, DSİ, İLBANK, Kültür, Vakıflar…) poz+rayiç+analiz kitaplarını içe aktar, **elle doğrula**, sürümle. PDF ayrıştırmayı "profil + insan doğrulaması" olarak kurumsallaştır.
2. **Veri paketi olarak dağıt.** Program içinden "2026/Temmuz ÇŞB paketi" tek tıkla kurulur (imzalı `.db`/paket dosyası). Bu, kullanım kolaylığının da kalbi.
3. **Doğruluk garantisi.** "Kaynağı resmî tebliğ, şu tarihte doğrulandı" damgası → denetim güveni.
4. **Ticari model.** Program: uygun/tek seferlik. **Güncel veri aboneliği:** OSKA'nın yıllık ücretinin altında, şeffaf. Asıl tekrarlayan gelir burada.
5. **Topluluk katkısı (opsiyonel):** kullanıcıların özel poz/analiz paylaşımı (moderasyonlu) — ağ etkisi.

> Uyarı: Kurum kitaplarının telif/dağıtım koşullarına dikkat. Resmî yayımlanan birim fiyat/rayiçler kamuya açıktır ama dağıtım biçimi/şartları için hukuki teyit alın.

---

## 11. Somut Sonraki Adımlar (6 Sprint)

| Sprint | Hedef | Çıktı |
|---|---|---|
| **1** | Doğruluk düzeltmeleri | H1 (kâr çifte sayım), H2 (KDV/Kamu kipi), H3 (tarih) + testleri. Küçük ama itibarı kuran iş. |
| **2** | Veri yeri + sağlamlık | DB/autosave `%APPDATA%`, atomik kayıt, yedek rotasyonu, transaction'lar, para tipi kararı. |
| **3** | Rayiç + Analiz çekirdeği | `tur`/`kar_dahil` şeması, analiz föyü ekranı, analizden poz fiyatı. |
| **4** | Metraj cetveli olgunlaşması | Benzer/çıkan/formül/imalat cinsi + satır içi hızlı giriş. |
| **5** | Resmî çıktılar | Yaklaşık maliyet hesap cetveli, metraj cetveli, metraj icmali, analiz föyü → Excel **ve** PDF (Gizli damga, imza blokları). |
| **6** | Çoklu format içe aktarma + veri paketi | Kurum ayrıştırma profilleri + "güncel dönemi kur" akışı. |

Bu 6 sprint sonunda Metrajmatik **mevzuata doğru, analiz yapabilen, resmî çıktı üreten, güncel veriyle beslenen** bir yaklaşık maliyet programı olur — yani OSKA/AMP ile **aynı masaya** oturur. P2 (hakediş, fiyat farkı, bulut) ile de masayı devirir.

---

### Ek: En kritik 3 iş (bugün başlanacak)
1. **İcmal doğruluğu (H1+H2).** `app.rs:1015-1018` ve `export.rs:236-239` — kâr çifte sayımını ve KDV'yi düzelt; "Kamu/Özel" anahtarı ekle. *En düşük efor, en yüksek itibar.*
2. **Veri yerini güvene al (E8).** `app.rs:105,119` — `%APPDATA%\Metrajmatik\`. *Veri kaybını önler.*
3. **Analiz çekirdeği (E1).** Rayiç kitabı + analiz föyü. *OSKA/AMP'den ayıran asıl özellik.*
