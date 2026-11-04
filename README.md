# Musely

A small Windows desktop widget that shows the lyrics of the song currently playing, always on top of other windows, so it stays visible while you open other tabs/apps.

Status: **working** — the widget reads the current song from GSMTC (no Spotify login), fetches synced lyrics from LRCLIB, and has a control bar (prev / play-pause / next / offset / quit).

---

## Controls & navigation

| Action | How |
|---|---|
| Move the widget | Drag the title/cover area |
| Previous / next song | `⏮` / `⏭` buttons |
| Play / pause | Middle button (icon follows the current state) |
| Adjust lyric timing | `−` (lyrics 100 ms later), `+` (100 ms earlier) |
| Reset offset | Click the offset value (`0 ms`) |
| Quit completely | `✕` button at the bottom right |
| Hide / show | Click the tray icon, or tray menu → *Show / Hide* |
| Quit from tray | Tray menu → *Quit* |

The offset is stored in `localStorage`, so it persists after the app is closed.

## Compact mode (hover)

When the cursor is not over the widget, the card/header/buttons are hidden and only the lyric text with a dark outline stays visible — still readable over other apps. Move the cursor onto the widget to bring the full view back (card, controls, progress).

- Compact mode only activates while lyrics are being shown. When idle, during ads, instrumentals, or "not available", the widget stays fully expanded.
- The window size never changes, so nothing flickers. Note: the transparent area still catches clicks (not click-through yet) because hover needs mouse events.

> On Windows 11, new tray icons are often hidden in the `^` panel. Open that panel and drag the *Musely* icon onto the taskbar to keep it visible.

## Folder structure

```text
musely/
├── README.md                     ← this file
├── .env.example                  ← configuration template
├── docs/
│   ├── 01-arsitektur.md          ← data flow & module breakdown
│   ├── 02-spotify-api.md         ← PKCE auth, endpoints, limits
│   ├── 03-lrclib-api.md          ← lyrics endpoints, format, rate limits
│   ├── 04-tauri-widget.md        ← overlay window config + plugins
│   ├── 05-data-model.md          ← data types & sync algorithm
│   ├── 06-legal-dan-risiko.md    ← Spotify policy & lyric copyright
│   └── 07-checklist-build.md     ← build order + prerequisites
└── samples/
    ├── spotify-currently-playing.sample.json
    └── lrclib-creep.sample.json
```

---

## Prerequisites

```text
Node.js        v24.13.1   OK
npm            11.8.0     OK
Rust           1.99.1     OK
Cargo          1.99.1     OK
MSVC Build Tools 2022     OK
WebView2 Runtime          OK (built into Windows 11)
```
---

## Run

```powershell
npm install
npm run tauri dev      # development mode
npm run tauri build    # NSIS installer in src-tauri/target/release/bundle/
```

Note: the `cargo` command must be on `PATH` (`$env:USERPROFILE\.cargo\bin`).

---

## How it works

1. A Rust thread polls GSMTC every 500 ms → `playback:update`.
2. When title/artist/duration changes, lyrics are fetched from LRCLIB → `lyrics:update`.
3. The frontend interpolates position between polls, corrects drift gradually, then picks the active line with binary search.
4. Lyrics results are cached in memory so replaying the same song does not load them again.
5. Idle is only announced after 3 consecutive read failures (~1.5 s) to prevent flicker.

---