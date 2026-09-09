use crate::{
    final_delivery::FinalDelivery,
    messages::{WorkerEvent, MAX_EVENT_BYTES},
    session::{Sessions, Update},
    target,
};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Child, ChildStdout},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::Emitter;

#[derive(serde::Serialize, Clone)]
struct DeliveryUpdate {
    generation: u64,
    status: &'static str,
}

pub fn read(
    app: tauri::AppHandle,
    sessions: Sessions,
    generation: u64,
    child: Arc<Mutex<Child>>,
    output: ChildStdout,
) {
    let mut reader = BufReader::new(output);
    let mut failed = false;
    let mut delivery = FinalDelivery::default();
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
        if !delivery.observe(&event) {
            failed = true;
            break;
        }
        let state = sessions.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == generation {
            let _ = app.emit("recognition", Update { generation, event });
        }
    }
    if failed {
        let _ = child.lock().unwrap_or_else(|e| e.into_inner()).kill();
    }
    // Never hold the child lock during wait: Cancel must still get it.
    let exit = loop {
        match child.lock().unwrap_or_else(|e| e.into_inner()).try_wait() {
            Ok(Some(status)) => break Some(status),
            Err(_) => break None,
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let clean = !failed && exit.is_some_and(|status| status.success());
    let text = delivery.take(clean);
    let main_app = app.clone();
    let fallback_sessions = sessions.clone();
    // Keep the worker slot occupied until this closure completes. Cancellation
    // invalidates generation before it queues cleanup, so queued sends are inert.
    let scheduled = app.run_on_main_thread(move || {
        sessions.with_current(generation, |state| {
        let token = state.target;
        let status = if let Some(text) = text.as_ref() {
            if token != 0 { let _ = main_app.emit("delivery", DeliveryUpdate { generation, status: "sending" }); }
            target::send(token, text)
        }
            else { target::discard(token); "copy" };
        state.target = 0;
        if status == "sent" || status == "uncertain" {
            state.delivered = Some(crate::session::DeliveryOutcome { status, text });
        }
        if token != 0 { let _ = main_app.emit("delivery", DeliveryUpdate { generation, status }); }
        if !clean {
            let _ = main_app.emit("recognition", Update { generation, event: WorkerEvent::Error {
                message: "Recognition stopped unexpectedly. Any partial text remains available.".into()
            } });
        }
        if exit.is_some() {
            state.child = None;
            state.finishing = false;
            let _ = main_app.emit("recognition", Update { generation, event: WorkerEvent::Stopped });
        }
        });
    });
    if scheduled.is_err() {
        // No write occurred. Keep the slot blocked until explicit cancellation.
        let state = fallback_sessions
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if state.generation == generation {
            let _ = app.emit(
                "recognition",
                Update {
                    generation,
                    event: WorkerEvent::Error {
                        message: "Could not finish delivery. Use Cancel, then copy your text."
                            .into(),
                    },
                },
            );
        }
    }
}
