use crate::{audio::AudioFrame, protocol::SessionId};

pub const MAX_SESSION_FRAMES: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionError {
    WrongSession,
    OutOfOrder,
    ExtraFrames,
    EarlyEnd,
    MissingEnd,
    ConflictingFinish,
    TooManyFrames,
    Failed,
}

/// Call `consume` only after inference has consumed a frame, not when queued.
pub struct CompletionBarrier {
    session: SessionId,
    consumed: u64,
    expected: Option<u64>,
    ended: bool,
    failed: bool,
}

impl CompletionBarrier {
    pub fn new(session: SessionId) -> Self {
        Self {
            session,
            consumed: 0,
            expected: None,
            ended: false,
            failed: false,
        }
    }
    fn fail(&mut self, error: CompletionError) -> Result<(), CompletionError> {
        self.failed = true;
        Err(error)
    }
    fn validate(&mut self) -> Result<(), CompletionError> {
        if let Some(expected) = self.expected {
            if self.consumed > expected {
                return self.fail(CompletionError::ExtraFrames);
            }
            if self.ended && self.consumed < expected {
                return self.fail(CompletionError::EarlyEnd);
            }
            if expected > 0 && self.consumed == expected && !self.ended {
                return self.fail(CompletionError::MissingEnd);
            }
        }
        Ok(())
    }
    pub fn finish(&mut self, session: SessionId, frames: u64) -> Result<(), CompletionError> {
        if session != self.session {
            return Err(CompletionError::WrongSession);
        }
        if self.failed {
            return Err(CompletionError::Failed);
        }
        if frames > MAX_SESSION_FRAMES {
            return self.fail(CompletionError::TooManyFrames);
        }
        if self.expected.is_some_and(|expected| expected != frames) {
            return self.fail(CompletionError::ConflictingFinish);
        }
        self.expected = Some(frames);
        self.validate()
    }
    pub fn consume(&mut self, frame: &AudioFrame) -> Result<(), CompletionError> {
        if frame.session() != self.session {
            return Err(CompletionError::WrongSession);
        }
        if self.failed {
            return Err(CompletionError::Failed);
        }
        if self.ended || self.expected == Some(self.consumed) {
            return self.fail(CompletionError::ExtraFrames);
        }
        if self.consumed >= MAX_SESSION_FRAMES {
            return self.fail(CompletionError::TooManyFrames);
        }
        if frame.index() != self.consumed {
            return self.fail(CompletionError::OutOfOrder);
        }
        self.consumed += 1;
        self.ended = frame.is_last();
        self.validate()
    }
    pub fn ready(&self) -> bool {
        !self.failed && self.expected == Some(self.consumed) && (self.consumed == 0 || self.ended)
    }
}
