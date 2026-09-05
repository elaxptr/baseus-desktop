use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    // The menu is what makes the app reachable on Linux: the tray there is backed by
    // libayatana-appindicator, which exposes a menu and never delivers click events, so
    // `on_tray_icon_event` below never fires and a window hidden at startup would be
    // unreachable — with no way to quit either.
    let show = MenuItem::with_id(app, "show", "Show Baseus Desktop", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?
                .clone(),
        )
        .tooltip("Baseus Desktop")
        .menu(&menu)
        // Keep left-click opening the window on Windows/macOS; the menu stays on right-click.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
