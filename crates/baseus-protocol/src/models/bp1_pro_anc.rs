use crate::{
    models::DecodeError,
    types::{AncMode, BatteryState, CaseState, DeviceEvent, EqPreset},
    Frame,
};

pub struct Bp1ProAnc;

impl Bp1ProAnc {
    /// Decode a GATT notification frame from the BP1 Pro ANC.
    ///
    /// Wire format confirmed via nRF Connect on BASS BP1 PRO (4A:01:CE:BA:C8:03):
    ///   AA 02 L% L_chg R% R_chg   → battery report (left + right buds)
    ///   AA 30 …                    → ANC off
    ///   AA 32 …                    → Transparency mode
    ///   AA 33 …                    → ANC active
    ///   AA 80 …                    → Case/connection event (partially decoded)
    pub fn decode_frame(frame: &Frame) -> Result<DeviceEvent, DecodeError> {
        match frame.cmd {
            0x02 => Self::decode_battery(&frame.payload),
            0x27 => Self::decode_case(&frame.payload),
            // 0x30 appears as a periodic keepalive in BLE — NOT an ANC state update.
            // ANC acks arrive as AA 34 [type] and are handled in device.rs.
            // 0x32/0x33 kept for safety in case the device ever sends them directly.
            0x32 => Ok(DeviceEvent::AncModeUpdate(AncMode::Transparency)),
            0x33 => Ok(DeviceEvent::AncModeUpdate(AncMode::Anc)),
            // EQ preset ack: AA 43 [preset_byte] or AA 42 [preset_byte] (query response).
            // Confirmed via btsnoop RFCOMM captures — same opcode echoed back by device.
            0x42 | 0x43 => Self::decode_eq_preset(&frame.payload),
            other => Err(DecodeError::UnknownOpcode(other)),
        }
    }

    fn decode_eq_preset(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        let byte = *payload.first().unwrap_or(&0);
        let preset = EqPreset::from_byte(byte).unwrap_or(EqPreset::Balanced);
        Ok(DeviceEvent::EqPresetUpdate(preset))
    }

    fn decode_battery(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        // Confirmed live: AA 02 64 00 64 01 = left 100%, right 100% (both in ear).
        // Frame structure: [left_pct, 0x00, right_pct, 0x01]
        // Bytes 1 and 3 are fixed bud-ID markers (0x00=left, 0x01=right), NOT charging flags.
        // Charging state is not present in this frame; set false until a charging frame is found.
        if payload.len() < 4 {
            return Err(DecodeError::PayloadTooShort {
                opcode: 0x02,
                need: 4,
                got: payload.len(),
            });
        }
        Ok(DeviceEvent::BatteryUpdate(BatteryState {
            left_pct: payload[0],
            left_charging: false,
            right_pct: payload[2],
            right_charging: false,
        }))
    }

    fn decode_case(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        // Confirmed live: AA 27 32 00 = case 50%, not charging.
        // payload[0] = case_pct, payload[1] = charging flag (0x00=no, 0x01=yes).
        if payload.len() < 2 {
            return Err(DecodeError::PayloadTooShort {
                opcode: 0x27,
                need: 2,
                got: payload.len(),
            });
        }
        Ok(DeviceEvent::CaseUpdate(CaseState {
            case_pct: payload[0],
            case_charging: payload[1] != 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;

    fn decode(raw: &[u8]) -> Result<DeviceEvent, DecodeError> {
        Bp1ProAnc::decode_frame(&Frame::decode(raw).unwrap())
    }

    #[test]
    fn battery_frame_decodes_correctly() {
        // Golden: AA 02 64 00 64 01 — captured live, both buds in ear at 100%.
        // Bytes [1] and [3] are bud-ID markers (0x00=left, 0x01=right), not charging flags.
        let ev = decode(&[0xAA, 0x02, 0x64, 0x00, 0x64, 0x01]).unwrap();
        assert_eq!(
            ev,
            DeviceEvent::BatteryUpdate(BatteryState {
                left_pct: 100,
                left_charging: false,
                right_pct: 100,
                right_charging: false,
            })
        );
    }

    #[test]
    fn anc_off_keepalive_is_ignored() {
        // AA 30 00 is a periodic BLE keepalive, not an ANC state notification.
        // ANC off state arrives as AA 34 00 (handled in device.rs, not here).
        assert!(matches!(
            decode(&[0xAA, 0x30, 0x00]),
            Err(DecodeError::UnknownOpcode(0x30))
        ));
    }

    #[test]
    fn anc_transparency_decodes_correctly() {
        // Golden: AA 32 02 FF
        assert_eq!(
            decode(&[0xAA, 0x32, 0x02, 0xFF]).unwrap(),
            DeviceEvent::AncModeUpdate(AncMode::Transparency)
        );
    }

    #[test]
    fn anc_on_decodes_correctly() {
        // Golden: AA 33 01 68
        assert_eq!(
            decode(&[0xAA, 0x33, 0x01, 0x68]).unwrap(),
            DeviceEvent::AncModeUpdate(AncMode::Anc)
        );
    }

    #[test]
    fn battery_too_short_is_error() {
        let frame = Frame { cmd: 0x02, payload: vec![0x64, 0x00, 0x5A] };
        assert!(matches!(
            Bp1ProAnc::decode_frame(&frame),
            Err(DecodeError::PayloadTooShort { opcode: 0x02, need: 4, got: 3 })
        ));
    }

    #[test]
    fn case_frame_decodes_correctly() {
        // Golden: AA 27 32 00 — captured live, case at 50%, not charging.
        let ev = decode(&[0xAA, 0x27, 0x32, 0x00]).unwrap();
        assert_eq!(
            ev,
            DeviceEvent::CaseUpdate(CaseState { case_pct: 50, case_charging: false })
        );
    }

    #[test]
    fn unknown_opcode_is_error() {
        let frame = Frame { cmd: 0x99, payload: vec![] };
        assert!(matches!(
            Bp1ProAnc::decode_frame(&frame),
            Err(DecodeError::UnknownOpcode(0x99))
        ));
    }

    #[test]
    fn eq_set_ack_balanced_decodes() {
        // AA 43 00 — device ack after BA 43 00 (set Balanced)
        let ev = decode(&[0xAA, 0x43, 0x00]).unwrap();
        assert_eq!(ev, DeviceEvent::EqPresetUpdate(EqPreset::Balanced));
    }

    #[test]
    fn eq_set_ack_bass_boost_decodes() {
        // AA 43 01 — device ack after BA 43 01 (set BassBoost)
        let ev = decode(&[0xAA, 0x43, 0x01]).unwrap();
        assert_eq!(ev, DeviceEvent::EqPresetUpdate(EqPreset::BassBoost));
    }

    #[test]
    fn eq_set_ack_voice_decodes() {
        // AA 43 02 — device ack after BA 43 02 (set Voice)
        let ev = decode(&[0xAA, 0x43, 0x02]).unwrap();
        assert_eq!(ev, DeviceEvent::EqPresetUpdate(EqPreset::Voice));
    }

    #[test]
    fn eq_set_ack_clear_decodes() {
        // AA 43 03 — device ack after BA 43 03 (set Clear; value extrapolated)
        let ev = decode(&[0xAA, 0x43, 0x03]).unwrap();
        assert_eq!(ev, DeviceEvent::EqPresetUpdate(EqPreset::Clear));
    }

    #[test]
    fn eq_query_response_decodes() {
        // AA 42 01 — query response (opcode 0x42) returning current preset BassBoost
        let ev = decode(&[0xAA, 0x42, 0x01]).unwrap();
        assert_eq!(ev, DeviceEvent::EqPresetUpdate(EqPreset::BassBoost));
    }

    #[test]
    fn eq_unknown_preset_falls_back_to_balanced() {
        // AA 43 FF — unknown preset byte falls back to Balanced rather than erroring
        let ev = decode(&[0xAA, 0x43, 0xFF]).unwrap();
        assert_eq!(ev, DeviceEvent::EqPresetUpdate(EqPreset::Balanced));
    }
}
