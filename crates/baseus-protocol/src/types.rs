use serde::{Deserialize, Serialize};

/// BLE GATT UUIDs confirmed from live nRF Connect capture on BASS BP1 PRO (4A:01:CE:BA:C8:03).
/// The 02F00000-… UUIDs found in APK static analysis are for a different Bluetrum device variant.
pub mod ble_uuids {
    /// Primary service UUID observed on BASS BP1 PRO via nRF Connect.
    pub const SERVICE: &str = "53527aa4-29f7-ae11-4e74-997334782568";
    /// Write characteristic: app → device commands (WRITE property).
    pub const WRITE:   &str = "ee684b1a-1e9b-ed3e-ee55-f894667e92ac";
    /// Notify characteristic: device → app events, battery, ANC (NOTIFY + READ).
    pub const NOTIFY:  &str = "654b749c-e37f-ae1f-ebab-40ca133e3690";
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryState {
    pub left_pct: u8,
    pub right_pct: u8,
    pub left_charging: bool,
    pub right_charging: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AncMode {
    Off,
    Anc,
    Transparency,
}

/// Events emitted from the device to the app (via Tauri `device-event`).
/// Serialised as `{ "type": "<variant>", "data": <payload> }` for TypeScript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum DeviceEvent {
    BatteryUpdate(BatteryState),
    CaseUpdate(CaseState),
    AncModeUpdate(AncMode),
    Connected,
    Disconnected,
}

/// Case battery state, emitted by AA 80 frames separately from bud battery (AA 02).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseState {
    pub case_pct: u8,
    pub case_charging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseusModel {
    Bp1ProAnc,
}
