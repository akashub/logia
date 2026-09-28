use crate::session::{Inner, Sessions};
use std::io::Write;

impl Sessions {
    pub(crate) fn finish_hold(&self, id: u64) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| "Session state unavailable")?;
        if state.hold == Some(id) {
            finish_locked(&mut state)?;
        }
        Ok(())
    }
    pub(crate) fn finish(&self) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| "Session state unavailable")?;
        finish_locked(&mut state)
    }
}
fn finish_locked(state: &mut Inner) -> Result<(), String> {
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
#[cfg(all(test, unix))]
#[path = "session_finish_tests.rs"]
mod tests;
