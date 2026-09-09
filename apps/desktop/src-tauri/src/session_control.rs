use crate::{
    messages::WorkerEvent,
    session::{DeliveryOutcome, Sessions, Update},
};
use std::{
    process::Child,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::Emitter;

struct Canceled {
    generation: u64,
    target: u64,
    child: Option<Arc<Mutex<Child>>>,
}

fn invalidate(sessions: &Sessions) -> Result<Canceled, String> {
    let mut state = sessions.0.lock().map_err(|_| "Session state unavailable")?;
    Ok(invalidate_locked(&mut state))
}

fn invalidate_locked(state: &mut crate::session::Inner) -> Canceled {
    state.generation += 1;
    let canceled = Canceled {
        generation: state.generation,
        target: state.target,
        child: state.child.clone(),
    };
    state.target = 0;
    canceled
}

enum CancelRequest {
    Stop(Canceled),
    Delivered(u64, DeliveryOutcome),
}
fn request_cancel(sessions: &Sessions) -> Result<CancelRequest, String> {
    let mut state = sessions.0.lock().map_err(|_| "Session state unavailable")?;
    // An external write cannot be retracted. Cancel racing that commit reports
    // the outcome so the UI retains the paragraph rather than erasing evidence.
    if let Some(outcome) = &state.delivered {
        return Ok(CancelRequest::Delivered(state.generation, outcome.clone()));
    }
    Ok(CancelRequest::Stop(invalidate_locked(&mut state)))
}

pub fn terminate(sessions: &Sessions) -> Result<u64, String> {
    let canceled = invalidate(sessions)?;
    reap(sessions, canceled)
}

fn reap(sessions: &Sessions, canceled: Canceled) -> Result<u64, String> {
    let generation = canceled.generation;
    if let Some(child) = canceled.child {
        let deadline = Instant::now() + Duration::from_secs(2);
        {
            let mut child = child.lock().map_err(|_| "Worker unavailable")?;
            if child
                .try_wait()
                .map_err(|_| "Could not check recognition")?
                .is_none()
            {
                child
                    .kill()
                    .map_err(|_| "Could not terminate recognition")?;
            }
        }
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
) -> Result<DeliveryOutcome, String> {
    let canceled = match request_cancel(&sessions)? {
        CancelRequest::Delivered(generation, status) => {
            let _ = app.emit(
                "recognition",
                Update {
                    generation,
                    event: WorkerEvent::Stopped,
                },
            );
            return Ok(status);
        }
        CancelRequest::Stop(canceled) => canceled,
    };
    let token = canceled.target;
    // Generation already invalidated: even if dispatch fails, stop owned inference.
    let _ = app.run_on_main_thread(move || crate::target::discard(token));
    let sessions = sessions.inner().clone();
    let generation = tauri::async_runtime::spawn_blocking(move || reap(&sessions, canceled))
        .await
        .map_err(|_| "Could not stop recognition")??;
    let _ = app.emit(
        "recognition",
        Update {
            generation,
            event: WorkerEvent::Stopped,
        },
    );
    Ok(DeliveryOutcome {
        status: "canceled",
        text: None,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::session::Inner;
    use std::process::Command;
    #[test]
    fn cancel_during_committed_delivery_reports_outcome_and_final_text() {
        let sessions = Sessions::default();
        let sending = sessions.clone();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            sending.with_current(0, |state| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap(); // Simulate AX verification in progress.
                state.delivered = Some(DeliveryOutcome {
                    status: "uncertain",
                    text: Some("Authoritative final".into()),
                });
            })
        });
        entered_rx.recv().unwrap();
        let canceling = sessions.clone();
        let cancel = std::thread::spawn(move || request_cancel(&canceling).unwrap());
        release_tx.send(()).unwrap();
        writer.join().unwrap();
        let CancelRequest::Delivered(generation, outcome) = cancel.join().unwrap() else {
            panic!("Committed send appeared canceled");
        };
        assert_eq!(generation, 0);
        assert_eq!(outcome.status, "uncertain");
        assert_eq!(outcome.text.as_deref(), Some("Authoritative final"));
    }
    #[test]
    fn cancel_invalidates_queued_delivery_and_cleanup_cannot_reap_a_replacement() {
        let sessions = Sessions::default();
        let old_generation = 0;
        let canceled = invalidate(&sessions).unwrap();
        assert!(sessions
            .with_current(old_generation, |_| panic!("canceled write executed"))
            .is_none());
        let child = Arc::new(Mutex::new(
            Command::new("/bin/sleep").arg("60").spawn().unwrap(),
        ));
        {
            let mut state = sessions.0.lock().unwrap();
            state.generation += 1;
            state.child = Some(child.clone());
        }
        reap(&sessions, canceled).unwrap();
        assert!(
            child.lock().unwrap().try_wait().unwrap().is_none(),
            "stale cleanup killed new worker"
        );
        terminate(&sessions).unwrap();
    }
    #[test]
    fn cancel_reaps_real_owned_process_before_allowing_replacement() {
        let child = Arc::new(Mutex::new(
            Command::new("/bin/sleep").arg("60").spawn().unwrap(),
        ));
        let sessions = Sessions(Arc::new(Mutex::new(Inner {
            generation: 7,
            child: Some(child.clone()),
            finishing: true,
            target: 0,
            delivered: None,
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
