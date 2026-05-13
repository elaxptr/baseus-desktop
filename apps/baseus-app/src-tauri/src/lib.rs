mod commands;
mod device;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::connect])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
