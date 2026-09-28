use crate::shortcut_gesture::Permit;
use std::process::{Child, Command};

pub(crate) fn spawn(command: &mut Command, permit: Option<&Permit>) -> Result<Child, String> {
    // Setup (model/input snapshot) can outlast a complete key gesture. Keep
    // this check immediately beside spawn while the caller owns Sessions.
    if let Some(permit) = permit {
        permit.check()?;
    }
    command
        .spawn()
        .map_err(|_| "Could not start recognition".into())
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::{
        shortcut_edge::ShortcutEdge,
        shortcut_gesture::{Gestures, Mode},
    };
    use std::sync::Arc;
    #[test]
    fn release_during_setup_prevents_actual_worker_spawn() {
        let gestures = Arc::new(Gestures::default());
        gestures.configure().unwrap().commit(1, Mode::Hold);
        let edge = ShortcutEdge::default();
        let event = gestures.edge(1, &edge, true).unwrap();
        let permit = gestures.claim(&event.id).unwrap();
        // Command setup happens after the native claim and can race release.
        let mut command = Command::new("/usr/bin/true");
        gestures.edge(1, &edge, false);
        let result = spawn(&mut command, Some(&permit));
        if let Ok(mut child) = result {
            let _ = child.wait();
            panic!("released hold started a worker");
        }
    }
    #[test]
    fn canceled_claim_cannot_spawn_but_toggle_release_and_voice_test_can() {
        let gestures = Arc::new(Gestures::default());
        gestures.configure().unwrap().commit(1, Mode::Toggle);
        let edge = ShortcutEdge::default();
        let event = gestures.edge(1, &edge, true).unwrap();
        let permit = gestures.claim(&event.id).unwrap();
        gestures.edge(1, &edge, false);
        assert!(spawn(&mut Command::new("/usr/bin/true"), Some(&permit))
            .unwrap()
            .wait()
            .unwrap()
            .success());
        gestures.invalidate();
        let result = spawn(&mut Command::new("/usr/bin/true"), Some(&permit));
        if let Ok(mut child) = result {
            let _ = child.wait();
            panic!("canceled gesture started a worker");
        }
        assert!(spawn(&mut Command::new("/usr/bin/true"), None)
            .unwrap()
            .wait()
            .unwrap()
            .success());
    }
}
