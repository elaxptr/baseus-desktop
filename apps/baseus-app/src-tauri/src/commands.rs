use crate::device::{CommandSender, DeviceCommand, Side};
use crate::settings::{self, Settings};
use baseus_protocol::types::{AncMode, BaseusModel, EqPreset};
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_autostart::ManagerExt;

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
        // XH1 adaptive modes — command is accepted but will no-op in execute_command
        // (wire format unverified). Logged server-side for debugging.
        "adaptive_self" => AncMode::AdaptiveSelf,
        "adaptive_indoor" => AncMode::AdaptiveIndoor,
        "adaptive_outdoor" => AncMode::AdaptiveOutdoor,
        "adaptive_commute" => AncMode::AdaptiveCommute,
        other => return Err(format!("unknown mode: {other}")),
    };
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
