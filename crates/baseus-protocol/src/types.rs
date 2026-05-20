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

/// Single-unit battery for over-ear headphones (no L/R split, no case).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadphoneBattery {
    pub pct: u8,
    pub charging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AncMode {
    // BP1 Pro ANC — verified
    Off,
    Anc,
    Transparency,
    // Inspire XH1 adaptive modes — APK-extracted, unverified wire format.
    // Mode IDs from NoiseTypeBean: Commute=0x08, Outdoor=0x09, Indoor=0x0A.
    AdaptiveCommute,
    AdaptiveOutdoor,
    AdaptiveIndoor,
    AdaptiveSelf,
}

impl AncMode {
    /// ANC modes supported by a given model (for UI filtering).
    pub fn supported_by(model: BaseusModel) -> &'static [AncMode] {
        match model {
            BaseusModel::Bp1ProAnc => &[AncMode::Off, AncMode::Anc, AncMode::Transparency],
            BaseusModel::InspireXh1 => &[
                AncMode::AdaptiveSelf,
                AncMode::AdaptiveIndoor,
                AncMode::AdaptiveOutdoor,
                AncMode::AdaptiveCommute,
            ],
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
    HeadphoneBatteryUpdate(HeadphoneBattery),
    CaseUpdate(CaseState),
    AncModeUpdate(AncMode),
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

/// Whether a model's support is owner-verified or APK-derived and untested.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Verified,
    Experimental,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseusModel {
    Bp1ProAnc,
    InspireXh1,
}

impl BaseusModel {
    pub fn all() -> &'static [BaseusModel] {
        &[BaseusModel::Bp1ProAnc, BaseusModel::InspireXh1]
    }

    pub fn status(self) -> ModelStatus {
        match self {
            BaseusModel::Bp1ProAnc => ModelStatus::Verified,
            BaseusModel::InspireXh1 => ModelStatus::Experimental,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            BaseusModel::Bp1ProAnc => "Bass BP1 Pro ANC",
            BaseusModel::InspireXh1 => "Inspire XH1",
        }
    }

    /// BLE advertising name(s) used to identify this device during scan.
    pub fn advertising_names(self) -> &'static [&'static str] {
        match self {
            BaseusModel::Bp1ProAnc => &["Bass BP1 Pro"],
            // APK-EXTRACTED: exact string key from EarFunctionManager2 dispatch.
            BaseusModel::InspireXh1 => &["Baseus Inspire XH1"],
        }
    }

    /// GATT (notify_uuid, write_uuid) for BLE control.
    /// BP1 values confirmed via nRF Connect. XH1 values APK-extracted, unverified.
    pub fn gatt_uuids(self) -> (&'static str, &'static str) {
        match self {
            BaseusModel::Bp1ProAnc => (
                "654b749c-e37f-ae1f-ebab-40ca133e3690",
                "ee684b1a-1e9b-ed3e-ee55-f894667e92ac",
            ),
            // APK-EXTRACTED, UNVERIFIED — 0000ae0x family from classes6.dex.
            // Alternative: 0000fae0-fae2 family from classes4.dex.
            BaseusModel::InspireXh1 => (
                "0000ae02-0000-1000-8000-00805f9b34fb",
                "0000ae01-0000-1000-8000-00805f9b34fb",
            ),
        }
    }

    /// Look up the model from a BLE advertising name seen during a scan.
    pub fn from_advertising_name(name: &str) -> Option<Self> {
        Self::all()
            .iter()
            .find(|m| m.advertising_names().contains(&name))
            .copied()
    }
}
