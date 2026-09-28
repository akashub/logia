use super::*;

#[test]
fn shortcut_reservation_excludes_launch_and_mutation_without_holding_sessions() {
    let sessions = Sessions::default();
    let reservation = sessions.reserve_shortcut().unwrap();
    {
        let state = sessions
            .0
            .try_lock()
            .expect("main-thread checks must not wait on plugin registration");
        assert!(
            state.require_idle().is_err(),
            "a recording or warmup may not launch during registration"
        );
    }
    assert!(sessions.reserve_shortcut().is_err());
    let mut mutated = false;
    assert!(sessions
        .when_idle(|| {
            mutated = true;
            Ok(())
        })
        .is_err());
    assert!(!mutated);
    drop(reservation);
    assert!(sessions.0.lock().unwrap().require_idle().is_ok());
    assert!(sessions.when_idle(|| Ok(())).is_ok());
    assert!(sessions.reserve_shortcut().is_ok());
}

#[test]
fn registration_failure_releases_reservation() {
    let sessions = Sessions::default();
    let result: Result<(), String> = (|| {
        let _reservation = sessions.reserve_shortcut()?;
        Err("synthetic registration failure".into())
    })();
    assert!(result.is_err());
    assert!(sessions.reserve_shortcut().is_ok());
}

#[test]
fn dropped_caller_does_not_release_a_blocking_registration_operation() {
    let sessions = Sessions::default();
    let reservation = sessions.reserve_shortcut().unwrap();
    let (release, wait) = std::sync::mpsc::channel();
    let (finished, done) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        wait.recv().unwrap();
        drop(reservation);
        finished.send(()).unwrap();
    });
    drop(handle);
    assert!(sessions.reserve_shortcut().is_err());
    assert!(sessions.when_idle(|| Ok(())).is_err());
    release.send(()).unwrap();
    done.recv().unwrap();
    assert!(sessions.reserve_shortcut().is_ok());
}

#[cfg(unix)]
#[test]
fn active_worker_cannot_reserve_shortcut_configuration() {
    use std::{process::Command, sync::Mutex};
    let child = Arc::new(Mutex::new(
        Command::new("/bin/sleep").arg("60").spawn().unwrap(),
    ));
    let sessions = Sessions(Arc::new(Mutex::new(Inner {
        child: Some(child),
        ..Inner::default()
    })));
    assert!(sessions.reserve_shortcut().is_err());
    crate::session_control::terminate(&sessions).unwrap();
    assert!(sessions.reserve_shortcut().is_ok());
}
