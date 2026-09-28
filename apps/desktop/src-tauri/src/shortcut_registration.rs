use crate::{
    shortcut_edge::ShortcutEdge,
    shortcut_gesture::{GestureEvent, Gestures, Mode},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Preset {
    ControlAlt = 1,
    ControlShift = 2,
    AltShift = 3,
}
impl Preset {
    pub(super) fn parse(value: Option<&str>) -> Result<Self, &'static str> {
        match value {
            None | Some("Control+Alt+Space") => Ok(Self::ControlAlt),
            Some("Control+Shift+Space") => Ok(Self::ControlShift),
            Some("Alt+Shift+Space") => Ok(Self::AltShift),
            _ => Err("Choose one of the available shortcuts."),
        }
    }
    pub(super) fn shortcut(self) -> &'static str {
        match self {
            Self::ControlAlt => "Control+Alt+Space",
            Self::ControlShift => "Control+Shift+Space",
            Self::AltShift => "Alt+Shift+Space",
        }
    }
    pub(super) fn label(self, mac: bool) -> &'static str {
        match (self, mac) {
            (Self::ControlAlt, true) => "⌃ ⌥ Space",
            (Self::ControlShift, true) => "⌃ ⇧ Space",
            (Self::AltShift, true) => "⌥ ⇧ Space",
            (Self::ControlAlt, false) => "Ctrl Alt Space",
            (Self::ControlShift, false) => "Ctrl Shift Space",
            (Self::AltShift, false) => "Alt Shift Space",
        }
    }
}
pub(super) struct Binding {
    edge: ShortcutEdge,
    gestures: Arc<Gestures>,
    preset: Preset,
}
impl Binding {
    pub(super) fn accept(&self, pressed: bool) -> Option<GestureEvent> {
        // Track both edges while inactive or suspended, so a held key's
        // auto-repeat cannot become a fresh gesture after configuration.
        self.gestures.edge(self.preset as u8, &self.edge, pressed)
    }
}
pub(super) trait Registrar {
    fn register(&mut self, preset: Preset, binding: Binding) -> Result<(), ()>;
    fn unregister(&mut self, preset: Preset) -> Result<(), ()>;
}
#[derive(Default)]
pub(super) struct Selection {
    current: Option<Preset>,
    pending_cleanup: Option<Preset>,
    gestures: Arc<Gestures>,
}
impl Selection {
    pub(super) fn new(gestures: Arc<Gestures>) -> Self {
        Self {
            gestures,
            ..Self::default()
        }
    }
    fn unregister(&self, preset: Preset, registrar: &mut impl Registrar) -> Result<(), ()> {
        registrar.unregister(preset)?;
        self.gestures.forget(preset as u8);
        Ok(())
    }
    pub(super) fn select(
        &mut self,
        next: Preset,
        mode: Mode,
        registrar: &mut impl Registrar,
    ) -> Result<(), &'static str> {
        let configuration = self.gestures.configure()?;
        if let Some(pending) = self.pending_cleanup.filter(|pending| *pending != next) {
            self.unregister(pending, registrar).map_err(|_| "Could not clean up the previous shortcut change. Your existing shortcut is still active. Retry, or quit and reopen Logia.")?;
            self.pending_cleanup = None;
        }
        if self.current == Some(next) {
            configuration.commit(next as u8, mode);
            return Ok(());
        }
        registrar.register(next, Binding { edge: ShortcutEdge::default(), gestures: self.gestures.clone(), preset: next })
            .map_err(|_| "Could not register the shortcut. It may be in use by another app. Your existing shortcut has not changed.")?;
        if let Some(previous) = self.current {
            if self.unregister(previous, registrar).is_err() {
                if self.unregister(next, registrar).is_err() {
                    self.pending_cleanup = Some(next);
                    return Err("Could not finish the shortcut change. Your existing shortcut is still active. Retry, or quit and reopen Logia to release the unused shortcut.");
                }
                self.pending_cleanup = None;
                return Err("Could not release the old shortcut. Your existing shortcut is still active. Retry the change.");
            }
        }
        self.current = Some(next);
        self.pending_cleanup = None;
        configuration.commit(next as u8, mode);
        Ok(())
    }
}

#[cfg(test)]
#[path = "shortcut_registration_tests.rs"]
mod tests;
