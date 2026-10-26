use baseus_protocol::types::{AncMode, DeviceEvent};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

use crate::device::{send_command, DeviceCommand, SharedSnapshot};

const TRAY_ID: &str = "main";
/// BP1 Pro strength byte used when no level was set this connection (as `set_anc_mode`).
const DEFAULT_LEVEL_BYTE: u8 = 0x68;

const MODES: [(AncMode, &str, &str); 3] = [
    (AncMode::Off, "anc:off", "Noise control off"),
    (AncMode::Anc, "anc:anc", "Noise cancellation"),
    (AncMode::Transparency, "anc:transparency", "Transparency"),
];

/// Menu items that follow the device; updated by [`refresh`].
struct TrayMenu {
    status: MenuItem<Wry>,
    modes: Vec<(AncMode, CheckMenuItem<Wry>)>,
}

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
    // unreachable — with no way to quit either. It also carries the battery line, since
    // appindicator shows no tooltip.
    let status = MenuItem::with_id(app, "status", "Not connected", false, None::<&str>)?;
    let modes = MODES
        .iter()
        .map(|&(mode, id, label)| {
            CheckMenuItem::with_id(app, id, label, false, false, None::<&str>).map(|i| (mode, i))
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let show = MenuItem::with_id(app, "show", "Show Baseus Desktop", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = || PredefinedMenuItem::separator(app);

    let menu = Menu::with_items(app, &[&status, &separator()?])?;
    for (_, item) in &modes {
        menu.append(item)?;
    }
    menu.append_items(&[&separator()?, &show, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
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
            id => {
                if let Some(&(mode, ..)) = MODES.iter().find(|(_, mode_id, _)| *mode_id == id) {
                    set_mode(app, mode);
                }
            }
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

    app.manage(TrayMenu { status, modes });
    Ok(())
}

/// Switch noise control from the tray at the strength last used this connection.
fn set_mode(app: &AppHandle, mode: AncMode) {
    // Discrete-level models (BP1 Ultra) fall back to their top step.
    let level = app
        .state::<SharedSnapshot>()
        .lock()
        .ok()
        .and_then(|s| s.anc_level.or(s.model.as_ref()?.anc_levels))
        .unwrap_or(DEFAULT_LEVEL_BYTE);
    if let Err(e) = send_command(app, DeviceCommand::SetAncMode(mode, level)) {
        tracing::warn!("tray mode change failed: {e}");
    }
    // Clicking toggled the check mark; put it back until the device confirms.
    refresh(app);
}

/// Mirror the connection, battery levels and noise-control mode into the tray.
pub fn refresh(app: &AppHandle) {
    let Some(menu) = app.try_state::<TrayMenu>() else {
        return;
    };
    let (summary, connected, mode) = match app.state::<SharedSnapshot>().lock() {
        Ok(s) if s.state == "connected" => {
            let mut mode = None;
            let mut parts = vec![s.model.as_ref().map_or("Connected", |m| m.name).to_string()];
            for event in &s.events {
                match event {
                    DeviceEvent::BatteryUpdate(b) => {
                        parts.push(format!("L {} · R {}", pct(b.left_pct), pct(b.right_pct)))
                    }
                    DeviceEvent::CaseUpdate(c) => parts.push(format!("Case {}", pct(c.case_pct))),
                    DeviceEvent::AncModeUpdate(m) => mode = Some(*m),
                    _ => {}
                }
            }
            (parts.join(" · "), true, mode)
        }
        _ => ("Not connected".to_string(), false, None),
    };

    let _ = menu.status.set_text(&summary);
    for (item_mode, item) in &menu.modes {
        let _ = item.set_enabled(connected);
        let _ = item.set_checked(mode == Some(*item_mode));
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(format!("Baseus Desktop\n{summary}")));
    }
}

/// 0 % reads as unknown, as in the low-battery alerts.
fn pct(value: u8) -> String {
    if value == 0 {
        "–".into()
    } else {
        format!("{value}%")
    }
}
