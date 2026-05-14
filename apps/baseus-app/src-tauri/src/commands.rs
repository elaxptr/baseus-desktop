use tauri::{AppHandle, Runtime, State};
use tauri_plugin_autostart::ManagerExt;
use crate::device::{CommandSender, DeviceCommand, Side};
use crate::settings::{self, Settings};
use baseus_protocol::types::{AncMode, EqPreset};

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
    let byte = level.unwrap_or(0x68);
    cmd_tx.send(DeviceCommand::SetAncMode(anc_mode, byte)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_eq_preset(preset: u8, cmd_tx: State<CommandSender>) -> Result<(), String> {
    let eq = EqPreset::from_byte(preset)
        .ok_or_else(|| format!("unknown EQ preset: {preset}"))?;
    cmd_tx.send(DeviceCommand::SetEqPreset(eq)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_earbud(side: String, cmd_tx: State<CommandSender>) -> Result<(), String> {
    let s = match side.as_str() {
        "left" => Side::Left,
        "right" => Side::Right,
        other => return Err(format!("unknown side: {other}")),
    };
    cmd_tx.send(DeviceCommand::FindEarbud(s)).map_err(|e| e.to_string())
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
