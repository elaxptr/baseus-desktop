use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("buffer too short: need at least {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },
    #[error("bad magic bytes: expected {expected:#04x} {expected2:#04x}, got {got:#04x} {got2:#04x}")]
    BadMagic { expected: u8, expected2: u8, got: u8, got2: u8 },
    #[error("checksum mismatch: computed {computed:#04x}, packet has {packet:#04x}")]
    ChecksumMismatch { computed: u8, packet: u8 },
}

/// Raw Baseus protocol frame: [magic0, magic1, len, cmd, payload..., checksum]
/// Magic bytes and checksum algorithm are confirmed in Phase 0 and filled in Task 6.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub cmd:     u8,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn encode(&self) -> Vec<u8> {
        todo!("fill in after Phase 0 — see docs/protocol/framing.md")
    }

    pub fn decode(buf: &[u8]) -> Result<Self, FrameError> {
        todo!("fill in after Phase 0 — see docs/protocol/framing.md")
    }
}
