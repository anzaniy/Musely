# 06 — Legal & Risiko

Ringkasan ini berdasarkan dokumentasi resmi Spotify dan karakteristik LRCLIB. **Bukan nasihat hukum.**

---

## 1. Kutipan kebijakan Spotify yang relevan

Dari halaman resmi `Get Currently Playing Track` (developer.spotify.com):

> **"Streaming applications may not be commercial"**
> *The Spotify Platform can not be used to develop commercial streaming integrations.*

> **"Do not synchronize Spotify content"**
> *You may not synchronize any sound recordings with any visual media, including any advertising, film, television program, slideshow, video, or similar content.*

> **"Spotify content may not be broadcasted"**
> *The Spotify Platform can not be used for non-interactive broadcasting.*

Interpretasi praktis:

- Menampilkan lirik yang **disinkronkan dengan lagu** adalah area yang paling rawan ditafsirkan melanggar butir kedua.
- Untuk **pemakaian pribadi, non-komersial, lokal**, risiko penegakan sangat rendah.
- Untuk **produk berbayar**, risiko meningkat drastis karena:
  1. melanggar larangan integrasi streaming komersial;
  2. memonetisasi konten lirik yang bukan milikmu.

---

## 2. Batasan teknis-akun yang perlu diketahui

| Hal | Dampak |
|---|---|
| Development mode: owner wajib Premium | Tanpa Premium, app tidak berfungsi |
| Maksimal 5 user terautentikasi | Tidak bisa jadi produk publik tanpa extended quota |
| Extended quota hanya untuk organisasi | Perusahaan terdaftar, ≥250.000 MAU |
| User di luar allowlist → `403` | Tidak bisa dibagikan bebas |

Artinya secara praktis proyek ini **memang dirancang untuk pemakaian pribadi**.

---

## 3. Hak cipta lirik

Lirik adalah karya sastra berhak cipta (pengarang lagu + penerbit).

| Penggunaan | Risiko |
|---|---|
| Menampilkan lirik untuk diri sendiri | Sangat rendah |
| Open-source, tanpa monetisasi, pengguna menyediakan sumber sendiri | Rendah |
| Aplikasi gratis yang membundel lirik pihak ketiga | Sedang — tetap berpotensi takedown |
| Aplikasi berbayar dengan lirik dari database komunitas | **Tinggi** |

Layanan legal (Spotify, Apple Music, Musixmatch) membayar lisensi lirik melalui agregator seperti **LyricFind** atau **Musixmatch for Business**. Menjual aplikasi lirik tanpa lisensi setara menjual konten yang tidak kamu miliki haknya.

Catatan tentang LRCLIB: database ini **komunitas/terbuka** dan menyediakan API gratis, tetapi lisensi tiap lirik tidak secara eksplisit diselesaikan dengan penerbit. Menggunakannya secara pribadi umum; menggunakannya sebagai basis produk komersial tidak aman.

---

## 4. Rekomendasi jalur aman

### Untuk pemakaian sendiri (pilihan utama)
- Jalankan lokal, jangan dijual.
- Jangan publikasikan lirik ke internet.
- Cache lirik di perangkat sendiri.

### Jika ingin dibagikan
- Bagikan **kode aplikasinya**, bukan kumpulan liriknya.
- Biarkan aplikasi mengambil lirik saat runtime dari sumber publik.
- Cantumkan atribusi "Lyrics provided by LRCLIB".
- Jangan memakai logo/branding Spotify secara menyesatkan; ikuti Spotify Design & Branding Guidelines.
- Tambahkan disclaimer: bukan produk resmi Spotify.

### Jika ingin dikomersialkan
Wajib:
1. Lisensi lirik komersial (Musixmatch for Business / LyricFind / setara).
2. Kepatuhan penuh Spotify Developer Terms + persetujuan tertulis.
3. Kebijakan privasi & ToS sendiri.
4. Struktur harga yang menutup biaya lisensi.

Tanpa poin 1 dan 2, **jangan dijual**.

---

## 5. Atribusi & branding

Jika tetap dipublikasikan (gratis):

```text
Lirik disediakan oleh LRCLIB (https://lrclib.net)
Data playback menggunakan Spotify Web API
Aplikasi ini tidak berafiliasi dengan Spotify AB
```

Aturan umum Spotify:
- jangan mengubah/menutupi logo Spotify;
- jangan menyiratkan endorsement;
- jangan memakai aset Spotify sebagai ikon aplikasi utama.

---

## 6. Ringkasan keputusan

| Skenario | Status |
|---|---|
| Lokal, pribadi, gratis | ✅ Aman secara praktis |
| Open-source, gratis, tanpa bundel lirik | ⚠️ Risiko rendah |
| Gratis, publik, membundel lirik | ⚠️ Sedang |
| Berbayar dengan lirik tanpa lisensi | ❌ Tidak disarankan |
| Berbayar + lisensi lirik + izin Spotify | ✅ Legal, tetapi mahal & prosesnya panjang |