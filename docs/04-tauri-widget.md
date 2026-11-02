# 04 — Tauri Widget (Windows)

Semua nilai di bawah mengikuti referensi konfigurasi Tauri v2.

---

## 1. Konfigurasi window overlay

`src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Spotify Lyrics Widget",
  "version": "0.1.0",
  "identifier": "com.yourname.spotifylyricswidget",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "widget",
        "title": "Lyrics",
        "width": 380,
        "height": 140,
        "x": 60,
        "y": 60,
        "resizable": false,
        "maximizable": false,
        "minimizable": false,
        "fullscreen": false,
        "decorations": false,
        "transparent": true,
        "shadow": false,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "focus": false,
        "visible": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico"
    ]
  }
}
```

### Arti tiap properti penting

| Properti | Efek |
|---|---|
| `decorations: false` | hilangkan title bar & border bawaan |
| `transparent: true` | latar bisa transparan (perlu CSS `background: transparent`) |
| `alwaysOnTop: true` | tetap di atas jendela lain |
| `skipTaskbar: true` | tidak muncul di taskbar |
| `shadow: false` | hilangkan bayangan OS agar sudut membulat bersih |
| `focus: false` | tidak merebut fokus saat muncul |
| `resizable: false` | ukuran tetap, lebih rapi untuk widget |

Catatan `transparent` di Windows: butuh `transparent: true` **dan** CSS:

```css
html, body { background: transparent; }
```

---

## 2. Mode click-through (opsional)

Agar klik menembus widget ke aplikasi di bawahnya:

```rust
// src-tauri/src/main.rs
use tauri::Manager;

#[tauri::command]
fn set_click_through(window: tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    window.set_ignore_cursor_events(enabled).map_err(|e| e.to_string())
}
```

Konsekuensi: saat click-through aktif, widget **tidak bisa digeser**. Sediakan hotkey untuk toggle.

---

## 3. Drag untuk memindahkan widget

Di HTML, tandai area yang bisa dipakai menggeser:

```html
<div class="widget" data-tauri-drag-region>
  <span class="title" data-tauri-drag-region>Judul Lagu</span>
</div>
```

`data-tauri-drag-region` membuat area tersebut menjadi handle drag tanpa perlu kode Rust.

Simpan posisi terakhir dengan plugin **window-state** agar widget muncul di tempat yang sama saat dibuka lagi.

---

## 4. Plugin yang dibutuhkan

| Plugin | Fungsi | Status |
|---|---|---|
| `tauri-plugin-store` | simpan token, preferensi, offset lirik | wajib |
| `tauri-plugin-opener` | buka browser untuk login Spotify | wajib (jika pakai OAuth) |
| `tauri-plugin-window-state` | ingat posisi & ukuran widget | disarankan |
| `tauri-plugin-global-shortcut` | hotkey show/hide/click-through | disarankan |
| `tauri-plugin-single-instance` | cegah aplikasi terbuka dua kali | disarankan |
| `tauri-plugin-deep-link` | callback `lyricswidget://` (alternatif loopback) | opsional |
| `tauri-plugin-http` | panggil LRCLIB dari JS (bukan Rust) | opsional |

Pemasangan (pilih salah satu gaya):

```bash
npm run tauri add store
npm run tauri add opener
npm run tauri add window-state
npm run tauri add global-shortcut
npm run tauri add single-instance
```

> Catatan: sebaiknya panggil LRCLIB dari **Rust**, bukan JS, supaya bisa mengatur `User-Agent` (syarat LRCLIB) dan menyembunyikan logika cache.

---

## 5. Capabilities

`src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Izin untuk window widget",
  "windows": ["widget"],
  "permissions": [
    "core:default",
    "core:window:allow-set-always-on-top",
    "core:window:allow-start-dragging",
    "opener:default",
    "store:default",
    "global-shortcut:default"
  ]
}
```

Prinsip: beri izin seminimal mungkin, hanya untuk window `widget`.

---

## 6. Tray icon

Berguna agar aplikasi bisa diakses saat widget disembunyikan.

```json
{
  "app": {
    "trayIcon": {
      "id": "main-tray",
      "iconPath": "icons/32x32.png",
      "tooltip": "Spotify Lyrics Widget",
      "menuOnLeftClick": false
    }
  }
}
```

Menu yang disarankan:

```text
Tampilkan / Sembunyikan widget
Mode klik tembus        [on/off]
Offset lirik +100 ms
Offset lirik -100 ms
Muat ulang lirik
Keluar
```

---

## 7. Efek visual (opsional)

Untuk latar semi-transparan bergaya kaca di Windows 11:

```json
{
  "label": "widget",
  "windowEffects": {
    "effects": ["acrylic"],
    "state": "active",
    "radius": 16
  }
}
```

- `acrylic` / `mica` bergantung versi Windows; jika tidak didukung, jatuh ke latar transparan biasa.
- Alternatif yang paling aman dan konsisten: `transparent: true` + CSS `background: rgba(18,18,20,0.72)` + `backdrop-filter: blur(18px)`.

---

## 8. Batasan yang perlu diketahui

1. **Always-on-top tidak menembus game fullscreen eksklusif.** Gunakan mode borderless/windowed.
2. Widget bisa tertutup jendela lain yang juga always-on-top.
3. Windows kadang menurunkan topmost setelah pergantian fokus layar penuh — panggil ulang `set_always_on_top(true)` saat event focus/visibility.
4. Multi-monitor: simpan posisi sebagai koordinat absolut; validasi ulang saat monitor berubah agar widget tidak muncul di luar layar.
5. DPI scaling: gunakan `logical` position/size agar konsisten di layar 125%/150%.

---

## 9. Ukuran & layout yang disarankan

```text
┌────────────────────────────────────────────┐
│ [cover 44px]  Judul Lagu                   │
│               Artist — Album               │
│                                            │
│        Baris lirik aktif (besar)           │
│        baris berikutnya (redup)            │
└────────────────────────────────────────────┘
```

Spesifikasi awal:

| Item | Nilai |
|---|---|
| Ukuran window | 380 × 140 px |
| Padding | 14 px |
| Cover | 44 × 44, radius 8 |
| Judul | 13 px, 600 |
| Artis | 11 px, opacity 0.65 |
| Lirik aktif | 20 px, 600 |
| Lirik berikutnya | 14 px, opacity 0.35 |
| Radius widget | 16 px |
| Latar | `rgba(18,18,20,0.72)` + blur |

Semua nilai ini sengaja dibuat sebagai bahan awal — silakan disesuaikan saat implementasi UI.