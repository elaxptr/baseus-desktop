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
            0x30 => Ok(DeviceEvent::AncModeUpdate(AncMode::Off)),
            0x32 => Ok(DeviceEvent::AncModeUpdate(AncMode::Transparency)),
            0x33 => Ok(DeviceEvent::AncModeUpdate(AncMode::Anc)),
            0x80 => Self::decode_case(&frame.payload),
            other => Err(DecodeError::UnknownOpcode(other)),
        }
    }

    fn decode_battery(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        // payload: [left_pct, left_charging, right_pct, right_charging]
        // Confirmed: AA 02 64 00 5A 01 → left=100% not charging, right=90% charging
        // Case battery arrives via AA 80 frames decoded separately.
        if payload.len() < 4 {
            return Err(DecodeError::PayloadTooShort {
                opcode: 0x02,
                need: 4,
                got: payload.len(),
            });
        }
        Ok(DeviceEvent::BatteryUpdate(BatteryState {
            left_pct: payload[0],
            left_charging: payload[1] != 0,
            right_pct: payload[2],
            right_charging: payload[3] != 0,
        }))
    }

    fn decode_case(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        // Observed: AA 80 01 4A 01 A8 EF BF A9
        // payload[0] = sub-type (0x01 = case-close event), payload[1] = case battery %
        // Trailing bytes not decoded yet; 0x4A = 74 matches a plausible charge level.
        if payload.len() < 2 {
            return Err(DecodeError::PayloadTooShort {
                opcode: 0x80,
                need: 2,
                got: payload.len(),
            });
        }
        Ok(DeviceEvent::CaseUpdate(CaseState {
            case_pct: payload[1],
            case_charging: payload[0] != 0,
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
        // Golden: AA 02 64 00 5A 01 — confirmed against Baseus app display
        // left=100% not charging, right=90% charging
        let ev = decode(&[0xAA, 0x02, 0x64, 0x00, 0x5A, 0x01]).unwrap();
        assert_eq!(
            ev,
            DeviceEvent::BatteryUpdate(BatteryState {
                left_pct: 100,
                left_charging: false,
                right_pct: 90,
                right_charging: true,
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
        // Observed: AA 80 01 4A 01 A8 EF BF A9
        // payload[0]=0x01 (sub-type), payload[1]=0x4A=74 (case battery %)
        let ev = decode(&[0xAA, 0x80, 0x01, 0x4A, 0x01, 0xA8, 0xEF, 0xBF, 0xA9]).unwrap();
        assert_eq!(
            ev,
            DeviceEvent::CaseUpdate(CaseState { case_pct: 74, case_charging: true })
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
