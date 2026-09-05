mod commands;
mod device;
mod settings;
mod tray;

#[cfg(not(target_os = "linux"))]
use tauri::Emitter;
use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (cmd_tx, cmd_rx) = device::command_channel();

    #[allow(unused_mut)]
    // Single-instance must be registered first. A second launch — a desktop entry, a
    // keybind — then surfaces the running window instead of starting a rival process
    // that would open its own GATT connection to the same earbuds. It is also the only
    // way back to a window hidden with `hide()`: that unmaps it, so the compositor has
    // nothing left to focus.
    let mut builder =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));

    // The bundled updater ships Windows/macOS artifacts only; on Linux the app is
    // installed and updated through the distro, and initialising the plugin without
    // a `plugins.updater` config aborts startup outright.
    #[cfg(not(target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .manage(cmd_tx)
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(device::run_loop(handle, cmd_rx));

            // Background update check — silent, fires 10s after startup.
            #[cfg(not(target_os = "linux"))]
            {
                let handle2 = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    if let Some(version) = commands::check_update_silent(&handle2).await {
                        let _ = handle2.emit("update-available", version);
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_anc_mode,
            commands::set_eq_preset,
            commands::set_game_mode,
            commands::find_earbud,
            commands::get_settings,
            commands::set_settings,
            commands::get_supported_anc_modes,
            commands::check_for_update,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
