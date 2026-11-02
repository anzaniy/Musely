# Musely

Widget desktop kecil untuk Windows yang menampilkan lirik lagu yang sedang diputar, selalu di atas jendela lain (always-on-top), sehingga tetap terlihat saat membuka tab/aplikasi lain.

Status: **berfungsi** — widget sudah membaca lagu dari GSMTC (tanpa login Spotify), mengambil lirik tersinkron dari LRCLIB, dan punya bar kontrol (prev / play-pause / next / offset / keluar).

---

## Kontrol & navigasi

| Aksi | Cara |
|---|---|
| Pindah posisi widget | Drag pada area judul/cover |
| Lagu sebelumnya / berikutnya | Tombol `⏮` / `⏭` |
| Play / pause | Tombol tengah (ikon berubah sesuai status) |
| Menyesuaikan waktu lirik | `−` (lirik lebih lambat 100 ms), `+` (lebih cepat 100 ms) |
| Reset offset | Klik angka offset (`0 ms`) |
| Keluar total | Tombol `✕` di kanan bawah |
| Sembunyikan / tampilkan | Klik ikon tray, atau menu tray → *Tampilkan / Sembunyikan* |
| Keluar lewat tray | Menu tray → *Keluar* |

Offset disimpan di `localStorage` sehingga tetap berlaku setelah aplikasi ditutup.

## Mode ringkas (hover)

Saat kursor tidak berada di atas widget, kartu/header/tombol disembunyikan dan hanya teks lirik dengan outline gelap yang tampil tetap terbaca di atas aplikasi lain. Arahkan kursor ke widget untuk memunculkan tampilan penuh lagi (kartu, kontrol, progress).

- Mode ringkas hanya aktif bila ada lirik yang sedang ditampilkan. Saat idle, iklan, instrumental, atau "tidak tersedia", widget tetap tampil penuh.
- Tidak ada perubahan ukuran window, supaya tidak berkedip. Catatan: area transparan tetap menangkap klik (belum click-through) karena hover membutuhkan event mouse.

> Di Windows 11 ikon tray baru sering disembunyikan di panel `^`. Buka panel itu, lalu drag ikon *Musely* ke taskbar agar selalu terlihat.

---

## Ringkasan keputusan

| Aspek | Keputusan | Alasan |
|---|---|---|
| Platform | Windows 10/11 | Target utama pengguna |
| Shell desktop | Tauri v2 (Rust + WebView2) | Binary kecil (~10 MB), RAM hemat, UI fleksibel |
| Frontend | HTML/CSS/JS (vanilla atau React) | Widget sederhana, tidak butuh framework berat |
| Sumber lagu | GSMTC (utama) + Spotify Web API (opsional) | GSMTC real-time & tanpa OAuth; Spotify API untuk metadata lengkap |
| Sumber lirik | LRCLIB | Gratis, tanpa API key, ada lirik tersinkron |
| Distribusi | Lokal / build sendiri | Menghindari risiko komersial (lihat `docs/06`) |

---

## Struktur folder

```text
musely/
├── README.md                     ← file ini
├── .env.example                  ← template konfigurasi
├── docs/
│   ├── 01-arsitektur.md          ← alur data & pembagian modul
│   ├── 02-spotify-api.md         ← auth PKCE, endpoint, batasan
│   ├── 03-lrclib-api.md          ← endpoint lirik, format, rate limit
│   ├── 04-tauri-widget.md        ← config window overlay + plugin
│   ├── 05-data-model.md          ← tipe data & algoritma sinkronisasi
│   ├── 06-legal-dan-risiko.md    ← kebijakan Spotify & hak cipta lirik
│   └── 07-checklist-build.md     ← urutan pengerjaan + prasyarat
└── samples/
    ├── spotify-currently-playing.sample.json
    └── lrclib-creep.sample.json
```

---

## Prasyarat

Sudah terpasang di mesin ini:

```text
Node.js        v24.13.1   OK
npm            11.8.0     OK
Rust           1.99.1     OK
Cargo          1.99.1     OK
MSVC Build Tools 2022     OK
WebView2 Runtime          OK (bawaan Windows 11)
```

Kalau menyiapkan mesin baru:

1. **rustup** → https://rustup.rs (toolchain `stable-x86_64-pc-windows-msvc`)
2. **Visual Studio Build Tools** dengan workload *Desktop development with C++*
3. **WebView2 Runtime** (umumnya sudah ada di Windows 11)

---

## Menjalankan

```powershell
npm install
npm run tauri dev      # mode pengembangan
npm run tauri build    # installer NSIS di src-tauri/target/release/bundle/
```

Catatan: perintah `cargo` harus ada di `PATH` (`$env:USERPROFILE\.cargo\bin`).

---

## Cara kerja singkat

1. Thread Rust melakukan polling GSMTC tiap 500 ms → `playback:update`.
2. Saat judul/artis/durasi berubah, lirik diambil dari LRCLIB → `lyrics:update`.
3. Frontend menginterpolasi posisi antar-poll, mengoreksi drift secara bertahap, lalu memilih baris aktif dengan binary search.
4. Hasil lirik di-cache di memori agar memutar ulang lagu yang sama tidak memuat ulang.
5. Idle baru diumumkan setelah 3 kegagalan baca berturut-turut (±1,5 dtk) untuk mencegah kedip.

---

## Belum dikerjakan

- Cache lirik persisten di disk (saat ini hanya in-memory)
- Hotkey global show/hide
- Thumbnail album dari GSMTC
- Uji installer di mesin bersih
