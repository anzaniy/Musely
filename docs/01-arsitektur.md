# 01 — Arsitektur

## Tujuan

Menampilkan lirik lagu Spotify yang sedang diputar di jendela kecil yang selalu terlihat, dengan sinkronisasi waktu yang halus dan konsumsi resource rendah.

---

## Alur data

```text
┌─────────────────────────────┐
│ Spotify Desktop / Web       │
└──────────────┬──────────────┘
               │ (audio diputar)
               ▼
┌─────────────────────────────┐
│ Sumber data playback        │
│ A. GSMTC (Windows, utama)   │
│ B. Spotify Web API (opsional)│
└──────────────┬──────────────┘
               │ TrackInfo + posisi ms
               ▼
┌─────────────────────────────┐
│ Rust backend (Tauri)        │
│ - polling playback          │
│ - cache lirik               │
│ - hit LRCLIB                │
└──────────────┬──────────────┘
               │ event / command IPC
               ▼
┌─────────────────────────────┐
│ Frontend widget (WebView)   │
│ - estimasi posisi lokal     │
│ - cari baris aktif          │
│ - animasi & highlight       │
└─────────────────────────────┘
```

Prinsip penting: **backend hanya memberi snapshot**, sedangkan **frontend menginterpolasi waktu** setiap frame. Ini menghindari polling 10×/detik ke API.

---

## Mengapa GSMTC sebagai sumber utama

GSMTC (Global System Media Transport Controls) adalah API Windows yang membaca sesi media dari aplikasi apa pun, termasuk Spotify.

Kelebihan:

- tidak butuh OAuth, Client ID, atau login;
- tidak kena kuota Spotify;
- mendukung Spotify Desktop maupun Web;
- menyediakan posisi playback (`TimelineProperties`) sehingga sinkronisasi lirik lebih akurat;
- tetap bekerja saat Spotify di-pause/seek.

Keterbatasan:

- Windows-only;
- metadata lebih terbatas (judul, artis, album, thumbnail; tanpa ID Spotify);
- beberapa aplikasi tidak mengekspos durasi/posisi dengan baik.

Karena itu Spotify Web API tetap disediakan sebagai **jalur kedua**, misalnya untuk mendapatkan album art resolusi tinggi, durasi presisi, atau ID track.

Namespace .NET: `Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager`.
Di Rust bisa diakses lewat crate `windows` (namespace `Windows::Media::Control`).

---

## Pembagian modul

```text
src-tauri/src/
├── main.rs              ← bootstrap Tauri
├── playback/
│   ├── mod.rs           ← trait PlaybackSource
│   ├── gsmtc.rs         ← implementasi Windows GSMTC
│   └── spotify.rs       ← implementasi Spotify Web API
├── lyrics/
│   ├── mod.rs
│   ├── lrclib.rs        ← HTTP client + pencarian berlapis
│   ├── parser.rs        ← LRC & Lyricsfile YAML → struct
│   └── cache.rs         ← cache disk (JSON per track)
├── auth/
│   └── spotify_pkce.rs  ← OAuth PKCE + refresh token
├── config.rs            ← baca .env / store
└── commands.rs          ← perintah IPC ke frontend

src/ (frontend)
├── main.ts
├── ui/renderer.ts       ← render baris lirik
├── ui/sync.ts           ← estimasi posisi & pencarian baris
└── styles.css
```

Interface antar modul dibuat seragam supaya GSMTC dan Spotify API bisa ditukar tanpa mengubah UI:

```rust
#[async_trait]
pub trait PlaybackSource {
    async fn snapshot(&self) -> Result<Option<PlaybackSnapshot>>;
}
```

---

## Strategi polling

| Data | Interval | Catatan |
|---|---|---|
| Metadata lagu (judul/artis/album) | 1–2 detik | Perubahan jarang |
| Posisi playback | 500 ms | GSMTC murah; Spotify API lebih boros |
| Lirik | saat lagu berganti | Di-cache, tidak diulang |
| Render baris aktif | `requestAnimationFrame` | Dihitung lokal, bukan polling |

Estimasi posisi di frontend:

```text
posisi_sekarang = posisi_snapshot + (waktu_sekarang - waktu_snapshot) × is_playing
```

Koreksi: jika selisih estimasi vs snapshot > 800 ms (misal habis seek), lakukan *snap* langsung.

---

## Penanganan kasus khusus

| Kasus | Perilaku |
|---|---|
| Tidak ada lagu diputar | Tampilkan "Tidak ada lagu" atau sembunyikan widget |
| Lagu instrumental | Tampilkan label "Instrumental" |
| Lirik tidak ditemukan | Coba `/api/search`, lalu tampilkan pesan halus |
| Lirik hanya plain (tanpa timestamp) | Tampilkan statis, tanpa highlight |
| Ada word-sync | Opsional: highlight per kata |
| Offset lirik meleset | Sediakan pengaturan offset ±ms |
| Ganti lagu cepat | Batalkan request lama (`AbortController` / cancel token) |
| Offline | Pakai cache terakhir, tampilkan indikator offline |

---

## Keputusan yang masih terbuka

1. **Frontend**: vanilla JS (paling ringan) vs React (lebih mudah dikembangkan).
2. **Sumber playback default**: GSMTC saja, atau GSMTC + Spotify API sekaligus.
3. **Bentuk widget**: kartu kecil (album + 3 baris lirik) atau lirik saja tanpa album art.
4. **Interaksi**: bisa digeser, atau benar-benar click-through (klik tembus).
5. **Hotkey global** untuk show/hide (mis. `Ctrl+Alt+L`).