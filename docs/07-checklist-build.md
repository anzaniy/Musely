# 07 — Checklist Build

Urutan pengerjaan yang sudah disusun agar tinggal eksekusi.

---

## Fase 0 — Prasyarat mesin

- [x] Install **rustup** → rustc/cargo 1.99.1
- [ ] Install **Visual Studio Build Tools** → workload *Desktop development with C++* (sedang berjalan)
- [x] Pastikan **WebView2 Runtime** ada → versi 156.0.4314.58
- [ ] Verifikasi:

```powershell
rustc --version
cargo --version
node -v
npm -v
```

- [ ] (Opsional) Install `ffmpeg` — hanya jika nanti ingin uji coba ekstraksi media

> Blocker saat ini: Rust & Cargo **belum terpasang** di mesin ini.

---

## Fase 1 — Kredensial Spotify

- [ ] Buat app di https://developer.spotify.com/dashboard
- [ ] Tambahkan Redirect URI: `http://127.0.0.1:8888/callback`
- [ ] Catat **Client ID**
- [ ] Pastikan akun owner **Premium**
- [ ] Isi `.env` dari `.env.example`

Checklist pengujian OAuth:

- [ ] URL authorize terbuka di browser
- [ ] Callback diterima di port 8888
- [ ] `state` diverifikasi
- [ ] Access token + refresh token tersimpan
- [ ] Refresh otomatis saat < 60 detik

---

## Fase 2 — Scaffold proyek

- [x] Scaffold `vanilla-ts` dibuat (create-tauri-app 4.7.5)
- [x] `identifier` diganti → `com.yourname.spotifylyricswidget`
- [x] Konfigurasi window overlay (transparan, always-on-top, tanpa dekorasi, skip taskbar)
- [x] UI demo lirik + animasi berjalan
- [x] `npm install` & `npm run build` berhasil
- [x] `npm run tauri dev` berhasil membuka window (terverifikasi, 380×150)
- [x] Pasang plugin: `store`, `window-state`, `global-shortcut`, `single-instance`

Kriteria selesai: window kotak transparan, tanpa title bar, tetap di atas Notepad. ✅

---

## Fase 3 — Sumber playback

Pilih salah satu jalur lebih dulu.

### 3A. GSMTC (disarankan lebih dulu)

- [x] Tambahkan crate `windows` dengan namespace `Windows::Media::Control`
- [x] Ambil session manager → current session
- [x] Baca `TryGetMediaPropertiesAsync` (title, artist, album)
- [x] Baca `GetTimelineProperties` (position, end time)
- [x] Baca `GetPlaybackInfo` (playback status)
- [x] Emit `playback:update` tiap 500 ms
- [x] Verifikasi dengan lagu Spotify yang benar-benar diputar (Pamungkas, Kaleb J, Baila)
- [ ] Thumbnail album (butuh `IRandomAccessStreamReference`, ditunda)

Kriteria selesai: memutar lagu di Spotify Desktop membuat widget menampilkan judul & artis yang benar.

### 3B. Spotify Web API (opsional / pelengkap)

- [ ] Implementasi PKCE (lihat `02-spotify-api.md`)
- [ ] Panggil `GET /me/player/currently-playing` tiap 2 detik
- [ ] Tangani `204` (tidak ada lagu) dan `429` (`Retry-After`)
- [ ] Refresh token otomatis

Kriteria selesai: metadata + cover resolusi tinggi muncul, termasuk saat Spotify Web dipakai.

---

## Fase 4 — Klien lirik

- [x] HTTP client Rust dengan header `User-Agent` aplikasi
- [x] `GET /api/get` dengan `track_name`, `artist_name`, `album_name`, `duration`
- [x] Fallback `GET /api/search`, pilih durasi terdekat
- [x] Parse `syncedLyrics` (LRC) — 4 unit test lulus
- [x] Deteksi `instrumental` dan `hasWordSync`
- [ ] Cache JSON per track di `app_data_dir/cache/lyrics/`
- [x] Batalkan request saat lagu berganti (dedupe per track key)
- [x] Verifikasi end-to-end dengan lagu nyata (lirik tersinkron tampil benar)
- [x] Mode ringkas: kartu disembunyikan saat kursor jauh (klik nyata), lirik ber-outline terbaca di atas aplikasi lain; tampilan penuh kembali saat kursor di atasnya
- [x] Deteksi iklan Spotify (tidak memanggil LRCLIB, tampilkan status iklan)
- [x] Fallback lirik tanpa timestamp (disebar merata + badge `PLAIN`)
- [x] Retry otomatis saat LRCLIB membalas 5xx

Kriteria selesai: memutar lagu apa pun menampilkan lirik dengan timestamp benar.

---

## Fase 5 — Sinkronisasi & UI

- [x] Estimasi posisi lokal (anchor + elapsed)
- [x] Binary search baris aktif
- [x] Koreksi drift bertahap (ambang 900 ms, koreksi 25%/poll)
- [x] Histeresis baris: abaikan mundur 1 baris < 140 ms (anti-kedip)
- [x] Animasi transisi baris (fade + slide)
- [x] Progress bar berbasis durasi lagu
- [x] Offset lirik (tombol −/+, klik nilai untuk reset, tersimpan di localStorage)
- [x] State kosong: tidak ada lagu / instrumental / lirik tidak ditemukan
- [x] Drag untuk memindah, posisi tersimpan (plugin window-state)
- [x] Bar kontrol: prev / play-pause / next / offset / keluar

Kriteria selesai: lirik berganti tepat waktu selama 3 menit penuh tanpa drift terlihat.

---

## Fase 6 — Fitur pendukung

- [x] Tray icon + menu (tampil/sembunyi, offset, keluar)
- [ ] Hotkey global show/hide
- [x] Toggle click-through (command tersedia, belum ada binding UI)
- [x] Ingat posisi window
- [x] Single instance
- [x] Cache lirik in-memory selama aplikasi berjalan
- [ ] Mode offline (cache persisten di disk)
- [x] Tombol keluar (✕ di widget + menu tray)

---

## Fase 7 — Build & distribusi

- [ ] Ikon aplikasi (32, 128, 128@2x, ico)
- [ ] `npm run tauri build`
- [ ] Uji installer NSIS di mesin bersih
- [ ] Pastikan `identifier` unik
- [ ] (Jika dibagikan) tambahkan atribusi LRCLIB & disclaimer non-afiliasi

---

## Kriteria penerimaan keseluruhan

1. Widget muncul < 1 detik setelah lagu diputar.
2. Lirik berganti dalam toleransi ±150 ms dari audio.
3. Tidak ada lonjakan CPU > 3% saat idle.
4. RAM < 120 MB.
5. Tetap terlihat di atas browser saat berpindah tab.
6. Tidak crash saat Spotify ditutup, di-pause, atau lagu diganti cepat.
7. Offline: menampilkan cache atau pesan yang jelas.

---

## Risiko teknis & mitigasi

| Risiko | Mitigasi |
|---|---|
| GSMTC tidak memberi posisi akurat | Fallback ke Spotify API / interpolasi |
| Lirik tidak ditemukan (lagu daerah/remix) | `/api/search` + opsi muat file `.lrc` manual |
| Rate limit LRCLIB | Cache 30 hari + `Retry-After` |
| Rate limit Spotify | Polling 2 detik + backoff |
| Widget hilang dari topmost | Re-assert `set_always_on_top(true)` |
| Transparansi tidak bekerja | Uji `transparent` + CSS; fallback latar solid |
| Token kedaluwarsa | Refresh proaktif + login ulang bila `invalid_grant` |

---

## Yang sudah selesai di tahap persiapan ini

- [x] Keputusan stack & arsitektur
- [x] Verifikasi endpoint Spotify (termasuk perubahan redirect URI 2026)
- [x] Verifikasi endpoint LRCLIB + contoh respons nyata
- [x] Rancangan konfigurasi window Tauri
- [x] Data model & algoritma sinkronisasi
- [x] Catatan legal & batasan akun
- [x] Sample JSON untuk fixture test

## Yang belum ada (memang belum dikoding)

- [ ] Seluruh kode Rust & frontend
- [ ] Instalasi Rust di mesin
- [ ] Client ID Spotify
- [ ] Ikon aplikasi
- [ ] Aset desain final
