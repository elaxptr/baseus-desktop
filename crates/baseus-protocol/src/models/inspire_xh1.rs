//! Baseus Inspire XH1 — APK-EXTRACTED, UNVERIFIED.
//!
//! All opcodes and frame shapes below are derived from static analysis of the Baseus
//! Android APK (bytecode-dump2.txt). No physical XH1 device has been used to verify them.
//!
//! Sources:
//!   - Feature opcode list: EarFunctionManager2.f("Baseus Inspire XH1") → lines 4079–4100
//!   - Query opcode list: EarFunctionManager2.i("Baseus Inspire XH1") → lines 4163–4179
//!   - ANC mode IDs: NoiseTypeBean constructors → lines 5699–5751
//!   - BLE UUIDs: apk-analysis.log classes6.dex (0000ae0x family)
//!
//! To add verified captures and promote this to Verified status, see:
//!   docs/protocol/inspire-xh1.md  and  docs/re-methodology.md

use crate::{
    models::DecodeError,
    types::{AncMode, DeviceEvent, HeadphoneBattery},
    Frame,
};

pub struct InspireXh1;

impl InspireXh1 {
    /// Decode a GATT notification frame from the Inspire XH1.
    ///
    /// Known / candidate opcodes (APK-extracted):
    ///   AA 02 …   → battery report (assumed same as BP1; layout unverified)
    ///   AA 68 …   → ANC mode notification (candidate opcode 0x68 from EarFunctionManager2)
    pub fn decode_frame(frame: &Frame) -> Result<DeviceEvent, DecodeError> {
        match frame.cmd {
            // Battery: assumed same framing as BP1 Pro ANC (AA 02 …).
            // APK-EXTRACTED, UNVERIFIED.
            0x02 => Self::decode_battery(&frame.payload),
            // ANC notification: opcode 0x68 appears in both the feature list and query list
            // for XH1 in EarFunctionManager2. Payload byte is the noise mode ID.
            // APK-EXTRACTED, UNVERIFIED.
            0x68 => Self::decode_anc(&frame.payload),
            other => Err(DecodeError::UnknownOpcode(other)),
        }
    }

    fn decode_battery(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        // Hypothesis A: AA 02 [pct: u8] [charging_flag: u8]
        // XH1 is an over-ear headphone — no L/R bud split, no case battery.
        // APK-EXTRACTED, UNVERIFIED.
        if payload.is_empty() {
            return Err(DecodeError::PayloadTooShort {
                opcode: 0x02,
                need: 1,
                got: 0,
            });
        }
        Ok(DeviceEvent::HeadphoneBatteryUpdate(HeadphoneBattery {
            pct: payload[0],
            charging: payload.get(1).copied().unwrap_or(0) != 0,
        }))
    }

    fn decode_anc(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        // Mode IDs from NoiseTypeBean constructor (EarFunctionManager2, lines 5699–5751):
        //   Commute = 0x08 (8), Outdoor = 0x09 (9), Indoor = 0x0A (10)
        //   Self and Off wire bytes are unknown — they fall through to AdaptiveSelf.
        // APK-EXTRACTED, UNVERIFIED.
        let mode_byte = payload.first().copied().unwrap_or(0);
        let mode = match mode_byte {
            0x08 => AncMode::AdaptiveCommute,
            0x09 => AncMode::AdaptiveOutdoor,
            0x0A => AncMode::AdaptiveIndoor,
            _ => AncMode::AdaptiveSelf,
        };
        Ok(DeviceEvent::AncModeUpdate(mode))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;

    fn decode(raw: &[u8]) -> Result<DeviceEvent, DecodeError> {
        InspireXh1::decode_frame(&Frame::decode(raw).unwrap())
    }

    // These tests assert that the APK-derived decoder logic is internally consistent.
    // They do NOT prove the protocol is correct — they prove the code matches the doc.

    #[test]
    fn battery_single_byte_decodes() {
        // Hypothesis A: AA 02 [pct] [charging_flag]
        let ev = decode(&[0xAA, 0x02, 0x50, 0x00]).unwrap();
        assert_eq!(
            ev,
            DeviceEvent::HeadphoneBatteryUpdate(HeadphoneBattery {
                pct: 80,
                charging: false,
            })
        );
    }

    #[test]
    fn battery_charging_flag_set() {
        let ev = decode(&[0xAA, 0x02, 0x32, 0x01]).unwrap();
        assert_eq!(
            ev,
            DeviceEvent::HeadphoneBatteryUpdate(HeadphoneBattery {
                pct: 50,
                charging: true,
            })
        );
    }

    #[test]
    fn battery_too_short_is_error() {
        let frame = Frame {
            cmd: 0x02,
            payload: vec![],
        };
        assert!(matches!(
            InspireXh1::decode_frame(&frame),
            Err(DecodeError::PayloadTooShort {
                opcode: 0x02,
                need: 1,
                got: 0
            })
        ));
    }

    #[test]
    fn anc_indoor_decodes() {
        // AA 68 0A — Indoor mode (NoiseTypeBean ID 0x0A)
        let ev = decode(&[0xAA, 0x68, 0x0A]).unwrap();
        assert_eq!(ev, DeviceEvent::AncModeUpdate(AncMode::AdaptiveIndoor));
    }

    #[test]
    fn anc_outdoor_decodes() {
        // AA 68 09 — Outdoor mode (NoiseTypeBean ID 0x09)
        let ev = decode(&[0xAA, 0x68, 0x09]).unwrap();
        assert_eq!(ev, DeviceEvent::AncModeUpdate(AncMode::AdaptiveOutdoor));
    }

    #[test]
    fn anc_commute_decodes() {
        // AA 68 08 — Commute mode (NoiseTypeBean ID 0x08)
        let ev = decode(&[0xAA, 0x68, 0x08]).unwrap();
        assert_eq!(ev, DeviceEvent::AncModeUpdate(AncMode::AdaptiveCommute));
    }

    #[test]
    fn anc_unknown_falls_back_to_self() {
        // Any other mode byte (Self, Off, Custom) falls back to AdaptiveSelf.
        let ev = decode(&[0xAA, 0x68, 0x00]).unwrap();
        assert_eq!(ev, DeviceEvent::AncModeUpdate(AncMode::AdaptiveSelf));
    }

    #[test]
    fn unknown_opcode_is_error() {
        let frame = Frame {
            cmd: 0x99,
            payload: vec![],
        };
        assert!(matches!(
            InspireXh1::decode_frame(&frame),
            Err(DecodeError::UnknownOpcode(0x99))
        ));
    }

    // This test requires a physical XH1 to provide a real notification capture.
    // It exists as a placeholder — fill in the raw bytes and remove `#[ignore]` once confirmed.
    #[test]
    #[ignore = "requires XH1 hardware — update raw bytes from a real nRF Connect capture"]
    fn real_battery_capture_decodes() {
        // Replace 0xAA, 0x02, 0x??, 0x?? with an actual capture.
        let ev = decode(&[0xAA, 0x02, 0x64, 0x00]).unwrap();
        assert!(matches!(
            ev,
            DeviceEvent::HeadphoneBatteryUpdate(HeadphoneBattery { pct: 100, .. })
        ));
    }
}
