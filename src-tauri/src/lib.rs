mod lyrics;
mod playback;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

/// Berapa kali pembacaan GSMTC boleh gagal sebelum widget dianggap idle.
/// 3 × 500 ms ≈ 1,5 detik — cukup untuk menutupi gangguan sesaat tanpa
/// membuat lirik berkedip.
const IDLE_MISS_LIMIT: u32 = 3;

/// Mengaktifkan/mematikan mode klik-tembus (click-through).
#[tauri::command]
fn set_click_through(window: WebviewWindow, enabled: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|e| e.to_string())
}

/// Menjamin widget tetap di atas jendela lain.
#[tauri::command]
fn set_always_on_top(window: WebviewWindow, enabled: bool) -> Result<(), String> {
    window.set_always_on_top(enabled).map_err(|e| e.to_string())
}

/// Info singkat untuk ditampilkan di UI.
#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({
        "name": "Musely",
        "version": env!("CARGO_PKG_VERSION"),
        "stage": "gsmtc"
    })
}

/// Play / pause / next / previous lewat GSMTC.
#[tauri::command]
fn playback_control(action: String) -> Result<(), String> {
    let parsed = playback::ControlAction::parse(&action)
        .ok_or_else(|| format!("Aksi tidak dikenal: {action}"))?;
    playback::control(parsed)
}

/// Menutup aplikasi sepenuhnya (bukan hanya menyembunyikan window).
#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Menyembunyikan atau menampilkan widget.
fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
        }
    }
}

/// Hasil fetch hanya boleh dipakai kalau lagunya masih yang diputar.
fn is_still_current(current: &Mutex<Option<String>>, key: &Option<String>) -> bool {
    current
        .lock()
        .map(|c| c.as_ref() == key.as_ref())
        .unwrap_or(false)
}

/// Polling GSMTC + pengambilan lirik saat lagu berganti.
fn start_playback_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        playback::init_com();

        let mut last_key: Option<String> = None;
        let mut misses: u32 = 0;
        let mut idle_announced = false;
        // Cache lirik di memori supaya memutar ulang lagu yang sama tidak
        // menampilkan "Memuat lirik…" lagi.
        let cache: Arc<Mutex<HashMap<String, lyrics::LyricsState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // Lagu yang sedang diputar. Hasil fetch lagu lama tidak boleh menimpa
        // lirik lagu baru setelah pengguna skip.
        let current_key: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        loop {
            // Abaikan snapshot tanpa track: bisa jadi gangguan sesaat saat
            // Spotify berganti lagu atau sedang membuka sesi baru.
            let snapshot = playback::read().filter(|s| s.track.is_some());

            match snapshot {
                Some(snapshot) => {
                    misses = 0;
                    idle_announced = false;
                    let _ = app.emit("playback:update", &snapshot);

                    let key = snapshot
                        .track
                        .as_ref()
                        .map(|t| format!("{}|{}|{}", t.title, t.artist, t.duration_ms));

                    if key != last_key {
                        last_key = key.clone();
                        if let Ok(mut current) = current_key.lock() {
                            *current = key.clone();
                        }

                        if let Some(track) = snapshot.track.clone() {
                            if lyrics::is_ad(&track) {
                                // Iklan Spotify: jangan panggil LRCLIB.
                                let _ = app.emit("lyrics:update", lyrics::LyricsState::Ad);
                            } else {
                                let cached =
                                    key.as_ref().and_then(|k| {
                                        cache.lock().ok().and_then(|c| c.get(k).cloned())
                                    });

                                if let Some(state) = cached {
                                    let _ = app.emit("lyrics:update", state);
                                } else {
                                    let _ =
                                        app.emit("lyrics:update", lyrics::LyricsState::Loading);

                                    let handle = app.clone();
                                    let cache_key = key.clone();
                                    let cache_ref = Arc::clone(&cache);
                                    let current_ref = Arc::clone(&current_key);
                                    tauri::async_runtime::spawn(async move {
                                        let mut state = lyrics::fetch(&track).await;

                                        // Error bisa berarti jaringan/LRCLIB sedang
                                        // gangguan: coba lagi sebelum menyerah.
                                        let mut retries = 0;
                                        while matches!(&state, lyrics::LyricsState::Error { .. })
                                            && retries < 2
                                        {
                                            retries += 1;
                                            tokio::time::sleep(Duration::from_secs(2)).await;
                                            if !is_still_current(&current_ref, &cache_key) {
                                                break;
                                            }
                                            state = lyrics::fetch(&track).await;
                                        }

                                        if is_still_current(&current_ref, &cache_key) {
                                            let _ = handle.emit("lyrics:update", state.clone());
                                        }

                                        // Simpan hanya hasil final, bukan error/timeout.
                                        let cacheable = matches!(
                                            state,
                                            lyrics::LyricsState::Ready { .. }
                                                | lyrics::LyricsState::Instrumental
                                                | lyrics::LyricsState::NotFound
                                        );
                                        if cacheable {
                                            if let (Some(k), Ok(mut map)) =
                                                (cache_key, cache_ref.lock())
                                            {
                                                map.insert(k, state);
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
                None => {
                    misses += 1;
                    // Emit idle pertama setelah ~1,5 dtk, lalu ulangi tiap
                    // ~6 dtk supaya frontend yang ter-reload bisa sinkron lagi.
                    let should_emit =
                        misses >= IDLE_MISS_LIMIT && (!idle_announced || misses % 12 == 0);

                    if should_emit {
                        idle_announced = true;
                        last_key = None;
                        if let Ok(mut current) = current_key.lock() {
                            *current = None;
                        }
                        let _ = app.emit("playback:update", playback::PlaybackSnapshot::idle());
                        let _ = app.emit("lyrics:update", lyrics::LyricsState::Idle);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

/// Menu tray: tampil/sembunyi, geser lirik, keluar.
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Tampilkan / Sembunyikan", true, None::<&str>)?;
    let slower = MenuItem::with_id(app, "offset_minus", "Lirik lebih lambat 100 ms", true, None::<&str>)?;
    let faster = MenuItem::with_id(app, "offset_plus", "Lirik lebih cepat 100 ms", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "offset_reset", "Reset geser lirik", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&toggle, &sep1, &slower, &faster, &reset, &sep2, &quit],
    )?;

    // Ikon tray diambil langsung dari bytes supaya tidak bergantung pada
    // `default_window_icon()` yang bisa kosong pada build tertentu.
    let icon = match tauri::image::Image::from_bytes(include_bytes!("../icons/128x128.png")) {
        Ok(icon) => Some(icon),
        Err(error) => {
            eprintln!("ikon tray gagal dimuat: {error}");
            app.default_window_icon().cloned()
        }
    };

    let Some(icon) = icon else {
        eprintln!("ikon tray tidak tersedia, tray dilewati");
        return Ok(());
    };

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Musely")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => toggle_window(app),
            "offset_minus" => {
                let _ = app.emit("offset:adjust", -100i64);
            }
            "offset_plus" => {
                let _ = app.emit("offset:adjust", 100i64);
            }
            "offset_reset" => {
                let _ = app.emit("offset:set", 0i64);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    eprintln!("tray icon siap");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance harus didaftarkan paling awal
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("widget") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("widget") {
                let _ = window.set_always_on_top(true);
                // window-state bisa memulihkan ukuran lama (150 px); paksa
                // tinggi yang dibutuhkan bar kontrol.
                let _ = window.set_size(tauri::LogicalSize::new(380.0, 196.0));
            }

            build_tray(app.handle())?;
            start_playback_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_click_through,
            set_always_on_top,
            app_info,
            playback_control,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_result_for_old_track() {
        let current = Mutex::new(Some("B".to_string()));
        assert!(!is_still_current(&current, &Some("A".to_string())));
        assert!(is_still_current(&current, &Some("B".to_string())));

        *current.lock().unwrap() = None;
        assert!(!is_still_current(&current, &Some("B".to_string())));
    }
}
