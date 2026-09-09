use crate::{
    messages::{WorkerEvent, MAX_EVENT_BYTES},
    model_file,
};
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::Emitter;

#[derive(Default)]
struct Inner {
    generation: u64,
    child: Option<Arc<Mutex<Child>>>,
    finishing: bool,
}
#[derive(Clone, Default)]
pub struct Sessions(Arc<Mutex<Inner>>);
impl Sessions {
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
struct Update {
    generation: u64,
    event: WorkerEvent,
}

#[tauri::command]
pub fn start_recording(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
) -> Result<u64, String> {
    launch(app, sessions, "--recognizer")
}

#[tauri::command]
pub fn warmup_recognizer(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
) -> Result<u64, String> {
    launch(app, sessions, "--warmup")
}

fn launch(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
    mode: &str,
) -> Result<u64, String> {
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
    let sessions = sessions.inner().clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(output);
        let mut failed = false;
        loop {
            let mut line = String::new();
            match (&mut reader).take(MAX_EVENT_BYTES + 2).read_line(&mut line) {
                Ok(0) => break,
                Ok(_) if line.ends_with('\n') && line.len() as u64 <= MAX_EVENT_BYTES + 1 => (),
                _ => {
                    failed = true;
                    break;
                }
            }
            let Ok(event) = serde_json::from_str::<WorkerEvent>(&line) else {
                failed = true;
                break;
            };
            let state = sessions.0.lock().unwrap_or_else(|e| e.into_inner());
            if state.generation == generation {
                let _ = app.emit("recognition", Update { generation, event });
            }
        }
        if failed {
            let _ = child.lock().unwrap_or_else(|e| e.into_inner()).kill();
        }
        // Never hold the child lock in blocking wait: Cancel must still get it.
        let exit = loop {
            match child.lock().unwrap_or_else(|e| e.into_inner()).try_wait() {
                Ok(Some(status)) => break Some(status),
                Err(_) => break None,
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            }
        };
        let mut state = sessions.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == generation {
            if exit.is_some() {
                state.child = None;
            }
            if failed || exit.is_none_or(|status| !status.success()) {
                let _ = app.emit("recognition", Update { generation, event: WorkerEvent::Error { message: "Recognition stopped unexpectedly. Any partial text remains available.".into() } });
            }
            if exit.is_some() {
                let _ = app.emit(
                    "recognition",
                    Update {
                        generation,
                        event: WorkerEvent::Stopped,
                    },
                );
            }
        }
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

pub fn terminate(sessions: &Sessions) -> Result<u64, String> {
    let (child, generation) = {
        let mut state = sessions.0.lock().map_err(|_| "Session state unavailable")?;
        state.generation += 1; // Reject old events before terminating native work.
        (state.child.clone(), state.generation)
    };
    if let Some(child) = child {
        let deadline = Instant::now() + Duration::from_secs(2);
        child
            .lock()
            .map_err(|_| "Worker unavailable")?
            .kill()
            .map_err(|_| "Could not terminate recognition")?;
        loop {
            if child
                .lock()
                .map_err(|_| "Worker unavailable")?
                .try_wait()
                .map_err(|_| "Could not verify worker exit")?
                .is_some()
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err(
                    "Recognition has not stopped yet; starting another recording is blocked".into(),
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut state = sessions.0.lock().map_err(|_| "Session state unavailable")?;
        if state.generation == generation {
            state.child = None;
            state.finishing = false;
        }
    }
    Ok(generation)
}

#[tauri::command]
pub async fn cancel_recording(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
) -> Result<(), String> {
    let sessions = sessions.inner().clone();
    let generation = tauri::async_runtime::spawn_blocking(move || terminate(&sessions))
        .await
        .map_err(|_| "Could not stop recognition")??;
    let _ = app.emit(
        "recognition",
        Update {
            generation,
            event: WorkerEvent::Stopped,
        },
    );
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn cancel_reaps_real_owned_process_before_allowing_replacement() {
        let child = Arc::new(Mutex::new(
            Command::new("/bin/sleep").arg("60").spawn().unwrap(),
        ));
        let sessions = Sessions(Arc::new(Mutex::new(Inner {
            generation: 7,
            child: Some(child.clone()),
            finishing: true,
        })));
        let started = Instant::now();
        assert!(
            sessions.when_idle(|| Ok(())).is_err(),
            "active recognition cannot be hidden"
        );
        assert_eq!(terminate(&sessions).unwrap(), 8);
        assert!(sessions.when_idle(|| Ok(())).is_ok());
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(child.lock().unwrap().try_wait().unwrap().is_some());
        let state = sessions.0.lock().unwrap();
        assert!(state.child.is_none());
        assert!(!state.finishing);
        assert_eq!(state.generation, 8);
    }
}
