mod commands;
mod lyrics;
mod playback;
mod tray;
mod watcher;

use tauri::Manager;

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

            tray::build_tray(app.handle())?;
            watcher::start_playback_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_click_through,
            commands::set_always_on_top,
            commands::app_info,
            commands::playback_control,
            commands::quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}