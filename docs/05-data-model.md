# 05 — Data Model & Algoritma Sinkronisasi

---

## 1. Tipe data bersama (TypeScript)

```ts
export type PlaybackStatus = "playing" | "paused" | "stopped";

export interface TrackInfo {
  /** ID stabil untuk cache; null jika sumber GSMTC tanpa ID */
  id: string | null;
  title: string;
  artist: string;
  album: string;
  /** ms */
  durationMs: number;
  /** URL cover, null jika tidak tersedia */
  coverUrl: string | null;
  /** sumber data */
  source: "gsmtc" | "spotify";
}

export interface PlaybackSnapshot {
  track: TrackInfo | null;
  status: PlaybackStatus;
  /** posisi playback saat snapshot diambil (ms) */
  positionMs: number;
  /** waktu lokal (epoch ms) saat snapshot diambil */
  capturedAt: number;
}

export interface LyricsLine {
  text: string;
  startMs: number;
  /** dari lyricsfile; fallback: startMs baris berikutnya */
  endMs: number;
  words?: WordTiming[];
}

export interface WordTiming {
  text: string;
  startMs: number;
  endMs: number;
}

export interface LyricsDoc {
  trackKey: string;
  instrumental: boolean;
  hasWordSync: boolean;
  lines: LyricsLine[];
  /** true jika hanya plainLyrics (tanpa timestamp) */
  unsynced: boolean;
  provider: "lrclib";
  providerId: number | null;
  fetchedAt: number;
}

export type LyricsState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ready"; doc: LyricsDoc }
  | { kind: "instrumental" }
  | { kind: "not_found" }
  | { kind: "error"; message: string };
```

---

## 2. Struct Rust (backend)

```rust
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub id: Option<String>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
    pub source: TrackSource,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackSource { Gsmtc, Spotify }

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshot {
    pub track: Option<TrackInfo>,
    pub status: PlaybackStatus,
    pub position_ms: u64,
    pub captured_at: u64, // epoch ms (Date::now())
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackStatus { Playing, Paused, Stopped }
```

Semua dikirim ke frontend lewat event:

```rust
window.emit("playback:update", &snapshot)?;
window.emit("lyrics:update", &lyrics_state)?;
```

---

## 3. Kunci cache lirik

Prioritas:

```text
1. track.id        → "sp:" + id                    (paling stabil)
2. metadata hash   → "meta:" + sha1(artist|title|round(duration))
```

Normalisasi sebelum hashing agar variasi penulisan tidak memecah cache:

```ts
const normalize = (s: string) =>
  s.toLowerCase()
   .replace(/\s*[([].*?[)\]]\s*/g, " ")   // buang "(Remastered 2011)" dll
   .replace(/\s*-\s*(remaster(ed)?|live|feat\.?.*)$/i, "")
   .replace(/[^\p{L}\p{N}\s]/gu, "")
   .replace(/\s+/g, " ")
   .trim();
```

Contoh normalisasi:

```text
"Creep (Remastered 2011)"      → "creep"
"Taylor Swift feat. Ed Sheeran" → "taylor swift"
```

---

## 4. Algoritma sinkronisasi lirik

### 4.1 Menghitung posisi berjalan

```ts
let snapshot: PlaybackSnapshot;
let anchorPosition = snapshot.positionMs;
let anchorTime = performance.now();
let rate = snapshot.status === "playing" ? 1 : 0;

function currentPositionMs(): number {
  return anchorPosition + (performance.now() - anchorTime) * rate;
}
```

### 4.2 Koreksi drift

```ts
function applySnapshot(next: PlaybackSnapshot) {
  const estimated = currentPositionMs();
  const actual = next.positionMs;

  // seek / lagu baru / pause → snap
  if (Math.abs(actual - estimated) > 800 || next.status !== "playing") {
    anchorPosition = actual;
    anchorTime = performance.now();
    rate = next.status === "playing" ? 1 : 0;
  }
  // selisih kecil → ninja-smooth, jangan lompat
  else {
    anchorPosition = actual;
    anchorTime = performance.now();
  }
}
```

Rekomendasi tambahan: offset global dari preferensi user (`lyricsOffsetMs`) untuk kompensasi delay audio Bluetooth:

```ts
const pos = currentPositionMs() + offsetMs;
```

### 4.3 Mencari baris aktif (binary search)

```ts
function findLineIndex(lines: LyricsLine[], posMs: number): number {
  let lo = 0, hi = lines.length - 1, ans = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].startMs <= posMs) { ans = mid; lo = mid + 1; }
    else hi = mid - 1;
  }
  return ans;
}
```

### 4.4 Progres baris (untuk animasi)

```ts
const line = lines[i];
const next = lines[i + 1];
const end = line.endMs || next?.startMs || line.startMs + 4000;
const progress = clamp01((posMs - line.startMs) / (end - line.startMs));
```

### 4.5 Render loop

```ts
function loop() {
  const pos = currentPositionMs() + offsetMs;
  const i = findLineIndex(lines, pos);

  if (i !== lastIndex) {
    render(i);          // ganti baris + animasi
    lastIndex = i;
  }
  updateProgress(pos);  // CSS variable, tanpa DOM reflow besar
  requestAnimationFrame(loop);
}
```

Optimasi: gunakan CSS `transform`/`opacity` dan CSS variable, hindari mengubah `innerText` tiap frame.

---

## 5. Alur state

```text
        ┌─────────┐
        │  idle   │
        └────┬────┘
             │ track terdeteksi
             ▼
        ┌─────────┐  tidak ada lirik
        │ loading ├──────────────► not_found
        └────┬────┘
             │ 200 OK
             ▼
        ┌─────────┐   instrumental = true
        │  ready  ├──────────────► instrumental
        └────┬────┘
             │ lagu berganti
             ▼
        ┌─────────┐
        │  idle   │
        └─────────┘
```

Aturan:

1. Saat lagu berganti → reset state ke `idle`, batalkan request sebelumnya.
2. Cache hit → langsung `ready`, tanpa `loading`.
3. Error jaringan → `error`, pertahankan lirik lama jika ada (jangan berkedip kosong).
4. `not_found` → tampilkan "Lirik tidak tersedia" kecil, jangan menutupi widget.

---

## 6. Penanganan offset lirik

Simpan di store:

```ts
interface Settings {
  offsetMs: number;          // -2000..2000, default 0
  showAlbumArt: boolean;     // default true
  fontSizeScale: number;     // 0.8..1.4
  clickThrough: boolean;
  hotkey: string;            // default "CommandOrControl+Alt+L"
}
```

UI kecil di tray: `+100 ms`, `-100 ms`, `Reset`.

---

## 7. Kontrak event IPC

| Event | Payload | Frekuensi |
|---|---|---|
| `playback:update` | `PlaybackSnapshot` | 2×/detik |
| `lyrics:update` | `LyricsState` | saat lagu berganti |
| `settings:update` | `Settings` | saat user mengubah |

| Command | Arah | Fungsi |
|---|---|---|
| `login_spotify` | frontend → Rust | mulai OAuth |
| `logout_spotify` | frontend → Rust | hapus token |
| `refresh_lyrics` | frontend → Rust | paksa ambil ulang |
| `set_click_through` | frontend → Rust | toggle klik tembus |
| `set_window_visible` | frontend → Rust | show/hide |

---

## 8. Contoh data uji

Gunakan `samples/lrclib-creep.sample.json` dan `samples/spotify-currently-playing.sample.json` sebagai fixture untuk unit test parser dan sync engine tanpa memanggil jaringan.