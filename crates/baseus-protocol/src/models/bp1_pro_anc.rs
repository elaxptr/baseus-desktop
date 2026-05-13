use crate::{models::DecodeError, types::DeviceEvent, Frame};

pub struct Bp1ProAnc;

impl Bp1ProAnc {
    pub fn decode_frame(_frame: &Frame) -> Result<DeviceEvent, DecodeError> {
        todo!("fill in opcodes from docs/protocol/bp1-pro-anc.md after Phase 0")
    }
}
