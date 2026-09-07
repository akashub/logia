//! Scheduling only: the caller must actually stop/reap inference on cancel.
use crate::{audio_queue::MAX_SAMPLE_BYTES, protocol::SessionId};

#[derive(Debug, PartialEq, Eq)]
pub enum PreviewError {
    TooLarge,
    InvalidAudio,
    WrongSession,
    OutOfOrder,
    Closed,
    Idle,
}

pub struct PreviewSnapshot {
    session: SessionId,
    seq: u64,
    samples: Box<[f32]>,
}
impl std::fmt::Debug for PreviewSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewSnapshot(<redacted>)")
    }
}
impl PreviewSnapshot {
    pub fn new(session: SessionId, seq: u64, samples: Vec<f32>) -> Result<Self, PreviewError> {
        if samples.len() > MAX_SAMPLE_BYTES / 4 {
            return Err(PreviewError::TooLarge);
        }
        if samples.is_empty() || samples.iter().any(|v| !v.is_finite()) {
            return Err(PreviewError::InvalidAudio);
        }
        Ok(Self {
            session,
            seq,
            samples: samples.into_boxed_slice(),
        })
    }
    pub fn seq(&self) -> u64 {
        self.seq
    }
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}

/// One caller-owned active snapshot plus one replaceable pending snapshot.
/// Each snapshot is independently capped at ten seconds of normalized PCM.
pub struct PreviewWork {
    session: SessionId,
    last_seq: Option<u64>,
    active: bool,
    closed: bool,
    pending: Option<PreviewSnapshot>,
}
impl PreviewWork {
    pub fn new(session: SessionId) -> Self {
        Self {
            session,
            last_seq: None,
            active: false,
            closed: false,
            pending: None,
        }
    }
    /// A returned snapshot is the sole new job to start; None coalesces pending work.
    pub fn submit(
        &mut self,
        snapshot: PreviewSnapshot,
    ) -> Result<Option<PreviewSnapshot>, PreviewError> {
        if self.closed {
            return Err(PreviewError::Closed);
        }
        if snapshot.session != self.session {
            return Err(PreviewError::WrongSession);
        }
        if self.last_seq.is_some_and(|seq| snapshot.seq <= seq) {
            return Err(PreviewError::OutOfOrder);
        }
        self.last_seq = Some(snapshot.seq);
        if self.active {
            self.pending = Some(snapshot);
            Ok(None)
        } else {
            self.active = true;
            Ok(Some(snapshot))
        }
    }
    /// Call only after the owned job has stopped. May return its replacement.
    pub fn complete(&mut self) -> Result<Option<PreviewSnapshot>, PreviewError> {
        if !self.active {
            return Err(PreviewError::Idle);
        }
        let pending = self.pending.take();
        self.active = pending.is_some();
        Ok(pending)
    }
    pub fn active(&self) -> bool {
        self.active
    }
    /// Suppresses pending work but retains active ownership until complete().
    pub fn cancel(&mut self) {
        self.closed = true;
        self.pending = None;
    }
}
