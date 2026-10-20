use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("unknown opcode {0:#04x}")]
    UnknownOpcode(u8),
    #[error("payload too short for opcode {opcode:#04x}: need {need}, got {got}")]
    PayloadTooShort { opcode: u8, need: usize, got: usize },
    #[error("unknown value {value:#04x} for opcode {opcode:#04x}")]
    UnknownValue { opcode: u8, value: u8 },
}

pub mod bp1_pro_anc;
pub mod bp1_ultra;

pub use bp1_pro_anc::Bp1ProAnc;
pub use bp1_ultra::Bp1Ultra;
