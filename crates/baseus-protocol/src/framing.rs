use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum FrameError {
    #[error("buffer too short: need at least {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },
    #[error("bad magic byte: expected 0xaa, got {got:#04x}")]
    BadMagic { got: u8 },
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
}
