use crate::shortcut_edge::ShortcutEdge;
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, LazyLock, Mutex,
};

pub(crate) static GESTURES: LazyLock<Arc<Gestures>> =
    LazyLock::new(|| Arc::new(Gestures::default()));
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    #[default]
    Toggle,
    Hold,
}
impl Mode {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, &'static str> {
        match value {
            None | Some("toggle") => Ok(Self::Toggle),
            Some("hold") => Ok(Self::Hold),
            _ => Err("Choose toggle or hold-to-talk."),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Phase {
    Pressed,
    Released,
}
#[derive(Clone, Debug, Serialize)]
pub(crate) struct GestureEvent {
    pub(crate) id: String,
    pub(crate) phase: Phase,
    pub(crate) mode: Mode,
}
pub(crate) struct Permit {
    pub(crate) id: u64,
    pub(crate) mode: Mode,
    released: AtomicBool,
    invalid: AtomicBool,
    claimed: AtomicBool,
}
impl Permit {
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.invalid.load(Ordering::SeqCst)
            || (self.mode == Mode::Hold && self.released.load(Ordering::SeqCst))
        {
            return Err("Shortcut gesture ended".into());
        }
        Ok(())
    }
    fn event(&self, phase: Phase) -> GestureEvent {
        GestureEvent {
            id: self.id.to_string(),
            phase,
            mode: self.mode,
        }
    }
}
#[derive(Default)]
struct State {
    next: u64,
    active: u8,
    mode: Mode,
    held: u8,
    suspended: bool,
    current: Option<Arc<Permit>>,
}
impl State {
    fn invalidate(&mut self) {
        if let Some(permit) = self.current.take() {
            permit.invalid.store(true, Ordering::SeqCst);
        }
    }
}
#[derive(Default)]
pub(crate) struct Gestures(Mutex<State>);
impl Gestures {
    pub(crate) fn edge(
        &self,
        binding: u8,
        edge: &ShortcutEdge,
        pressed: bool,
    ) -> Option<GestureEvent> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if !edge.accept(pressed) {
            return None;
        }
        if pressed {
            state.held |= 1 << binding;
        } else {
            state.held &= !(1 << binding);
        }
        if state.active != binding || state.suspended {
            return None;
        }
        if pressed {
            state.invalidate();
            state.next = state.next.checked_add(1)?;
            let permit = Arc::new(Permit {
                id: state.next,
                mode: state.mode,
                released: AtomicBool::new(false),
                invalid: AtomicBool::new(false),
                claimed: AtomicBool::new(false),
            });
            let event = permit.event(Phase::Pressed);
            state.current = Some(permit);
            Some(event)
        } else {
            let permit = state.current.as_ref()?;
            permit.released.store(true, Ordering::SeqCst);
            Some(permit.event(Phase::Released))
        }
    }
    pub(crate) fn claim(&self, id: &str) -> Result<Arc<Permit>, String> {
        let state = self.0.lock().map_err(|_| "Shortcut unavailable")?;
        let permit = state
            .current
            .as_ref()
            .filter(|p| p.id.to_string() == id)
            .ok_or("Shortcut gesture expired")?;
        permit.check()?;
        if state.suspended || permit.claimed.swap(true, Ordering::SeqCst) {
            return Err("Shortcut gesture already used".into());
        }
        Ok(permit.clone())
    }
    pub(crate) fn forget(&self, binding: u8) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).held &= !(1 << binding);
    }
    pub(crate) fn invalidate(&self) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .invalidate();
    }
    pub(crate) fn configure(self: &Arc<Self>) -> Result<Configuration, &'static str> {
        let mut state = self.0.lock().map_err(|_| "Shortcut unavailable")?;
        if state.suspended || state.held != 0 {
            return Err("Release the shortcut keys before changing shortcut settings.");
        }
        state.suspended = true;
        state.invalidate();
        Ok(Configuration(self.clone()))
    }
}
pub(crate) struct Configuration(Arc<Gestures>);
impl Configuration {
    pub(crate) fn commit(self, active: u8, mode: Mode) {
        let mut state = self.0 .0.lock().unwrap_or_else(|e| e.into_inner());
        state.active = active;
        state.mode = mode;
    }
}
impl Drop for Configuration {
    fn drop(&mut self) {
        self.0
             .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .suspended = false;
    }
}
#[cfg(test)]
#[path = "shortcut_gesture_tests.rs"]
mod tests;
