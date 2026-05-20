use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("unknown opcode {0:#04x}")]
    UnknownOpcode(u8),
    #[error("payload too short for opcode {opcode:#04x}: need {need}, got {got}")]
    PayloadTooShort { opcode: u8, need: usize, got: usize },
}

pub mod bp1_pro_anc;
pub mod inspire_xh1;

pub use bp1_pro_anc::Bp1ProAnc;
pub use inspire_xh1::InspireXh1;
