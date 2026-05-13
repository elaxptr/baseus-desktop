use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryState {
    pub left_pct:        u8,
    pub right_pct:       u8,
    pub case_pct:        u8,
    pub left_charging:   bool,
    pub right_charging:  bool,
    pub case_charging:   bool,
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
