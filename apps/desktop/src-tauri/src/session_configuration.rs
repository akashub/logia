use crate::session::{Inner, Sessions};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// The operation owns this reservation even if an IPC caller stops waiting.
// No Sessions lock is held while the shortcut plugin dispatches to main.
pub(crate) struct ShortcutConfiguration(Arc<AtomicBool>);
impl Drop for ShortcutConfiguration {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl Inner {
    pub(crate) fn require_idle(&self) -> Result<(), String> {
        if self.shortcut_configuration.load(Ordering::Acquire) {
            return Err("Wait for the shortcut change to finish.".into());
        }
        if self.child.is_some() {
            return Err("Wait for recognition to stop, or choose Cancel, before changing settings or dismissing Logia.".into());
        }
        Ok(())
    }
}
impl Sessions {
    pub(crate) fn reserve_shortcut(&self) -> Result<ShortcutConfiguration, String> {
        let state = self.0.lock().map_err(|_| "Session state unavailable")?;
        state.require_idle()?;
        state.shortcut_configuration.store(true, Ordering::Release);
        Ok(ShortcutConfiguration(state.shortcut_configuration.clone()))
    }
}
#[cfg(test)]
#[path = "session_configuration_tests.rs"]
mod tests;
