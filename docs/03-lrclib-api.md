# 03 — LRCLIB API

Diverifikasi langsung dari `https://lrclib.net/docs.txt` dan uji request nyata (Oktober 2026).

---

## Ringkasan

| Aspek | Nilai |
|---|---|
| Base URL | `https://lrclib.net` |
| API key | **tidak perlu** |
| CORS | aktif (`Access-Control-Allow-Origin: *`) |
| Format | JSON |
| Identifikasi klien | **wajib** mengirim `User-Agent` |

---

## 1. Header wajib

Setiap request harus mengidentifikasi aplikasi:

```http
User-Agent: SpotifyLyricsWidget v0.1.0 (https://github.com/USERNAME/spotify-lyrics-widget)
```

Jika header tidak bisa diubah (browser), gunakan salah satu alternatif:

```http
X-User-Agent: SpotifyLyricsWidget v0.1.0 (...)
Lrclib-Client: SpotifyLyricsWidget v0.1.0 (...)
```

Di Tauri, request dilakukan dari Rust sehingga `User-Agent` bisa diatur bebas. Ini pilihan yang disarankan.

### Opsional: registrasi aplikasi

Daftar di https://lrclib.net/developers untuk limit lebih tinggi.

| Metode | Header | Limit perkiraan |
|---|---|---|
| Tanpa identifikasi | – | ~45 req/menit per IP |
| Klien teridentifikasi | `User-Agent` | ~90 req/menit per IP |
| Terdaftar (client-side) | `Lrclib-App-Id` | ~180 req/menit |
| Terdaftar (server-side) | `Lrclib-Api-Key` | kuota app-wide ~6.000 req/menit |

`Lrclib-App-Id` bersifat publik dan aman disertakan di aplikasi. `Lrclib-Api-Key` **rahasia**, jangan dimasukkan ke client.

---

## 2. Endpoint

### A. Cari lirik berdasarkan tanda tangan lagu (utama)

```http
GET /api/get?track_name=...&artist_name=...&album_name=...&duration=...
```

| Parameter | Wajib | Keterangan |
|---|---|---|
| `track_name` | ya | judul lagu |
| `artist_name` | ya | nama artis |
| `album_name` | disarankan | meningkatkan akurasi |
| `duration` | disarankan | detik, 1–3600 |

**Penting:** `duration` harus cocok dalam toleransi **±2 detik**. Jika tidak cocok, hasilkan `404`. Kirim `duration = round(duration_ms / 1000)` dari Spotify.

Sukses `200 OK` → satu objek lyrics record.
Gagal `404` → `{"name": "TrackNotFound"}`.

### B. Pencarian kata kunci (fallback)

```http
GET /api/search?q=...            (paling longgar)
GET /api/search?track_name=22&artist_name=taylor+swift
```

- Minimal salah satu `q` **atau** `track_name`.
- `q` menang atas parameter terstruktur.
- Maksimal **20 hasil**, tanpa pagination.

Gunakan sebagai cadangan saat `/api/get` gagal, lalu pilih hasil dengan durasi terdekat.

### C. Ambil berdasarkan ID

```http
GET /api/get/{id}
```

Berguna untuk cache: simpan `id` setelah pencarian pertama, lalu ambil ulang tanpa mencocokkan metadata.

---

## 3. Bentuk data (lyrics record)

```json
{
  "id": 496,
  "name": "Creep",
  "trackName": "Creep",
  "artistName": "Radiohead",
  "albumName": "Pablo Honey",
  "duration": 239.0,
  "instrumental": false,
  "hasWordSync": false,
  "plainLyrics": "When you were here before\n...",
  "syncedLyrics": "[00:19.16]When you were here before\n[00:24.09]Couldn't look you in the eye\n...",
  "lyricsfile": "version: '1.0'\nmetadata:\n  title: Creep\n..."
}
```

| Field | Tipe | Catatan |
|---|---|---|
| `id` | number | stabil, bagus untuk cache |
| `duration` | number | detik (bisa desimal) |
| `instrumental` | boolean | `true` → tidak ada lirik |
| `hasWordSync` | boolean | `true` → `lyricsfile` punya timing per kata |
| `plainLyrics` | string \| null | tanpa timestamp |
| `syncedLyrics` | string \| null | format LRC per baris |
| `lyricsfile` | string | YAML, **selalu ada** |

### Format `syncedLyrics` (LRC)

```text
[mm:ss.xx] Teks baris
[01:05.83] I'm a weirdo
```

Regex parsing:

```regex
^\[(\d{2}):(\d{2})\.(\d{2,3})\]\s?(.*)$
```

Milidetik bisa 2 atau 3 digit → normalisasi:

```text
ms = detik * 1000 + (fraksi.length == 2 ? fraksi * 10 : fraksi)
```

### Format `lyricsfile` (YAML) — untuk word-sync

```yaml
version: '1.0'
metadata:
  title: Creep
  artist: Radiohead
  album: Pablo Honey
  duration_ms: 239000
  instrumental: false
lines:
- text: When you were here before
  start_ms: 19160
  end_ms: 24090
- text: ''
  start_ms: 230780
```

Jika ada `lines[].words`, tiap kata punya `start_ms`/`end_ms` sendiri → bisa untuk highlight karaoke per kata.

Rekomendasi: **parse `lyricsfile`** dan turunkan `syncedLyrics` dari situ, karena `lyricsfile` selalu ada dan menyediakan `end_ms` (berguna untuk animasi progres per baris).

---

## 4. Error yang harus ditangani

| Status | `name` | Aksi |
|---|---|---|
| `400` | `ValidationError` | Perbaiki parameter (durasi di luar 1–3600, nama kosong) |
| `404` | `TrackNotFound` | Coba `/api/search`, lalu tampilkan "lirik tidak tersedia" |
| `429` | `RateLimitExceeded` | Tunggu `Retry-After` + jitter 0–30% |
| `500` | `UnknownError` | Retry sekali |
| `503` | `ServerOverloaded` | Tunggu `Retry-After` (biasanya 1 detik) |

Selalu cek status code dan `Content-Type` sebelum parse JSON — error di level framework bisa mengembalikan teks biasa.

---

## 5. Aturan pemakaian yang sopan

1. Kirim request **berurutan**, tunggu selesai sebelum yang berikutnya.
2. Beri jeda **200–500 ms** antar request batch.
3. **Cache agresif** per lagu — lirik tidak berubah, tidak perlu diambil ulang.
4. Hormati `Retry-After`; jangan retry beruntun.
5. Jangan melakukan pencarian setiap 500 ms mengikuti posisi playback.

### Strategi cache yang disarankan

```text
key   = spotify_track_id  atau  hash(artist|title|duration)
value = lyrics record + timestamp diambil
TTL   = 30 hari
path  = app_data_dir/cache/lyrics/<key>.json
```

Alur pencarian:

```text
1. cek cache lokal            → hit? pakai
2. GET /api/get (lengkap)     → 200? simpan & pakai
3. GET /api/search (q=...)    → pilih durasi terdekat (±5 dtk), simpan
4. gagal                      → tampilkan status "lirik tidak tersedia"
```

---

## 6. Uji cepat (sudah berhasil)

```bash
curl -s "https://lrclib.net/api/get?artist_name=Radiohead&track_name=Creep&album_name=Pablo%20Honey&duration=239" \
  -H "User-Agent: SpotifyLyricsWidget v0.1.0 (https://example.com)"
```

Respons nyata tersimpan di `samples/lrclib-creep.sample.json`.