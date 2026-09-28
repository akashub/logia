pub use crate::session_control::terminate;
use crate::{messages::WorkerEvent, model_file};
use serde::Serialize;
use std::{
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};
use tauri::Manager;

#[derive(Default)]
pub(super) struct Inner {
    pub(super) generation: u64,
    pub(super) shortcut_configuration: Arc<std::sync::atomic::AtomicBool>,
    pub(super) child: Option<Arc<Mutex<Child>>>,
    pub(super) finishing: bool,
    pub(super) hold: Option<u64>,
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
        state.require_idle()?;
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
    gesture: Option<String>,
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
) -> Result<u64, String> {
    launch(app, sessions, "--recognizer", target, gesture)
}

#[tauri::command]
pub fn warmup_recognizer(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
) -> Result<u64, String> {
    launch(app, sessions, "--warmup", None, None)
}

fn launch(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
    mode: &str,
    target: Option<String>,
    gesture: Option<String>,
) -> Result<u64, String> {
    let target = target
        .unwrap_or_else(|| "0".into())
        .parse::<u64>()
        .map_err(|_| "Invalid destination")?;
    let mut state = sessions.0.lock().map_err(|_| "Session state unavailable")?;
    // Consume a global gesture once even when startup fails or is denied.
    let permit = gesture
        .as_deref()
        .map(|id| crate::shortcut_gesture::GESTURES.claim(id))
        .transpose()?;
    state.require_idle()?;
    if mode == "--recognizer" {
        crate::permissions::require_microphone()?;
    }
    // Same lock order as model mutations: session lock, then mutation gate.
    // The worker slot is occupied before releasing this lock, so a file change
    // can neither race the selected path lookup nor the child startup.
    if app
        .state::<crate::model_download::DownloadState>()
        .0
        .load(std::sync::atomic::Ordering::Acquire)
    {
        return Err("Wait for the model download or change to finish".into());
    }
    let path = model_file::path(&app)?;
    if !path.is_file() {
        return Err("Download your selected model first".into());
    }
    let executable = std::env::current_exe().map_err(|_| "Could not locate the recognizer")?;
    let input = if mode == "--recognizer" {
        Some(crate::input_devices::snapshot(
            &crate::input_settings::folder(&app)?,
        )?)
    } else {
        None
    };
    let mut command = Command::new(executable);
    command.env_remove(crate::input_devices::WORKER_INPUT);
    if let Some(input) = input {
        command.env(crate::input_devices::WORKER_INPUT, input);
    }
    command
        .arg(mode)
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = crate::session_start::spawn(&mut command, permit.as_deref())?;
    let output = child.stdout.take().expect("piped worker stdout");
    state.generation += 1;
    let generation = state.generation;
    let child = Arc::new(Mutex::new(child));
    state.child = Some(child.clone());
    state.finishing = false;
    state.hold = permit
        .filter(|p| p.mode == crate::shortcut_gesture::Mode::Hold)
        .map(|p| p.id);
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
    sessions.finish()
}
