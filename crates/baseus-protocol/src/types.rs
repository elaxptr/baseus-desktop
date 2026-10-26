use serde::{Deserialize, Serialize};

use crate::framing::WireFormat;

pub mod ble_uuids {
    /// BP1 Pro ANC — confirmed via nRF Connect on physical unit.
    /// The BP1 Ultra exposes the very same service and characteristics.
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
            BaseusModel::Bp1ProAnc | BaseusModel::Bp1Ultra => {
                &[AncMode::Off, AncMode::Anc, AncMode::Transparency]
            }
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
    /// Discrete ANC strength reported by the device (BP1 Ultra: 1–5).
    AncLevelUpdate(u8),
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
    Bp1Ultra,
}

impl BaseusModel {
    pub fn all() -> &'static [BaseusModel] {
        &[BaseusModel::Bp1ProAnc, BaseusModel::Bp1Ultra]
    }

    pub fn display_name(self) -> &'static str {
        match self {
            BaseusModel::Bp1ProAnc => "Bass BP1 Pro ANC",
            BaseusModel::Bp1Ultra => "Bass BP1 Ultra",
        }
    }

    /// BLE advertising name(s) used to identify this device during scan.
    /// Includes short-form aliases for devices that omit the model suffix.
    ///
    /// Treat these as a hint, not as the primary key: the name travels in the scan
    /// response, so BlueZ reports it only intermittently and `local_name` is often
    /// `None` for a device that is plainly there. Match on `service_uuid` first.
    pub fn advertising_names(self) -> &'static [&'static str] {
        match self {
            BaseusModel::Bp1ProAnc => &["Bass BP1 Pro"],
            // Deliberately empty. The BP1 Ultra earbuds also advertise a Google Fast
            // Pair identity under the same "BP1 Ultra" name, without the control
            // service, so a name match could latch onto the wrong peripheral. The
            // Ultra is found through the shared service UUID instead and identified
            // by its wire format (see `from_wire_format`).
            BaseusModel::Bp1Ultra => &[],
        }
    }

    /// GATT service UUID advertised by this model. Unlike the advertising name,
    /// this is always present in the advertisement payload, which makes it the only
    /// reliable way to spot the device on BlueZ (see `advertising_names`).
    ///
    /// The BP1 Pro and BP1 Ultra share this UUID, so it finds the device but cannot
    /// tell the two apart; [`BaseusModel::from_wire_format`] does that from the
    /// first notification.
    pub fn service_uuid(self) -> &'static str {
        match self {
            BaseusModel::Bp1ProAnc | BaseusModel::Bp1Ultra => ble_uuids::SERVICE,
        }
    }

    /// GATT (notify_uuid, write_uuid) for BLE control.
    /// Confirmed via nRF Connect (BP1 Pro) and a WinRT GATT dump (BP1 Ultra).
    pub fn gatt_uuids(self) -> (&'static str, &'static str) {
        match self {
            BaseusModel::Bp1ProAnc | BaseusModel::Bp1Ultra => (ble_uuids::NOTIFY, ble_uuids::WRITE),
        }
    }

    /// Frame envelope this model speaks.
    pub fn wire_format(self) -> WireFormat {
        match self {
            BaseusModel::Bp1ProAnc => WireFormat::Plain,
            BaseusModel::Bp1Ultra => WireFormat::Crc,
        }
    }

    /// Model that speaks a given wire format. The BP1 Pro and BP1 Ultra advertise the
    /// same GATT service, so a scan cannot tell them apart, but their frames can: the
    /// Ultra wraps every notification in a `78 9C` CRC envelope (and pushes its
    /// battery report right after the notify subscription), while the Pro sends bare
    /// `AA` frames.
    pub fn from_wire_format(format: WireFormat) -> Self {
        match format {
            WireFormat::Plain => BaseusModel::Bp1ProAnc,
            WireFormat::Crc => BaseusModel::Bp1Ultra,
        }
    }

    /// Selectable ANC strength steps, or `None` when strength is a continuous byte.
    /// The BP1 Ultra exposes five discrete levels (`34 01 01`…`34 01 05`).
    pub fn anc_levels(self) -> Option<u8> {
        match self {
            BaseusModel::Bp1ProAnc => None,
            BaseusModel::Bp1Ultra => Some(5),
        }
    }

    /// Noise-control modes that take a strength value. The BP1 Ultra's transparency
    /// has no strength, only "full" and "voice" variants.
    pub fn strength_modes(self) -> &'static [AncMode] {
        match self {
            BaseusModel::Bp1ProAnc => &[AncMode::Anc, AncMode::Transparency],
            BaseusModel::Bp1Ultra => &[AncMode::Anc],
        }
    }

    /// Whether the find-my-earbuds command is known for this model. The BP1 Ultra
    /// opcode was not captured (the official app gates the feature behind a login).
    pub fn supports_find_earbud(self) -> bool {
        match self {
            BaseusModel::Bp1ProAnc => true,
            BaseusModel::Bp1Ultra => false,
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
