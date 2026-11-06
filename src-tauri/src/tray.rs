use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

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

/// Menu tray: tampil/sembunyi, geser lirik, keluar.
pub(crate) fn build_tray(app: &AppHandle) -> tauri::Result<()> {
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