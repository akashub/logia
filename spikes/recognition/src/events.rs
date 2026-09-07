use crate::protocol::{write_message, Event, Message, SessionId};

#[derive(Debug, PartialEq, Eq)]
pub enum EventError {
    WrongSession,
    OutOfOrder,
    StableRevision,
    Closed,
    TooLarge,
    Unexpected,
}

/// Holds at most one cumulative partial or one terminal event, plus known stable
/// text. Terminal final text replaces the preview; it is never appended to it.
pub struct EventMailbox {
    session: SessionId,
    last_seq: Option<u64>,
    stable: String,
    closed: bool,
    pending: Option<Event>,
}
impl EventMailbox {
    pub fn new(session: SessionId) -> Self {
        Self {
            session,
            last_seq: None,
            stable: String::new(),
            closed: false,
            pending: None,
        }
    }
    pub fn push(&mut self, event: Event) -> Result<(), EventError> {
        if self.closed {
            return Err(EventError::Closed);
        }
        let (session, seq) = match &event {
            Event::ModelReady {} => return Err(EventError::Unexpected),
            Event::Partial { session, seq, .. } | Event::Final { session, seq, .. } => {
                (*session, Some(*seq))
            }
            Event::Canceled { session } | Event::Failed { session, .. } => (*session, None),
        };
        if session != self.session {
            return Err(EventError::WrongSession);
        }
        if seq.is_some_and(|s| self.last_seq.is_some_and(|last| s <= last)) {
            return Err(EventError::OutOfOrder);
        }
        // Enforce actual encoded size, including JSON escaping and metadata.
        let message = Message::Event(event);
        write_message(&mut std::io::sink(), &message).map_err(|_| EventError::TooLarge)?;
        let Message::Event(event) = message else {
            unreachable!()
        };
        match &event {
            Event::Partial { stable, .. } => {
                if !stable.starts_with(&self.stable) {
                    return Err(EventError::StableRevision);
                }
                self.stable.clone_from(stable);
            }
            Event::Final { text, .. } => {
                if !text.starts_with(&self.stable) {
                    return Err(EventError::StableRevision);
                }
                self.closed = true;
            }
            Event::Canceled { .. } | Event::Failed { .. } => self.closed = true,
            Event::ModelReady {} => unreachable!(),
        }
        if seq.is_some() {
            self.last_seq = seq;
        }
        self.pending = Some(event);
        Ok(())
    }
    pub fn pop(&mut self) -> Option<Event> {
        self.pending.take()
    }
    pub fn stable(&self) -> &str {
        &self.stable
    }
    /// Suppresses late output; stopping/reaping inference is the supervisor's job.
    pub fn cancel(&mut self) {
        self.closed = true;
        self.pending = None;
    }
}
