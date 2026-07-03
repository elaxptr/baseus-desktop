use serde::{Deserialize, Serialize};

pub mod ble_uuids {
    /// BP1 Pro ANC — confirmed via nRF Connect on physical unit.
    pub const SERVICE: &str = "53527aa4-29f7-ae11-4e74-997334782568";
    pub const WRITE: &str = "ee684b1a-1e9b-ed3e-ee55-f894667e92ac";
    pub const NOTIFY: &str = "654b749c-e37f-ae1f-ebab-40ca133e3690";
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryState {
    pub left_pct: u8,
    pub right_pct: u8,
    pub left_charging: bool,
    pub right_charging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AncMode {
    // BP1 Pro ANC — verified on hardware.
    Off,
    Anc,
    Transparency,
}

impl AncMode {
    /// ANC modes supported by a given model (for UI filtering).
    pub fn supported_by(model: BaseusModel) -> &'static [AncMode] {
        match model {
            BaseusModel::Bp1ProAnc => &[AncMode::Off, AncMode::Anc, AncMode::Transparency],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqPreset {
    Balanced = 0,
    BassBoost = 1,
    Voice = 2,
    Clear = 3,
}

impl EqPreset {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Balanced),
            1 => Some(Self::BassBoost),
            2 => Some(Self::Voice),
            3 => Some(Self::Clear),
            _ => None,
        }
    }

    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WearState {
    pub left_in_ear: bool,
    pub right_in_ear: bool,
}

/// Events emitted from the device to the app (via Tauri `device-event`).
/// Serialised as `{ "type": "<variant>", "data": <payload> }` for TypeScript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum DeviceEvent {
    BatteryUpdate(BatteryState),
    CaseUpdate(CaseState),
    AncModeUpdate(AncMode),
    /// Game/low-latency mode — independent toggle, not a mutually-exclusive ANC state.
    GameModeUpdate(bool),
    WearUpdate(WearState),
    EqPresetUpdate(EqPreset),
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseState {
    pub case_pct: u8,
    pub case_charging: bool,
}

/// Registry of supported Baseus models.
///
/// Only hardware-verified models live here. The enum is intentionally kept as a
/// registry (rather than flattened to BP1-only) so future owner-contributed,
/// verified models can be added without reworking the dispatch structure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseusModel {
    Bp1ProAnc,
}

impl BaseusModel {
    pub fn all() -> &'static [BaseusModel] {
        &[BaseusModel::Bp1ProAnc]
    }

    pub fn display_name(self) -> &'static str {
        match self {
            BaseusModel::Bp1ProAnc => "Bass BP1 Pro ANC",
        }
    }

    /// BLE advertising name(s) used to identify this device during scan.
    /// Includes short-form aliases for devices that omit the model suffix.
    pub fn advertising_names(self) -> &'static [&'static str] {
        match self {
            BaseusModel::Bp1ProAnc => &["Bass BP1 Pro"],
        }
    }

    /// GATT (notify_uuid, write_uuid) for BLE control.
    /// Confirmed via nRF Connect on a physical unit.
    pub fn gatt_uuids(self) -> (&'static str, &'static str) {
        match self {
            BaseusModel::Bp1ProAnc => (ble_uuids::NOTIFY, ble_uuids::WRITE),
        }
    }

    /// Look up the model from a BLE advertising name seen during a scan.
    /// Matching is case-insensitive to handle firmware variations.
    pub fn from_advertising_name(name: &str) -> Option<Self> {
        Self::all()
            .iter()
            .find(|m| {
                m.advertising_names()
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(name))
            })
            .copied()
    }
}
