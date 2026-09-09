pub use crate::session_control::terminate;
use crate::{messages::WorkerEvent, model_file};
use serde::Serialize;
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(super) struct Inner {
    pub(super) generation: u64,
    pub(super) child: Option<Arc<Mutex<Child>>>,
    pub(super) finishing: bool,
    pub(super) target: u64,
    pub(super) delivered: Option<DeliveryOutcome>,
}
#[derive(Clone, Serialize)]
pub struct DeliveryOutcome {
    pub status: &'static str,
    pub text: Option<String>,
}
#[derive(Clone, Default)]
pub struct Sessions(pub(super) Arc<Mutex<Inner>>);
impl Sessions {
    pub(super) fn with_current<T>(
        &self,
        generation: u64,
        action: impl FnOnce(&mut Inner) -> T,
    ) -> Option<T> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation != generation {
            return None;
        }
        Some(action(&mut state))
    }
    pub fn when_idle<T>(&self, action: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let state = self.0.lock().map_err(|_| "Session state unavailable")?;
        if state.child.is_some() {
            return Err(
                "Wait for recognition to stop, or choose Cancel, before dismissing Logia.".into(),
            );
        }
        action()
    }
}
#[derive(Clone, Serialize)]
pub(super) struct Update {
    pub(super) generation: u64,
    pub(super) event: WorkerEvent,
}

#[tauri::command]
pub fn start_recording(
    target: Option<String>,
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
) -> Result<u64, String> {
    launch(app, sessions, "--recognizer", target)
}

#[tauri::command]
pub fn warmup_recognizer(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
) -> Result<u64, String> {
    launch(app, sessions, "--warmup", None)
}

fn launch(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
    mode: &str,
    target: Option<String>,
) -> Result<u64, String> {
    let target = target
        .unwrap_or_else(|| "0".into())
        .parse::<u64>()
        .map_err(|_| "Invalid destination")?;
    let mut state = sessions.0.lock().map_err(|_| "Session state unavailable")?;
    if state.child.is_some() {
        return Err("Wait for the current recording to stop".into());
    }
    let path = model_file::path(&app)?;
    if !path.is_file() {
        return Err("Download the English model first".into());
    }
    let executable = std::env::current_exe().map_err(|_| "Could not locate the recognizer")?;
    let mut child = Command::new(executable)
        .arg(mode)
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not start recognition")?;
    let output = child.stdout.take().expect("piped worker stdout");
    state.generation += 1;
    let generation = state.generation;
    let child = Arc::new(Mutex::new(child));
    state.child = Some(child.clone());
    state.finishing = false;
    state.target = target;
    state.delivered = None;
    let sessions = sessions.inner().clone();
    std::thread::spawn(move || {
        crate::session_output::read(app, sessions, generation, child, output)
    });
    Ok(generation)
}

#[tauri::command]
pub fn stop_recording(sessions: tauri::State<'_, Sessions>) -> Result<(), String> {
    let mut state = sessions.0.lock().map_err(|_| "Session state unavailable")?;
    if state.finishing {
        return Ok(());
    }
    let Some(child) = &state.child else {
        return Ok(());
    };
    let mut child = child.lock().map_err(|_| "Worker unavailable")?;
    child
        .stdin
        .as_mut()
        .ok_or("Recognition connection closed")?
        .write_all(b"finish\n")
        .map_err(|_| "Could not stop recording; use Cancel")?;
    drop(child);
    state.finishing = true;
    Ok(())
}
