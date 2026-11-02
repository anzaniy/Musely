# 02 — Spotify Web API

Diverifikasi dari dokumentasi resmi Spotify (Oktober 2026).

---

## 1. Membuat aplikasi

1. Buka https://developer.spotify.com/dashboard
2. **Create app**
3. Isi nama & deskripsi (bebas, untuk pemakaian pribadi)
4. **Redirect URI** → `http://127.0.0.1:8888/callback`
5. Simpan **Client ID**
6. Client Secret **tidak diperlukan** jika memakai PKCE

### Aturan redirect URI (penting)

- `http://localhost:...` **tidak lagi didukung** untuk aplikasi baru.
- Wajib memakai **loopback IP**: `http://127.0.0.1:PORT/callback`.
- Untuk non-loopback wajib `https://`.
- **Dynamic port didukung** untuk loopback, tetapi port harus tetap dikunci di satu nilai agar cocok persis.

---

## 2. OAuth 2.0 Authorization Code + PKCE

### Scope minimal

```text
user-read-currently-playing user-read-playback-state
```

Scope tambahan hanya jika nanti ingin tombol kontrol:

```text
user-modify-playback-state
```

### Langkah A — buat code verifier & challenge

```text
code_verifier  = 64 karakter acak [A-Z a-z 0-9 - . _ ~]
code_challenge = BASE64URL( SHA256(code_verifier) )   tanpa padding "="
method         = S256
```

Simpan `code_verifier` di memori sampai callback datang.

### Langkah B — buka halaman otorisasi

```http
GET https://accounts.spotify.com/authorize
```

| Parameter | Nilai |
|---|---|
| `client_id` | Client ID aplikasi |
| `response_type` | `code` |
| `redirect_uri` | `http://127.0.0.1:8888/callback` (harus sama persis) |
| `scope` | `user-read-currently-playing user-read-playback-state` |
| `state` | string acak (anti-CSRF, wajib diverifikasi saat callback) |
| `code_challenge_method` | `S256` |
| `code_challenge` | hasil langkah A |

Buka URL ini di browser default. Setelah user menyetujui, browser diarahkan ke:

```text
http://127.0.0.1:8888/callback?code=AQD...&state=xyz
```

### Langkah C — tukar code menjadi token

```http
POST https://accounts.spotify.com/api/token
Content-Type: application/x-www-form-urlencoded
```

| Body | Nilai |
|---|---|
| `grant_type` | `authorization_code` |
| `code` | dari callback |
| `redirect_uri` | sama persis seperti di langkah B |
| `client_id` | Client ID |
| `code_verifier` | dari langkah A |

Respons `200 OK`:

```json
{
  "access_token": "BQD...",
  "token_type": "Bearer",
  "scope": "user-read-currently-playing user-read-playback-state",
  "expires_in": 3600,
  "refresh_token": "AQD..."
}
```

### Langkah D — refresh token

```http
POST https://accounts.spotify.com/api/token
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token
refresh_token=<refresh_token>
client_id=<client_id>
```

- Access token berlaku **3600 detik**. Refresh saat sisa < 60 detik.
- Respons refresh **mungkin tidak** menyertakan `refresh_token` baru; jika tidak ada, tetap pakai yang lama.
- Refresh token bisa kedaluwarsa jika user mencabut akses → tangani `400 invalid_grant` dengan meminta login ulang.

---

## 3. Endpoint yang dipakai

### Get Currently Playing Track

```http
GET https://api.spotify.com/v1/me/player/currently-playing
Authorization: Bearer <access_token>
```

Parameter opsional: `market`, `additional_types=track,episode`.

**Status respons:**

| Status | Arti |
|---|---|
| `200` | ada objek lagu |
| `204` | tidak ada yang diputar (body kosong) — **wajib ditangani** |
| `401` | token kedaluwarsa/invalid |
| `403` | user tidak ada di allowlist app development mode |
| `429` | kena rate limit / kuota |

Field penting:

| Field | Kegunaan |
|---|---|
| `is_playing` | status putar |
| `progress_ms` | posisi saat ini (ms) |
| `timestamp` | waktu server saat snapshot diambil (epoch ms) |
| `item.name` | judul lagu |
| `item.artists[].name` | artis |
| `item.album.name` | album |
| `item.album.images[]` | cover, urut dari terbesar |
| `item.duration_ms` | durasi total |
| `item.id` | Spotify track ID (untuk cache) |
| `item.external_urls.spotify` | link lagu |
| `currently_playing_type` | `track` / `episode` / `ad` / `unknown` |

Catatan: `item` bisa `null` (misal saat iklan). `currently_playing_type` harus dicek sebelum render.

### Get Playback State (opsional)

```http
GET https://api.spotify.com/v1/me/player
```

Memberi info tambahan: `device`, `shuffle_state`, `repeat_state`, `actions`. Bisa dipakai untuk indikator perangkat aktif. Tidak wajib.

---

## 4. Batasan development mode

| Aturan | Nilai |
|---|---|
| Status awal app | development mode |
| Owner app | **wajib Spotify Premium** agar app berfungsi |
| Jumlah user terautentikasi | maksimal **5** |
| User lain | harus ditambahkan ke allowlist lewat Settings → Users Management |
| Tanpa allowlist | request dapat `403` |
| Extended quota mode | hanya untuk organisasi, butuh ≥250k MAU → tidak relevan untuk proyek pribadi |

Konsekuensi: **pakai akun Spotify Premium milik sendiri** sebagai owner dan satu-satunya user.

---

## 5. Rate limit & kuota

- Rate limit dihitung dari jumlah request dalam **rolling window 30 detik**.
- Development mode punya kuota terpisah (bukan rate limit) yang bisa menghasilkan `429` dengan `reason: "QUOTA_EXCEEDED"`.
- Saat `429`, baca header **`Retry-After`** (detik) lalu tunggu, tambahkan jitter.
- Rekomendasi: polling `currently-playing` **jangan lebih dari 1×/detik**. Ideal 2 detik sekali. Ini sudah cukup karena posisi diinterpolasi lokal.

Contoh penanganan:

```text
if status == 429:
    tunggu = Retry-After + random(0..1s)
    retry (maks 3×)
```

---

## 6. Penyimpanan token

- Simpan `refresh_token` di penyimpanan lokal aplikasi (mis. `tauri-plugin-store` atau file di `app_data_dir`).
- Jangan pernah menaruh refresh token di localStorage frontend yang bisa dibaca skrip pihak ketiga.
- Client Secret tidak dipakai sama sekali (PKCE).

---

## 7. Catatan kebijakan (ringkas)

Dua butir dari halaman resmi endpoint ini yang relevan:

1. *"Streaming applications may not be commercial — The Spotify Platform can not be used to develop commercial streaming integrations."*
2. *"Do not synchronize Spotify content — You may not synchronize any sound recordings with any visual media..."*

Untuk aplikasi pribadi non-komersial risikonya rendah, tetapi aplikasi berbayar yang menampilkan lirik tersinkron berada di area berisiko. Detail di `06-legal-dan-risiko.md`.