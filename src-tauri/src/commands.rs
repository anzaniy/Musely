use tauri::{AppHandle, WebviewWindow};

use crate::playback;

/// Mengaktifkan/mematikan mode klik-tembus (click-through).
#[tauri::command]
pub fn set_click_through(window: WebviewWindow, enabled: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|e| e.to_string())
}

/// Menjamin widget tetap di atas jendela lain.
#[tauri::command]
pub fn set_always_on_top(window: WebviewWindow, enabled: bool) -> Result<(), String> {
    window.set_always_on_top(enabled).map_err(|e| e.to_string())
}

/// Info singkat untuk ditampilkan di UI.
#[tauri::command]
pub fn app_info() -> serde_json::Value {
    serde_json::json!({
        "name": "Musely",
        "version": env!("CARGO_PKG_VERSION"),
        "stage": "gsmtc"
    })
}

/// Play / pause / next / previous lewat GSMTC.
#[tauri::command]
pub fn playback_control(action: String) -> Result<(), String> {
    let parsed = playback::ControlAction::parse(&action)
        .ok_or_else(|| format!("Aksi tidak dikenal: {action}"))?;
    playback::control(parsed)
}

/// Menutup aplikasi sepenuhnya (bukan hanya menyembunyikan window).
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}