use serde::{Deserialize, Serialize};

/// Bluetrum CCSDK BLE GATT UUIDs for the BP1 Pro ANC (and other Bluetrum-based Baseus earbuds).
/// Source: static analysis of com.baseus.intelligent APK, classes2.dex.
pub mod ble_uuids {
    pub const SERVICE:  &str = "02F00000-0000-0000-0000-00000000FE00";
    /// Write characteristic: app → device commands.
    pub const WRITE:    &str = "02F00000-0000-0000-0000-00000000FF01";
    /// Notify characteristic: device → app events (battery, ANC, etc.).
    pub const NOTIFY:   &str = "02F00000-0000-0000-0000-00000000FF02";
    /// Extra characteristic, purpose TBD (possibly OTA or config).
    pub const EXTRA:    &str = "02F00000-0000-0000-0000-00000000FF05";
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryState {
    pub left_pct: u8,
    pub right_pct: u8,
    pub case_pct: u8,
    pub left_charging: bool,
    pub right_charging: bool,
    pub case_charging: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AncMode {
    Off,
    Anc,
    Transparency,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceEvent {
    BatteryUpdate(BatteryState),
    AncModeUpdate(AncMode),
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseusModel {
    Bp1ProAnc,
}
