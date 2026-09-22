use serde::{Deserialize, Serialize};
use std::io::{self, Write};

pub const MAX_EVENT_BYTES: u64 = 65_536;
pub const MAX_SECONDS: u64 = 60;

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkerEvent {
    Loading,
    Listening,
    // claude 2026-09-11: measured peak amplitude of a captured chunk. Carries
    // evidence, not decoration: macOS hands a denied microphone silent buffers
    // rather than an error, and only real samples can distinguish that from a
    // recognizer that has nothing to say.
    Level { peak: f32 },
    Partial { text: String },
    Final { text: String },
    Error { message: String },
    Stopped,
}

// stdout is private parent/worker IPC, never a diagnostic log.
pub fn emit(event: &WorkerEvent) -> Result<(), String> {
    let bytes = serde_json::to_vec(event).map_err(|_| "Could not encode recognition update")?;
    if bytes.len() as u64 > MAX_EVENT_BYTES {
        return Err("Transcript exceeded the preview limit".into());
    }
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(&bytes)
        .and_then(|_| stdout.write_all(b"\n"))
        .and_then(|_| stdout.flush())
        .map_err(|_| "Recognition connection closed".into())
}
