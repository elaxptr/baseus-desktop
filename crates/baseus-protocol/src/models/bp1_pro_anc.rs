use crate::{
    models::DecodeError,
    types::{AncMode, BatteryState, CaseState, DeviceEvent},
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
            0x30 => Ok(DeviceEvent::AncModeUpdate(AncMode::Off)),
            0x32 => Ok(DeviceEvent::AncModeUpdate(AncMode::Transparency)),
            0x33 => Ok(DeviceEvent::AncModeUpdate(AncMode::Anc)),
            other => Err(DecodeError::UnknownOpcode(other)),
        }
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
    fn anc_off_decodes_correctly() {
        // Golden: AA 30 00
        assert_eq!(
            decode(&[0xAA, 0x30, 0x00]).unwrap(),
            DeviceEvent::AncModeUpdate(AncMode::Off)
        );
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
}
