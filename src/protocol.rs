use crate::hook_event::HookEvent;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub const MAX_FRAME_SIZE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayEvent {
    pub event: HookEvent,
    pub seed: u64,
    pub volume: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("frame larger than {0} bytes")]
    TooLarge(usize),
    #[error("decode error: {0}")]
    Decode(#[from] bincode::Error),
}

pub fn write_frame<W: Write>(w: &mut W, event: &PlayEvent) -> Result<(), FrameError> {
    let bytes = bincode::serialize(event)?;
    if bytes.len() > MAX_FRAME_SIZE {
        return Err(FrameError::TooLarge(MAX_FRAME_SIZE));
    }
    w.write_all(&(bytes.len() as u32).to_be_bytes())?;
    w.write_all(&bytes)?;
    Ok(())
}

pub fn read_frame<R: Read>(r: &mut R) -> Result<PlayEvent, FrameError> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(FrameError::TooLarge(MAX_FRAME_SIZE));
    }
    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload)?;
    let event = bincode::deserialize(&payload)?;
    Ok(event)
}
