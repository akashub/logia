use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub type SessionId = u64;
pub const VERSION: u16 = 1;
pub const MAX_PAYLOAD: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    Truncated,
    TooLarge,
    InvalidPayload,
    UnsupportedVersion,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Control {
    Start { session: SessionId },
    Finish { session: SessionId },
    Cancel { session: SessionId },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    ModelReady {},
    Partial {
        session: SessionId,
        seq: u64,
        stable: String,
        tail: String,
    },
    Final {
        session: SessionId,
        seq: u64,
        text: String,
    },
    Canceled {
        session: SessionId,
    },
    Failed {
        session: SessionId,
        code: FailureCode,
    },
}

// Debug output must never expose recognized content.
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ModelReady {} => "ModelReady",
            Self::Partial { .. } => "Partial(<redacted>)",
            Self::Final { .. } => "Final(<redacted>)",
            Self::Canceled { .. } => "Canceled",
            Self::Failed { .. } => "Failed",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    QueueFull,
    TextTooLarge,
    InvalidAudio,
    InvalidProtocol,
    Engine,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Message {
    Control(Control),
    Event(Event),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u16,
    message: Message,
}

pub fn accepts(active: Option<SessionId>, canceled: bool, incoming: SessionId) -> bool {
    !canceled && active == Some(incoming)
}

pub(crate) fn read_payload(
    reader: &mut impl Read,
    cap: usize,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    let mut header = [0; 4];
    // EOF is clean only between frames, never within a header or body.
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ProtocolError::Io),
        }
    }
    reader.read_exact(&mut header[1..]).map_err(read_error)?;
    let length = u32::from_le_bytes(header) as usize;
    if length > cap {
        return Err(ProtocolError::TooLarge);
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload).map_err(read_error)?;
    Ok(Some(payload))
}

fn read_error(error: io::Error) -> ProtocolError {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        ProtocolError::Truncated
    } else {
        ProtocolError::Io
    }
}

pub(crate) fn write_payload(writer: &mut impl Write, payload: &[u8]) -> Result<(), ProtocolError> {
    if payload.len() > MAX_PAYLOAD {
        return Err(ProtocolError::TooLarge);
    }
    writer
        .write_all(&(payload.len() as u32).to_le_bytes())
        .map_err(|_| ProtocolError::Io)?;
    writer.write_all(payload).map_err(|_| ProtocolError::Io)
}

pub fn read_message(reader: &mut impl Read) -> Result<Option<Message>, ProtocolError> {
    let Some(payload) = read_payload(reader, MAX_PAYLOAD)? else {
        return Ok(None);
    };
    let envelope: Envelope =
        serde_json::from_slice(&payload).map_err(|_| ProtocolError::InvalidPayload)?;
    if envelope.version != VERSION {
        return Err(ProtocolError::UnsupportedVersion);
    }
    Ok(Some(envelope.message))
}

struct BoundedBytes(Vec<u8>);
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_PAYLOAD - self.0.len() {
            return Err(io::Error::other("payload too large"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn write_message(writer: &mut impl Write, message: &Message) -> Result<(), ProtocolError> {
    #[derive(Serialize)]
    struct BorrowedEnvelope<'a> {
        version: u16,
        message: &'a Message,
    }
    let mut bytes = BoundedBytes(Vec::with_capacity(MAX_PAYLOAD));
    serde_json::to_writer(
        &mut bytes,
        &BorrowedEnvelope {
            version: VERSION,
            message,
        },
    )
    .map_err(|_| ProtocolError::TooLarge)?;
    write_payload(writer, &bytes.0)
}
