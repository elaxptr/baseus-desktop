use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum FrameError {
    #[error("buffer too short: need at least {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },
    #[error("bad magic byte: expected 0xaa, got {got:#04x}")]
    BadMagic { got: u8 },
    #[error("bad magic bytes: expected 78 9c, got {got:02x?}")]
    BadCrcMagic { got: [u8; 2] },
    #[error("length mismatch: header declares {declared} bytes, frame has {actual}")]
    LengthMismatch { declared: usize, actual: usize },
    #[error("checksum mismatch: computed {computed:#06x}, frame carries {carried:#06x}")]
    BadChecksum { computed: u16, carried: u16 },
}

/// How a model's frames look on the wire. Every model decodes to the same
/// logical [`Frame`] (opcode + payload); only the envelope differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFormat {
    /// BP1 Pro ANC: notifications are `AA cmd payload…`, writes `BA cmd payload…`,
    /// with no length field or checksum.
    Plain,
    /// BP1 Ultra: length-prefixed frames with a CRC16/MODBUS trailer, see [`crc`].
    Crc,
}

impl WireFormat {
    /// Identify the format of a raw notification from its leading magic bytes.
    pub fn detect(buf: &[u8]) -> Option<Self> {
        match buf {
            [0x78, 0x9C, ..] => Some(Self::Crc),
            [0xAA, ..] => Some(Self::Plain),
            _ => None,
        }
    }

    /// Decode a raw notification in this format into a logical frame.
    pub fn decode(self, buf: &[u8]) -> Result<Frame, FrameError> {
        match self {
            Self::Plain => Frame::decode(buf),
            Self::Crc => crc::decode(buf),
        }
    }
}

/// CRC framing used by the BP1 Ultra (firmware 1.0.26), identical over BLE GATT
/// and Classic SPP:
///
/// ```text
/// 78 9C | LEN (u16 BE, whole frame) | 02 | KIND | N | CMD PAYLOAD… (N bytes) | CRC (u16 BE)
/// ```
///
/// `CRC` is CRC16/MODBUS over every byte before it. `KIND` is [`Kind::Read`] or
/// [`Kind::Write`] on app → device frames; device → app frames always carry `01`.
/// Confirmed against the official app's SPP traffic (Android HCI snoop) and by
/// replaying the same frames over BLE from a PC — see `docs/protocol/bp1-ultra.md`.
pub mod crc {
    use super::{Frame, FrameError};

    pub const MAGIC: [u8; 2] = [0x78, 0x9C];
    /// Magic (2) + length (2) + `02` + kind + N.
    const HEADER_LEN: usize = 7;
    const CRC_LEN: usize = 2;

    /// Request kind carried in byte 5 of an app → device frame.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Kind {
        /// Query: the device answers with the state opcode.
        Read = 0x01,
        /// Set: the device acks with `<cmd> 01`, then usually reports the new state.
        Write = 0x03,
    }

    /// CRC16/MODBUS: reflected polynomial 0xA001, initial value 0xFFFF.
    pub fn crc16_modbus(data: &[u8]) -> u16 {
        data.iter().fold(0xFFFF_u16, |crc, &byte| {
            (0..8).fold(crc ^ u16::from(byte), |c, _| {
                if c & 1 != 0 {
                    (c >> 1) ^ 0xA001
                } else {
                    c >> 1
                }
            })
        })
    }

    /// Build an app → device frame.
    pub fn encode(kind: Kind, cmd: u8, payload: &[u8]) -> Vec<u8> {
        let body_len = 1 + payload.len();
        let total = HEADER_LEN + body_len + CRC_LEN;
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&(total as u16).to_be_bytes());
        out.extend_from_slice(&[0x02, kind as u8, body_len as u8, cmd]);
        out.extend_from_slice(payload);
        let crc = crc16_modbus(&out);
        out.extend_from_slice(&crc.to_be_bytes());
        out
    }

    /// Validate a frame (magic, both length fields, checksum) and extract opcode + payload.
    pub fn decode(buf: &[u8]) -> Result<Frame, FrameError> {
        let min = HEADER_LEN + 1 + CRC_LEN;
        if buf.len() < min {
            return Err(FrameError::TooShort {
                need: min,
                got: buf.len(),
            });
        }
        if buf[..2] != MAGIC {
            return Err(FrameError::BadCrcMagic {
                got: [buf[0], buf[1]],
            });
        }
        let declared = usize::from(u16::from_be_bytes([buf[2], buf[3]]));
        if declared != buf.len() {
            return Err(FrameError::LengthMismatch {
                declared,
                actual: buf.len(),
            });
        }
        let body_len = usize::from(buf[6]);
        if body_len == 0 || HEADER_LEN + body_len + CRC_LEN != buf.len() {
            return Err(FrameError::LengthMismatch {
                declared: HEADER_LEN + body_len + CRC_LEN,
                actual: buf.len(),
            });
        }
        let (data, trailer) = buf.split_at(buf.len() - CRC_LEN);
        let carried = u16::from_be_bytes([trailer[0], trailer[1]]);
        let computed = crc16_modbus(data);
        if computed != carried {
            return Err(FrameError::BadChecksum { computed, carried });
        }
        Ok(Frame {
            cmd: buf[HEADER_LEN],
            payload: buf[HEADER_LEN + 1..buf.len() - CRC_LEN].to_vec(),
        })
    }
}

/// A raw BP1 Pro ANC GATT frame: [0xAA, cmd, payload...]
///
/// The frame length is determined by the GATT PDU size.
/// No embedded length field or checksum is present in the observed wire format.
/// Confirmed from live nRF Connect capture on BASS BP1 PRO (4A:01:CE:BA:C8:03).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub cmd: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    /// Parse a raw GATT notification or write payload.
    pub fn decode(buf: &[u8]) -> Result<Self, FrameError> {
        if buf.len() < 2 {
            return Err(FrameError::TooShort {
                need: 2,
                got: buf.len(),
            });
        }
        if buf[0] != 0xAA {
            return Err(FrameError::BadMagic { got: buf[0] });
        }
        Ok(Self {
            cmd: buf[1],
            payload: buf[2..].to_vec(),
        })
    }

    /// Encode a frame for writing to the GATT write characteristic.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + self.payload.len());
        out.push(0xAA);
        out.push(self.cmd);
        out.extend_from_slice(&self.payload);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_anc_off() {
        // Captured: AA 30 00 (ANC off, default state)
        let f = Frame::decode(&[0xAA, 0x30, 0x00]).unwrap();
        assert_eq!(f.cmd, 0x30);
        assert_eq!(f.payload, &[0x00]);
    }

    #[test]
    fn decode_anc_transparency() {
        // Captured: AA 32 02 FF
        let f = Frame::decode(&[0xAA, 0x32, 0x02, 0xFF]).unwrap();
        assert_eq!(f.cmd, 0x32);
        assert_eq!(f.payload, &[0x02, 0xFF]);
    }

    #[test]
    fn decode_anc_on() {
        // Captured: AA 33 01 68
        let f = Frame::decode(&[0xAA, 0x33, 0x01, 0x68]).unwrap();
        assert_eq!(f.cmd, 0x33);
        assert_eq!(f.payload, &[0x01, 0x68]);
    }

    #[test]
    fn decode_battery() {
        // Captured: AA 02 64 00 5A 01 → left=100% not charging, right=90% charging
        let f = Frame::decode(&[0xAA, 0x02, 0x64, 0x00, 0x5A, 0x01]).unwrap();
        assert_eq!(f.cmd, 0x02);
        assert_eq!(f.payload, &[0x64, 0x00, 0x5A, 0x01]);
    }

    #[test]
    fn encode_then_decode_round_trips() {
        let original = Frame {
            cmd: 0x02,
            payload: vec![0x64, 0x00, 0x5A, 0x01],
        };
        let decoded = Frame::decode(&original.encode()).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_rejects_bad_magic() {
        assert!(matches!(
            Frame::decode(&[0x00, 0x30]),
            Err(FrameError::BadMagic { got: 0x00 })
        ));
    }

    #[test]
    fn decode_rejects_too_short() {
        assert!(matches!(
            Frame::decode(&[0xAA]),
            Err(FrameError::TooShort { need: 2, got: 1 })
        ));
        assert!(matches!(
            Frame::decode(&[]),
            Err(FrameError::TooShort { need: 2, got: 0 })
        ));
    }

    // ---- CRC framing (BP1 Ultra). Every frame below was captured from a real unit. ----

    #[test]
    fn crc16_modbus_matches_captured_frames() {
        // Battery report: 78 9C 00 0E 02 01 05 02 64 00 64 01 | 2C 56
        let frame = [
            0x78, 0x9C, 0x00, 0x0E, 0x02, 0x01, 0x05, 0x02, 0x64, 0x00, 0x64, 0x01,
        ];
        assert_eq!(crc::crc16_modbus(&frame), 0x2C56);
        // Standard CRC16/MODBUS check value.
        assert_eq!(crc::crc16_modbus(b"123456789"), 0x4B37);
    }

    #[test]
    fn crc_decode_battery_report() {
        let f = crc::decode(&[
            0x78, 0x9C, 0x00, 0x0E, 0x02, 0x01, 0x05, 0x02, 0x64, 0x00, 0x64, 0x01, 0x2C, 0x56,
        ])
        .unwrap();
        assert_eq!(f.cmd, 0x02);
        assert_eq!(f.payload, &[0x64, 0x00, 0x64, 0x01]);
    }

    #[test]
    fn crc_decode_single_byte_body() {
        // Shortest real frame: a bare query, here the official app's ANC state query `33`.
        let f = crc::decode(&[0x78, 0x9C, 0x00, 0x0A, 0x02, 0x01, 0x01, 0x33, 0x7C, 0x93]).unwrap();
        assert_eq!(f.cmd, 0x33);
        assert!(f.payload.is_empty());
    }

    #[test]
    fn crc_encode_matches_official_app_writes() {
        // ANC on, level 5 — sent by the Baseus app: 78 9C 00 0C 02 03 03 34 01 05 3B 03
        assert_eq!(
            crc::encode(crc::Kind::Write, 0x34, &[0x01, 0x05]),
            [0x78, 0x9C, 0x00, 0x0C, 0x02, 0x03, 0x03, 0x34, 0x01, 0x05, 0x3B, 0x03]
        );
        // ANC state query: 78 9C 00 0A 02 01 01 33 7C 93
        assert_eq!(
            crc::encode(crc::Kind::Read, 0x33, &[]),
            [0x78, 0x9C, 0x00, 0x0A, 0x02, 0x01, 0x01, 0x33, 0x7C, 0x93]
        );
    }

    #[test]
    fn crc_encode_then_decode_round_trips() {
        let bytes = crc::encode(crc::Kind::Write, 0x24, &[0x01]);
        let f = crc::decode(&bytes).unwrap();
        assert_eq!(f.cmd, 0x24);
        assert_eq!(f.payload, &[0x01]);
    }

    #[test]
    fn crc_decode_rejects_bad_checksum() {
        assert_eq!(
            crc::decode(&[
                0x78, 0x9C, 0x00, 0x0E, 0x02, 0x01, 0x05, 0x02, 0x64, 0x00, 0x64, 0x01, 0x2C, 0x57,
            ]),
            Err(FrameError::BadChecksum {
                computed: 0x2C56,
                carried: 0x2C57
            })
        );
    }

    #[test]
    fn crc_decode_rejects_length_mismatch() {
        // Header declares 0x0F bytes but the frame is 14 long (e.g. a truncated PDU).
        assert_eq!(
            crc::decode(&[
                0x78, 0x9C, 0x00, 0x0F, 0x02, 0x01, 0x05, 0x02, 0x64, 0x00, 0x64, 0x01, 0x2C, 0x56,
            ]),
            Err(FrameError::LengthMismatch {
                declared: 15,
                actual: 14
            })
        );
    }

    #[test]
    fn crc_decode_rejects_bad_magic_and_short_input() {
        assert_eq!(
            crc::decode(&[0xAA, 0x02, 0x64, 0x00, 0x64, 0x01, 0, 0, 0, 0]),
            Err(FrameError::BadCrcMagic { got: [0xAA, 0x02] })
        );
        assert_eq!(
            crc::decode(&[0x78, 0x9C, 0x00]),
            Err(FrameError::TooShort { need: 10, got: 3 })
        );
    }

    #[test]
    fn wire_format_detects_by_magic() {
        assert_eq!(
            WireFormat::detect(&[0xAA, 0x02, 0x64]),
            Some(WireFormat::Plain)
        );
        assert_eq!(
            WireFormat::detect(&[0x78, 0x9C, 0x00, 0x0B]),
            Some(WireFormat::Crc)
        );
        assert_eq!(WireFormat::detect(&[0xBA, 0x34]), None);
        assert_eq!(WireFormat::detect(&[]), None);
    }
}
