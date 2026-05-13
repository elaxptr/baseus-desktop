use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("buffer too short: need at least {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },
    #[error(
        "bad magic bytes: expected {expected:#04x} {expected2:#04x}, got {got:#04x} {got2:#04x}"
    )]
    BadMagic {
        expected: u8,
        expected2: u8,
        got: u8,
        got2: u8,
    },
    #[error("checksum mismatch: computed {computed:#04x}, packet has {packet:#04x}")]
    ChecksumMismatch { computed: u8, packet: u8 },
}

/// CRC-8 (poly=0x07, init=0x00).
pub(crate) fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Raw Baseus protocol frame: [magic0, magic1, len, cmd, payload..., checksum]
/// Magic bytes and checksum algorithm are confirmed in Phase 0 and filled in Task 6.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub cmd: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn encode(&self) -> Vec<u8> {
        const MAGIC: [u8; 2] = [0xAA, 0x03];
        let len = self.payload.len() as u8;
        let mut out = Vec::with_capacity(5 + self.payload.len());
        out.extend_from_slice(&MAGIC);
        out.push(len);
        out.push(self.cmd);
        out.extend_from_slice(&self.payload);
        let chk = crc8(&out);
        out.push(chk);
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Self, FrameError> {
        const MAGIC: [u8; 2] = [0xAA, 0x03];
        const MIN_LEN: usize = 5; // magic(2) + len(1) + cmd(1) + crc(1)
        if buf.len() < MIN_LEN {
            return Err(FrameError::TooShort {
                need: MIN_LEN,
                got: buf.len(),
            });
        }
        if buf[0] != MAGIC[0] || buf[1] != MAGIC[1] {
            return Err(FrameError::BadMagic {
                expected: MAGIC[0],
                expected2: MAGIC[1],
                got: buf[0],
                got2: buf[1],
            });
        }
        let payload_len = buf[2] as usize;
        let total = MIN_LEN + payload_len;
        if buf.len() < total {
            return Err(FrameError::TooShort {
                need: total,
                got: buf.len(),
            });
        }
        let cmd = buf[3];
        let payload = buf[4..4 + payload_len].to_vec();
        let computed = crc8(&buf[..total - 1]);
        let packet = buf[total - 1];
        if computed != packet {
            return Err(FrameError::ChecksumMismatch { computed, packet });
        }
        Ok(Self { cmd, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // cmd=0x01, payload=[0x64, 0x5a, 0x78, 0x00]
    // The last byte (0x6b) is the CRC-8 (poly=0x07, init=0x00) of all preceding bytes.
    // Computed over: [0xaa, 0x03, 0x04, 0x01, 0x64, 0x5a, 0x78, 0x00] → 0x6b
    const SYNTH_FRAME: &[u8] = &[0xaa, 0x03, 0x04, 0x01, 0x64, 0x5a, 0x78, 0x00, 0x6b];

    #[test]
    fn decode_synth_frame_gives_correct_cmd_and_payload() {
        let f = Frame::decode(SYNTH_FRAME).expect("should decode");
        assert_eq!(f.cmd, 0x01);
        assert_eq!(f.payload, &[0x64, 0x5a, 0x78, 0x00]);
    }

    #[test]
    fn encode_then_decode_round_trips() {
        let original = Frame {
            cmd: 0x01,
            payload: vec![0x64, 0x5a, 0x78, 0x00],
        };
        let bytes = original.encode();
        let decoded = Frame::decode(&bytes).expect("should decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut bad = SYNTH_FRAME.to_vec();
        bad[0] = 0x00;
        assert!(matches!(
            Frame::decode(&bad),
            Err(FrameError::BadMagic { .. })
        ));
    }

    #[test]
    fn decode_rejects_checksum_mismatch() {
        let mut bad = SYNTH_FRAME.to_vec();
        *bad.last_mut().unwrap() ^= 0xff;
        assert!(matches!(
            Frame::decode(&bad),
            Err(FrameError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn decode_rejects_buffer_too_short() {
        assert!(matches!(
            Frame::decode(&[0xaa]),
            Err(FrameError::TooShort { .. })
        ));
    }
}
