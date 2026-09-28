use super::*;
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

// A real stdin/stdout child exercises the production finish writer without
// compiling in a substitute recognition path or opening a microphone.
fn worker(hold: Option<u64>) -> Sessions {
    let child = Command::new("/bin/cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    Sessions(Arc::new(Mutex::new(Inner {
        generation: 7,
        child: Some(Arc::new(Mutex::new(child))),
        hold,
        ..Inner::default()
    })))
}
fn output(sessions: &Sessions) -> String {
    let child = sessions.0.lock().unwrap().child.take().unwrap();
    let mut child = child.lock().unwrap();
    drop(child.stdin.take());
    let mut result = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut result)
        .unwrap();
    assert!(child.wait().unwrap().success());
    result
}
#[test]
fn hold_release_finishes_loading_worker_once_even_with_ui_stop_and_duplicate_release() {
    let sessions = worker(Some(41));
    sessions.finish_hold(41).unwrap();
    assert!(
        sessions.0.lock().unwrap().finishing,
        "release must finish without a webview stop or Listening event"
    );
    sessions.finish().unwrap();
    sessions.finish_hold(41).unwrap();
    assert_eq!(output(&sessions), "finish\n");
}
#[test]
fn stale_release_cannot_finish_replacement_or_button_started_worker() {
    for hold in [Some(42), None] {
        let sessions = worker(hold);
        sessions.finish_hold(41).unwrap();
        assert!(!sessions.0.lock().unwrap().finishing);
        assert_eq!(output(&sessions), "");
    }
}
#[test]
fn late_release_waiting_for_session_lock_observes_replacement_ownership() {
    let sessions = worker(Some(41));
    let mut state = sessions.0.lock().unwrap();
    let pending = sessions.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let release = std::thread::spawn(move || {
        tx.send(()).unwrap();
        pending.finish_hold(41).unwrap();
    });
    rx.recv().unwrap();
    state.hold = Some(42);
    state.generation += 1;
    drop(state);
    release.join().unwrap();
    assert_eq!(output(&sessions), "");
}

#[test]
fn release_during_spawn_waits_for_new_worker_ownership_and_finishes_it() {
    let sessions = worker(None);
    let mut state = sessions.0.lock().unwrap();
    let pending = sessions.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let release = std::thread::spawn(move || {
        tx.send(()).unwrap();
        pending.finish_hold(41).unwrap();
    });
    rx.recv().unwrap();
    state.hold = Some(41);
    drop(state);
    release.join().unwrap();
    assert_eq!(output(&sessions), "finish\n");
}
