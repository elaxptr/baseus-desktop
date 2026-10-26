use crate::device::{CommandSender, DeviceCommand, DeviceSnapshot, SharedSnapshot, Side};
use crate::settings::{self, Settings};
use baseus_protocol::types::{AncMode, BaseusModel, EqPreset};
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_autostart::ManagerExt;
#[cfg(not(target_os = "linux"))]
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub fn set_anc_mode(
    mode: String,
    level: Option<u8>,
    cmd_tx: State<CommandSender>,
) -> Result<(), String> {
    let anc_mode = match mode.as_str() {
        "off" => AncMode::Off,
        "anc" => AncMode::Anc,
        "transparency" => AncMode::Transparency,
        other => return Err(format!("unknown mode: {other}")),
    };
    // The level byte is model-specific: a 0x10–0xFF strength on the BP1 Pro, a
    // discrete 1–5 step on the BP1 Ultra (clamped by its encoder). The frontend
    // sends the right scale for the connected model.
    let byte = level.unwrap_or(0x68);
    cmd_tx
        .send(DeviceCommand::SetAncMode(anc_mode, byte))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_eq_preset(preset: u8, cmd_tx: State<CommandSender>) -> Result<(), String> {
    let eq = EqPreset::from_byte(preset).ok_or_else(|| format!("unknown EQ preset: {preset}"))?;
    cmd_tx
        .send(DeviceCommand::SetEqPreset(eq))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_game_mode(enabled: bool, cmd_tx: State<CommandSender>) -> Result<(), String> {
    cmd_tx
        .send(DeviceCommand::SetGameMode(enabled))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_spatial_audio(enabled: bool, cmd_tx: State<CommandSender>) -> Result<(), String> {
    cmd_tx
        .send(DeviceCommand::SetSpatialAudio(enabled))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_wind_reduction(enabled: bool, cmd_tx: State<CommandSender>) -> Result<(), String> {
    cmd_tx
        .send(DeviceCommand::SetWindReduction(enabled))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_bass_boost(
    enabled: bool,
    level: u8,
    cmd_tx: State<CommandSender>,
) -> Result<(), String> {
    cmd_tx
        .send(DeviceCommand::SetBassBoost(enabled, level))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_earbud(side: String, cmd_tx: State<CommandSender>) -> Result<(), String> {
    let s = match side.as_str() {
        "left" => Side::Left,
        "right" => Side::Right,
        other => return Err(format!("unknown side: {other}")),
    };
    cmd_tx
        .send(DeviceCommand::FindEarbud(s))
        .map_err(|e| e.to_string())
}

/// Current connection state and connected model, for a frontend that subscribed to the
/// `connection-state` / `model-info` events after they were emitted.
#[tauri::command]
pub fn get_device_snapshot(snapshot: State<SharedSnapshot>) -> DeviceSnapshot {
    snapshot.lock().map(|s| s.clone()).unwrap_or_default()
}

/// Return the ANC modes supported by a given model name.
/// The frontend calls this after receiving a `model-info` event to know which modes to show.
#[tauri::command]
pub fn get_supported_anc_modes(model_name: String) -> Vec<String> {
    let model = BaseusModel::all()
        .iter()
        .find(|m| m.display_name() == model_name)
        .copied();

    let Some(m) = model else {
        // Fallback to BP1 defaults for unknown models.
        return vec![
            "off".to_string(),
            "anc".to_string(),
            "transparency".to_string(),
        ];
    };

    AncMode::supported_by(m)
        .iter()
        .map(|mode| {
            serde_json::to_value(mode)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        })
        .collect()
}

/// Silent background check — returns version string if an update is available, None otherwise.
#[cfg(not(target_os = "linux"))]
pub(crate) async fn check_update_silent(app: &AppHandle) -> Option<String> {
    app.updater().ok()?.check().await.ok()?.map(|u| u.version)
}

/// Linux builds carry no updater plugin — updates come from the distro package,
/// so the in-app check reports "up to date" and the install is a no-op.
#[cfg(target_os = "linux")]
#[tauri::command]
pub async fn check_for_update(_app: AppHandle) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    Ok(updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .map(|u| u.version))
}

#[cfg(target_os = "linux")]
#[tauri::command]
pub async fn install_update(_app: AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app.restart();
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings() -> Settings {
    settings::load()
}

#[tauri::command]
pub fn set_settings<R: Runtime>(app: AppHandle<R>, settings: Settings) -> Result<(), String> {
    settings::save(&settings)?;
    if settings.launch_at_login {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}
