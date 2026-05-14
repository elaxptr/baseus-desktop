mod commands;
mod device;
mod settings;
mod tray;

use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (cmd_tx, cmd_rx) = device::command_channel();

    tauri::Builder::default()
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_anc_mode,
            commands::set_eq_preset,
            commands::find_earbud,
            commands::get_settings,
            commands::set_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
