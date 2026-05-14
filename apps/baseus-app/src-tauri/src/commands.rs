use tauri::State;
use crate::device::{CommandSender, DeviceCommand, Side};
use crate::settings::{self, Settings};
use baseus_protocol::types::AncMode;

#[tauri::command]
pub fn set_anc_mode(
    mode: String,
    cmd_tx: State<CommandSender>,
) -> Result<(), String> {
    let anc_mode = match mode.as_str() {
        "off" => AncMode::Off,
        "anc" => AncMode::Anc,
        "transparency" => AncMode::Transparency,
        other => return Err(format!("unknown mode: {other}")),
    };
    cmd_tx.send(DeviceCommand::SetAncMode(anc_mode)).map_err(|e| e.to_string())
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
pub fn set_settings(settings: Settings) -> Result<(), String> {
    settings::save(&settings)
}
