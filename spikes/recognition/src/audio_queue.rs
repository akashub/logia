use crate::{audio::AudioFrame, protocol::SessionId};
use std::collections::VecDeque;

pub const MAX_FRAMES: usize = 500;
pub const MAX_SAMPLE_BYTES: usize = 640_000;

#[derive(Debug, PartialEq, Eq)]
pub enum QueueError {
    Full,
    WrongSession,
    OutOfOrder,
    Finished,
}

/// Nonblocking queue. A caller receiving Full must fail the session with QueueFull;
/// it must not slow a paced producer or silently discard captured audio.
pub struct AudioQueue {
    session: SessionId,
    next_index: u64,
    finished: bool,
    frames: VecDeque<AudioFrame>,
    sample_bytes: usize,
}
impl AudioQueue {
    pub fn new(session: SessionId) -> Self {
        Self {
            session,
            next_index: 0,
            finished: false,
            frames: VecDeque::new(),
            sample_bytes: 0,
        }
    }
    pub fn try_push(&mut self, frame: AudioFrame) -> Result<(), QueueError> {
        if self.finished {
            return Err(QueueError::Finished);
        }
        if frame.session() != self.session {
            return Err(QueueError::WrongSession);
        }
        if frame.index() != self.next_index {
            return Err(QueueError::OutOfOrder);
        }
        let next = self
            .next_index
            .checked_add(1)
            .ok_or(QueueError::OutOfOrder)?;
        if self.frames.len() == MAX_FRAMES
            || frame.sample_bytes() > MAX_SAMPLE_BYTES - self.sample_bytes
        {
            return Err(QueueError::Full);
        }
        self.sample_bytes += frame.sample_bytes();
        self.finished = frame.is_last();
        self.next_index = next;
        self.frames.push_back(frame);
        Ok(())
    }
    pub fn pop(&mut self) -> Option<AudioFrame> {
        let frame = self.frames.pop_front()?;
        self.sample_bytes -= frame.sample_bytes();
        Some(frame)
    }
    pub fn sample_bytes(&self) -> usize {
        self.sample_bytes
    }
    /// Releases queued audio without resetting the session's ordering state.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.sample_bytes = 0;
    }
}
