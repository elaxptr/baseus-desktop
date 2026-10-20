//! Baseus Bass BP1 Ultra (firmware 1.0.26).
//!
//! Same GATT service as the BP1 Pro ANC, but every frame uses the CRC envelope in
//! [`crate::framing::crc`], and several opcodes changed meaning (`0x43` is spatial
//! audio here, not EQ). Every opcode below was captured from the official Baseus app
//! (Android HCI snoop over SPP) and then replayed over BLE from a PC against a
//! physical unit; the full table lives in `docs/protocol/bp1-ultra.md`.

use crate::{
    framing::crc::{self, Kind},
    models::DecodeError,
    types::{AncMode, BatteryState, CaseState, DeviceEvent, EqPreset},
    Frame,
};

pub struct Bp1Ultra;

/// Opcodes used by this model.
pub mod op {
    /// Battery report (L/R), also pushed unprompted right after subscribing.
    pub const BATTERY: u8 = 0x02;
    /// Low-latency mode state: `23 ON`.
    pub const LOW_LATENCY_STATE: u8 = 0x23;
    /// Set low-latency mode: `24 ON`, acked with `24 01`.
    pub const LOW_LATENCY_SET: u8 = 0x24;
    /// Case battery report: `27 PCT CHARGING`.
    pub const CASE: u8 = 0x27;
    /// Active EQ preset index: `30 IDX`.
    pub const EQ_STATE: u8 = 0x30;
    /// Apply an EQ curve: `31 IDX` + 8 filters, acked with `31 01`.
    pub const EQ_SET: u8 = 0x31;
    /// Noise-control state: `33 MODE ANC_SELECTION TRANSPARENCY_SELECTION CUSTOM_LEVEL`.
    pub const ANC_STATE: u8 = 0x33;
    /// Set noise control: `34 MODE PARAM`, acked with a flat `34 01`.
    pub const ANC_SET: u8 = 0x34;
}

/// `MODE` byte of the noise-control opcodes.
const MODE_OFF: u8 = 0x00;
const MODE_ANC: u8 = 0x01;
const MODE_TRANSPARENCY: u8 = 0x02;

/// `PARAM` for Off.
const PARAM_NONE: u8 = 0xFF;
/// `PARAM` for "full" transparency (`0x02` is the voice-focused variant).
const TRANSPARENCY_FULL: u8 = 0x01;
/// The app's "Customizable" ANC strength runs over discrete levels 1–5. The scene
/// presets live above that range (`0x65` indoor, `0x66` outdoor, `0x67` commuting,
/// `0x6C` traveling, `0x68` adaptive).
pub const ANC_LEVEL_MIN: u8 = 1;
pub const ANC_LEVEL_MAX: u8 = 5;

/// EQ curves for the presets the desktop app exposes, byte for byte as the official
/// app sends them after the `31` opcode: the preset index, then 8 filters of four
/// little-endian u16 each (frequency Hz, gain, Q, filter type). The device does not
/// store curves for these presets — the app streams the whole curve every time.
const EQ_BASEUS_CLASSIC: [u8; 65] = [
    0x00, 0x68, 0x10, 0x46, 0x00, 0x0A, 0x00, 0x01, 0x00, 0xE0, 0x2E, 0x18, 0x01, 0x0A, 0x00, 0x01,
    0x00, 0x7E, 0x00, 0x46, 0x00, 0x0A, 0x00, 0x01, 0x00, 0x40, 0x01, 0x12, 0x00, 0x06, 0x00, 0x01,
    0x00, 0x28, 0x0A, 0x36, 0x01, 0x08, 0x00, 0x01, 0x00, 0x6C, 0x02, 0x69, 0x00, 0x0A, 0x00, 0x01,
    0x00, 0x70, 0x17, 0xB4, 0x00, 0x10, 0x00, 0x01, 0x00, 0x18, 0x06, 0x8E, 0x00, 0x0A, 0x00, 0x01,
    0x00,
];
const EQ_POWERFUL_BASS: [u8; 65] = [
    0x01, 0x68, 0x10, 0x46, 0x00, 0x0A, 0x00, 0x01, 0x00, 0xE0, 0x2E, 0x18, 0x01, 0x0A, 0x00, 0x01,
    0x00, 0x4C, 0x00, 0xAA, 0x00, 0x08, 0x00, 0x01, 0x00, 0xDC, 0x00, 0x12, 0x00, 0x04, 0x00, 0x01,
    0x00, 0x28, 0x0A, 0x36, 0x01, 0x08, 0x00, 0x01, 0x00, 0x6C, 0x02, 0x55, 0x00, 0x08, 0x00, 0x01,
    0x00, 0x70, 0x17, 0xB4, 0x00, 0x10, 0x00, 0x01, 0x00, 0x18, 0x06, 0x8E, 0x00, 0x0A, 0x00, 0x01,
    0x00,
];
const EQ_PODCAST: [u8; 65] = [
    0x10, 0x68, 0x10, 0x5A, 0x00, 0x0A, 0x00, 0x01, 0x00, 0xE0, 0x2E, 0xC8, 0x00, 0x12, 0x00, 0x01,
    0x00, 0x50, 0x00, 0x78, 0x00, 0x08, 0x00, 0x04, 0x00, 0x40, 0x01, 0x26, 0x00, 0x06, 0x00, 0x01,
    0x00, 0x98, 0x08, 0x2C, 0x01, 0x08, 0x00, 0x01, 0x00, 0x6C, 0x02, 0x69, 0x00, 0x0A, 0x00, 0x01,
    0x00, 0x70, 0x17, 0xB4, 0x00, 0x10, 0x00, 0x01, 0x00, 0xEC, 0x04, 0x8E, 0x00, 0x0A, 0x00, 0x01,
    0x00,
];
const EQ_CLEAR_TREBLE: [u8; 65] = [
    0x09, 0x68, 0x10, 0x46, 0x00, 0x0A, 0x00, 0x01, 0x00, 0xE0, 0x2E, 0x22, 0x01, 0x0A, 0x00, 0x01,
    0x00, 0x7E, 0x00, 0x46, 0x00, 0x0A, 0x00, 0x01, 0x00, 0x40, 0x01, 0x14, 0x00, 0x06, 0x00, 0x01,
    0x00, 0x60, 0x09, 0x40, 0x01, 0x08, 0x00, 0x01, 0x00, 0x6C, 0x02, 0x69, 0x00, 0x0A, 0x00, 0x01,
    0x00, 0x70, 0x17, 0xC8, 0x00, 0x0C, 0x00, 0x01, 0x00, 0x50, 0x05, 0x8E, 0x00, 0x0A, 0x00, 0x01,
    0x00,
];

/// Preset index → desktop preset. The device reports the active index via `30 IDX`;
/// indexes outside the four mapped here belong to presets only the phone app offers.
const EQ_MAP: [(u8, EqPreset); 4] = [
    (0x00, EqPreset::Balanced),
    (0x01, EqPreset::BassBoost),
    (0x10, EqPreset::Voice),
    (0x09, EqPreset::Clear),
];

impl Bp1Ultra {
    /// Decode a validated frame (opcode + payload) from the BP1 Ultra.
    ///
    /// `33` is the authoritative noise-control state; the `34 01` ack carries none
    /// and is ignored, as are the `24 01` / `31 01` acks.
    pub fn decode_frame(frame: &Frame) -> Result<DeviceEvent, DecodeError> {
        match frame.cmd {
            op::BATTERY => Self::decode_battery(&frame.payload),
            op::LOW_LATENCY_STATE => Ok(DeviceEvent::GameModeUpdate(
                Self::require(op::LOW_LATENCY_STATE, &frame.payload, 1)?[0] != 0,
            )),
            op::CASE => Self::decode_case(&frame.payload),
            op::EQ_STATE => Self::decode_eq_state(&frame.payload),
            op::ANC_STATE => Self::decode_anc_state(&frame.payload),
            other => Err(DecodeError::UnknownOpcode(other)),
        }
    }

    /// Encode a noise-control command. `level` is the ANC strength (clamped to 1–5)
    /// and is ignored for Off and Transparency.
    pub fn encode_anc(mode: AncMode, level: u8) -> Vec<u8> {
        let payload = match mode {
            AncMode::Off => [MODE_OFF, PARAM_NONE],
            AncMode::Anc => [MODE_ANC, level.clamp(ANC_LEVEL_MIN, ANC_LEVEL_MAX)],
            AncMode::Transparency => [MODE_TRANSPARENCY, TRANSPARENCY_FULL],
        };
        crc::encode(Kind::Write, op::ANC_SET, &payload)
    }

    /// Encode a state query for one of the opcodes in [`op`] (no payload).
    pub fn encode_query(opcode: u8) -> Vec<u8> {
        crc::encode(Kind::Read, opcode, &[])
    }

    /// Encode an EQ preset change (the full curve, as the official app does).
    pub fn encode_eq(preset: EqPreset) -> Vec<u8> {
        let curve: &[u8] = match preset {
            EqPreset::Balanced => &EQ_BASEUS_CLASSIC,
            EqPreset::BassBoost => &EQ_POWERFUL_BASS,
            EqPreset::Voice => &EQ_PODCAST,
            EqPreset::Clear => &EQ_CLEAR_TREBLE,
        };
        crc::encode(Kind::Write, op::EQ_SET, curve)
    }

    /// Encode a low-latency ("game") mode change.
    pub fn encode_low_latency(on: bool) -> Vec<u8> {
        crc::encode(Kind::Write, op::LOW_LATENCY_SET, &[u8::from(on)])
    }

    /// Queries sent after connecting so the UI shows the real state straight away.
    /// The device pushes battery, case and EQ unprompted, but not these.
    pub fn initial_queries() -> [Vec<u8>; 2] {
        [
            Self::encode_query(op::ANC_STATE),
            Self::encode_query(op::LOW_LATENCY_STATE),
        ]
    }

    fn require(opcode: u8, payload: &[u8], need: usize) -> Result<&[u8], DecodeError> {
        if payload.len() < need {
            return Err(DecodeError::PayloadTooShort {
                opcode,
                need,
                got: payload.len(),
            });
        }
        Ok(payload)
    }

    fn decode_battery(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        // `02 L 00 R 01`: bytes 1 and 3 are bud markers, as on the BP1 Pro.
        let p = Self::require(op::BATTERY, payload, 4)?;
        Ok(DeviceEvent::BatteryUpdate(BatteryState {
            left_pct: p[0],
            left_charging: false,
            right_pct: p[2],
            right_charging: false,
        }))
    }

    fn decode_case(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        let p = Self::require(op::CASE, payload, 2)?;
        Ok(DeviceEvent::CaseUpdate(CaseState {
            case_pct: p[0],
            case_charging: p[1] != 0,
        }))
    }

    fn decode_anc_state(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        let p = Self::require(op::ANC_STATE, payload, 1)?;
        let mode = match p[0] {
            MODE_OFF => AncMode::Off,
            MODE_ANC => AncMode::Anc,
            MODE_TRANSPARENCY => AncMode::Transparency,
            value => {
                return Err(DecodeError::UnknownValue {
                    opcode: op::ANC_STATE,
                    value,
                })
            }
        };
        Ok(DeviceEvent::AncModeUpdate(mode))
    }

    /// Custom ANC strength (1–5) from a `33` state payload. The device remembers it
    /// whatever the active mode or scene, e.g. `33 01 66 01 05` is the "outdoor"
    /// scene with the custom level still at 5, and `33 00 05 01 05` is Normal.
    pub fn anc_level(payload: &[u8]) -> Option<u8> {
        payload
            .get(3)
            .copied()
            .filter(|level| (ANC_LEVEL_MIN..=ANC_LEVEL_MAX).contains(level))
    }

    fn decode_eq_state(payload: &[u8]) -> Result<DeviceEvent, DecodeError> {
        let idx = Self::require(op::EQ_STATE, payload, 1)?[0];
        EQ_MAP
            .iter()
            .find(|(i, _)| *i == idx)
            .map(|&(_, preset)| DeviceEvent::EqPresetUpdate(preset))
            .ok_or(DecodeError::UnknownValue {
                opcode: op::EQ_STATE,
                value: idx,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::WireFormat;

    /// Decode a complete captured wire frame (CRC envelope included).
    fn decode(raw: &[u8]) -> Result<DeviceEvent, DecodeError> {
        Bp1Ultra::decode_frame(&WireFormat::Crc.decode(raw).expect("valid capture"))
    }

    #[test]
    fn battery_report_decodes() {
        // Pushed on subscribe: L 100 %, R 100 %.
        assert_eq!(
            decode(&[
                0x78, 0x9C, 0x00, 0x0E, 0x02, 0x01, 0x05, 0x02, 0x64, 0x00, 0x64, 0x01, 0x2C, 0x56
            ])
            .unwrap(),
            DeviceEvent::BatteryUpdate(BatteryState {
                left_pct: 100,
                left_charging: false,
                right_pct: 100,
                right_charging: false,
            })
        );
        // Later session: L 90 %, R 100 %.
        assert_eq!(
            decode(&[
                0x78, 0x9C, 0x00, 0x0E, 0x02, 0x01, 0x05, 0x02, 0x5A, 0x00, 0x64, 0x01, 0xC4, 0x5B
            ])
            .unwrap(),
            DeviceEvent::BatteryUpdate(BatteryState {
                left_pct: 90,
                left_charging: false,
                right_pct: 100,
                right_charging: false,
            })
        );
    }

    #[test]
    fn case_report_decodes() {
        // Case 100 %, not charging.
        assert_eq!(
            decode(&[0x78, 0x9C, 0x00, 0x0C, 0x02, 0x01, 0x03, 0x27, 0x64, 0x00, 0x6D, 0x60])
                .unwrap(),
            DeviceEvent::CaseUpdate(CaseState {
                case_pct: 100,
                case_charging: false
            })
        );
    }

    #[test]
    fn anc_state_decodes_all_three_modes() {
        // 33 00 05 01 05 — Normal (last ANC level 5 and transparency choice remembered).
        assert_eq!(
            decode(&[
                0x78, 0x9C, 0x00, 0x0E, 0x02, 0x01, 0x05, 0x33, 0x00, 0x05, 0x01, 0x05, 0x4A, 0x0E
            ])
            .unwrap(),
            DeviceEvent::AncModeUpdate(AncMode::Off)
        );
        // 33 01 05 01 05 — noise cancellation, level 5.
        assert_eq!(
            decode(&[
                0x78, 0x9C, 0x00, 0x0E, 0x02, 0x01, 0x05, 0x33, 0x01, 0x05, 0x01, 0x05, 0xB6, 0x0F
            ])
            .unwrap(),
            DeviceEvent::AncModeUpdate(AncMode::Anc)
        );
        // 33 02 05 01 05 — transparency.
        assert_eq!(
            decode(&[
                0x78, 0x9C, 0x00, 0x0E, 0x02, 0x01, 0x05, 0x33, 0x02, 0x05, 0x01, 0x05, 0xF2, 0x0F
            ])
            .unwrap(),
            DeviceEvent::AncModeUpdate(AncMode::Transparency)
        );
    }

    #[test]
    fn anc_level_reads_the_remembered_custom_level() {
        // 33 01 01 01 01 — after the app set custom level 1.
        assert_eq!(Bp1Ultra::anc_level(&[0x01, 0x01, 0x01, 0x01]), Some(1));
        // 33 01 66 01 05 — "outdoor" scene active, custom level still 5.
        assert_eq!(Bp1Ultra::anc_level(&[0x01, 0x66, 0x01, 0x05]), Some(5));
        // 33 00 05 01 05 — Normal; the level is remembered for the next ANC switch.
        assert_eq!(Bp1Ultra::anc_level(&[0x00, 0x05, 0x01, 0x05]), Some(5));
        // Truncated or out-of-range payloads carry no level.
        assert_eq!(Bp1Ultra::anc_level(&[0x01, 0x05]), None);
        assert_eq!(Bp1Ultra::anc_level(&[0x01, 0x05, 0x01, 0x00]), None);
    }

    #[test]
    fn anc_state_with_unknown_mode_is_an_error() {
        assert!(matches!(
            Bp1Ultra::decode_frame(&Frame {
                cmd: op::ANC_STATE,
                payload: vec![0x07, 0x05, 0x01, 0x05],
            }),
            Err(DecodeError::UnknownValue {
                opcode: op::ANC_STATE,
                value: 0x07
            })
        ));
    }

    #[test]
    fn acks_carry_no_state() {
        // 34 01 (ANC ack), 24 01 (low-latency ack), 31 01 (EQ ack).
        for raw in [
            [
                0x78, 0x9C, 0x00, 0x0B, 0x02, 0x01, 0x02, 0x34, 0x01, 0x4C, 0x0E,
            ],
            [
                0x78, 0x9C, 0x00, 0x0B, 0x02, 0x01, 0x02, 0x24, 0x01, 0x8C, 0x03,
            ],
            [
                0x78, 0x9C, 0x00, 0x0B, 0x02, 0x01, 0x02, 0x31, 0x01, 0x1C, 0x0D,
            ],
        ] {
            assert!(matches!(decode(&raw), Err(DecodeError::UnknownOpcode(_))));
        }
    }

    #[test]
    fn low_latency_state_decodes() {
        assert_eq!(
            decode(&[0x78, 0x9C, 0x00, 0x0B, 0x02, 0x01, 0x02, 0x23, 0x01, 0xBC, 0x01]).unwrap(),
            DeviceEvent::GameModeUpdate(true)
        );
        assert_eq!(
            decode(&[0x78, 0x9C, 0x00, 0x0B, 0x02, 0x01, 0x02, 0x23, 0x00, 0x7C, 0xC0]).unwrap(),
            DeviceEvent::GameModeUpdate(false)
        );
    }

    #[test]
    fn eq_state_maps_known_presets() {
        // 30 00 — Baseus Classic.
        assert_eq!(
            decode(&[0x78, 0x9C, 0x00, 0x0B, 0x02, 0x01, 0x02, 0x30, 0x00, 0x4C, 0xCD]).unwrap(),
            DeviceEvent::EqPresetUpdate(EqPreset::Balanced)
        );
        // 30 01 — Powerful Bass.
        assert_eq!(
            decode(&[0x78, 0x9C, 0x00, 0x0B, 0x02, 0x01, 0x02, 0x30, 0x01, 0x8C, 0x0C]).unwrap(),
            DeviceEvent::EqPresetUpdate(EqPreset::BassBoost)
        );
    }

    #[test]
    fn eq_state_from_phone_only_preset_is_not_an_event() {
        // 30 65 — custom EQ set from the phone app; no desktop preset matches it.
        assert!(matches!(
            decode(&[0x78, 0x9C, 0x00, 0x0B, 0x02, 0x01, 0x02, 0x30, 0x65, 0x67, 0x0D]),
            Err(DecodeError::UnknownValue {
                opcode: op::EQ_STATE,
                value: 0x65
            })
        ));
    }

    #[test]
    fn anc_commands_match_official_app() {
        // Noise cancellation, level 5: 78 9C 00 0C 02 03 03 34 01 05 3B 03
        assert_eq!(
            Bp1Ultra::encode_anc(AncMode::Anc, 5),
            [0x78, 0x9C, 0x00, 0x0C, 0x02, 0x03, 0x03, 0x34, 0x01, 0x05, 0x3B, 0x03]
        );
        // Level 1: 78 9C 00 0C 02 03 03 34 01 01 F8 02
        assert_eq!(
            Bp1Ultra::encode_anc(AncMode::Anc, 1),
            [0x78, 0x9C, 0x00, 0x0C, 0x02, 0x03, 0x03, 0x34, 0x01, 0x01, 0xF8, 0x02]
        );
        // Transparency (full): 78 9C 00 0C 02 03 03 34 02 01 08 02
        assert_eq!(
            Bp1Ultra::encode_anc(AncMode::Transparency, 5),
            [0x78, 0x9C, 0x00, 0x0C, 0x02, 0x03, 0x03, 0x34, 0x02, 0x01, 0x08, 0x02]
        );
        // Normal: 78 9C 00 0C 02 03 03 34 00 FF E8 82
        assert_eq!(
            Bp1Ultra::encode_anc(AncMode::Off, 5),
            [0x78, 0x9C, 0x00, 0x0C, 0x02, 0x03, 0x03, 0x34, 0x00, 0xFF, 0xE8, 0x82]
        );
    }

    #[test]
    fn anc_level_is_clamped_to_the_discrete_range() {
        assert_eq!(
            Bp1Ultra::encode_anc(AncMode::Anc, 0),
            Bp1Ultra::encode_anc(AncMode::Anc, 1)
        );
        assert_eq!(
            Bp1Ultra::encode_anc(AncMode::Anc, 0x68),
            Bp1Ultra::encode_anc(AncMode::Anc, 5)
        );
    }

    #[test]
    fn low_latency_commands_match_official_app() {
        assert_eq!(
            Bp1Ultra::encode_low_latency(true),
            [0x78, 0x9C, 0x00, 0x0B, 0x02, 0x03, 0x02, 0x24, 0x01, 0x34, 0x02]
        );
        assert_eq!(
            Bp1Ultra::encode_low_latency(false),
            [0x78, 0x9C, 0x00, 0x0B, 0x02, 0x03, 0x02, 0x24, 0x00, 0xF4, 0xC3]
        );
    }

    #[test]
    fn eq_commands_match_official_app_byte_for_byte() {
        // CRC trailers of the official app's writes: Classic EE 3E, Powerful Bass B0 BC,
        // Podcast 5D 1D, Clear Treble B0 3E. Matching them proves the curves are exact.
        for (preset, idx, crc) in [
            (EqPreset::Balanced, 0x00, [0xEE, 0x3E]),
            (EqPreset::BassBoost, 0x01, [0xB0, 0xBC]),
            (EqPreset::Voice, 0x10, [0x5D, 0x1D]),
            (EqPreset::Clear, 0x09, [0xB0, 0x3E]),
        ] {
            let bytes = Bp1Ultra::encode_eq(preset);
            assert_eq!(bytes.len(), 0x4B);
            assert_eq!(
                &bytes[..9],
                &[0x78, 0x9C, 0x00, 0x4B, 0x02, 0x03, 0x42, 0x31, idx]
            );
            assert_eq!(&bytes[bytes.len() - 2..], &crc, "{preset:?}");
        }
    }

    #[test]
    fn initial_queries_are_well_formed() {
        let [anc, low_latency] = Bp1Ultra::initial_queries();
        assert_eq!(
            anc,
            [0x78, 0x9C, 0x00, 0x0A, 0x02, 0x01, 0x01, 0x33, 0x7C, 0x93]
        );
        assert_eq!(
            low_latency,
            [0x78, 0x9C, 0x00, 0x0A, 0x02, 0x01, 0x01, 0x23, 0xB0, 0x92]
        );
    }

    #[test]
    fn short_payloads_are_errors() {
        let frame = Frame {
            cmd: op::BATTERY,
            payload: vec![0x64, 0x00, 0x64],
        };
        assert!(matches!(
            Bp1Ultra::decode_frame(&frame),
            Err(DecodeError::PayloadTooShort {
                opcode: op::BATTERY,
                need: 4,
                got: 3
            })
        ));
    }
}
