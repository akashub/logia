use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct FakeRegistrar {
    bindings: HashMap<Preset, Binding>,
    register_failures: HashSet<Preset>,
    unregister_failures: HashSet<Preset>,
    press_on_register: bool,
    press_on_unregister: bool,
    callbacks: Vec<bool>,
    calls: Vec<(&'static str, Preset)>,
}
impl Registrar for FakeRegistrar {
    fn register(&mut self, preset: Preset, binding: Binding) -> Result<(), ()> {
        self.calls.push(("register", preset));
        if self.register_failures.contains(&preset) {
            return Err(());
        }
        let binding = self.bindings.entry(preset).or_insert(binding);
        if self.press_on_register {
            self.callbacks.push(binding.accept(true).is_some());
        }
        Ok(())
    }
    fn unregister(&mut self, preset: Preset) -> Result<(), ()> {
        self.calls.push(("unregister", preset));
        if self.unregister_failures.contains(&preset) {
            return Err(());
        }
        if self.press_on_unregister {
            if let Some(binding) = self.bindings.get(&preset) {
                self.callbacks.push(binding.accept(true).is_some());
            }
        }
        self.bindings.remove(&preset);
        Ok(())
    }
}
impl FakeRegistrar {
    fn press(&self, preset: Preset) -> bool {
        self.bindings
            .get(&preset)
            .is_some_and(|binding| binding.accept(true).is_some())
    }
    fn release(&self, preset: Preset) {
        if let Some(binding) = self.bindings.get(&preset) {
            binding.accept(false);
        }
    }
}

#[test]
fn preserves_default_and_rejects_unsupported_shortcuts() {
    assert_eq!(Preset::parse(None), Ok(Preset::ControlAlt));
    for (value, expected) in [
        ("Control+Alt+Space", Preset::ControlAlt),
        ("Control+Shift+Space", Preset::ControlShift),
        ("Alt+Shift+Space", Preset::AltShift),
    ] {
        assert_eq!(Preset::parse(Some(value)), Ok(expected));
        assert_eq!(expected.shortcut(), value);
    }
    for value in [
        "",
        "Space",
        "Control++Space",
        "CommandOrControl+Shift+Space",
    ] {
        assert!(Preset::parse(Some(value)).is_err());
    }
}

#[test]
fn labels_match_the_registered_modifiers() {
    assert_eq!(Preset::ControlAlt.label(true), "⌃ ⌥ Space");
    assert_eq!(Preset::ControlShift.label(true), "⌃ ⇧ Space");
    assert_eq!(Preset::AltShift.label(true), "⌥ ⇧ Space");
    assert_eq!(Preset::ControlShift.label(false), "Ctrl Shift Space");
}

#[test]
fn registering_and_retrying_same_binding_emits_once_per_press() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    assert_eq!(registrar.calls, [("register", Preset::ControlAlt)]);
    assert!(registrar.press(Preset::ControlAlt));
    for _ in 0..20 {
        assert!(!registrar.press(Preset::ControlAlt));
    }
    registrar.release(Preset::ControlAlt);
    assert!(registrar.press(Preset::ControlAlt));
}

#[test]
fn replacement_registers_before_removing_old_and_only_new_binding_works() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    registrar.calls.clear();
    selection
        .select(Preset::AltShift, Mode::Toggle, &mut registrar)
        .unwrap();
    assert_eq!(
        registrar.calls,
        [
            ("register", Preset::AltShift),
            ("unregister", Preset::ControlAlt)
        ]
    );
    assert!(!registrar.press(Preset::ControlAlt));
    assert!(registrar.press(Preset::AltShift));
    assert_eq!(registrar.bindings.len(), 1);
}

#[test]
fn registration_conflict_preserves_working_old_binding_and_can_be_retried() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    registrar.register_failures.insert(Preset::ControlShift);
    assert!(selection
        .select(Preset::ControlShift, Mode::Toggle, &mut registrar)
        .is_err());
    assert_eq!(selection.current, Some(Preset::ControlAlt));
    assert!(registrar.press(Preset::ControlAlt));
    assert_eq!(registrar.bindings.len(), 1);
    registrar.release(Preset::ControlAlt);
    registrar.register_failures.clear();
    selection
        .select(Preset::ControlShift, Mode::Toggle, &mut registrar)
        .unwrap();
    assert!(registrar.press(Preset::ControlShift));
}

#[test]
fn first_registration_failure_does_not_claim_an_active_binding() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    registrar.register_failures.insert(Preset::ControlAlt);
    assert!(selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .is_err());
    assert_eq!(selection.current, None);
    assert!(registrar.bindings.is_empty());
}

#[test]
fn old_removal_failure_rolls_back_replacement_without_disabling_old() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    registrar.unregister_failures.insert(Preset::ControlAlt);
    assert!(selection
        .select(Preset::AltShift, Mode::Toggle, &mut registrar)
        .is_err());
    assert_eq!(selection.current, Some(Preset::ControlAlt));
    assert!(registrar.press(Preset::ControlAlt));
    assert!(!registrar.press(Preset::AltShift));
    assert_eq!(registrar.bindings.len(), 1);
}

#[test]
fn rollback_failure_keeps_extra_binding_inert_and_retry_can_recover() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    registrar
        .unregister_failures
        .extend([Preset::ControlAlt, Preset::AltShift]);
    assert!(selection
        .select(Preset::AltShift, Mode::Toggle, &mut registrar)
        .is_err());
    assert_eq!(registrar.bindings.len(), 2);
    assert!(registrar.press(Preset::ControlAlt));
    assert!(!registrar.press(Preset::AltShift));
    registrar.release(Preset::ControlAlt);
    registrar.release(Preset::AltShift);
    registrar.unregister_failures.clear();
    selection
        .select(Preset::AltShift, Mode::Toggle, &mut registrar)
        .unwrap();
    assert_eq!(registrar.bindings.len(), 1);
    assert!(!registrar.press(Preset::ControlAlt));
    assert!(registrar.press(Preset::AltShift));
}

#[test]
fn conflicting_shortcut_and_mode_change_preserves_previous_mode() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Hold, &mut registrar)
        .unwrap();
    registrar.register_failures.insert(Preset::ControlShift);
    assert!(selection
        .select(Preset::ControlShift, Mode::Toggle, &mut registrar)
        .is_err());
    let event = registrar.bindings[&Preset::ControlAlt]
        .accept(true)
        .unwrap();
    assert_eq!(event.mode, Mode::Hold);
    let release = registrar.bindings[&Preset::ControlAlt]
        .accept(false)
        .unwrap();
    assert_eq!(event.id, release.id);
    assert!(selection.gestures.claim(&event.id).is_err());
}

#[test]
fn mode_changes_require_release_and_do_not_reregister() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    assert!(registrar.press(Preset::ControlAlt));
    assert!(selection
        .select(Preset::ControlAlt, Mode::Hold, &mut registrar)
        .is_err());
    registrar.release(Preset::ControlAlt);
    selection
        .select(Preset::ControlAlt, Mode::Hold, &mut registrar)
        .unwrap();
    assert_eq!(registrar.calls, [("register", Preset::ControlAlt)]);
    assert_eq!(
        registrar.bindings[&Preset::ControlAlt]
            .accept(true)
            .unwrap()
            .mode,
        Mode::Hold
    );
}

#[test]
fn callbacks_during_registration_cannot_emit_or_make_repeat_a_new_press() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    registrar.press_on_register = true;
    selection
        .select(Preset::AltShift, Mode::Hold, &mut registrar)
        .unwrap();
    assert_eq!(registrar.callbacks, [false]);
    assert!(!registrar.press(Preset::AltShift));
    assert!(selection
        .select(Preset::AltShift, Mode::Toggle, &mut registrar)
        .is_err());
    assert!(registrar.bindings[&Preset::AltShift]
        .accept(false)
        .is_none());
    assert_eq!(
        registrar.bindings[&Preset::AltShift]
            .accept(true)
            .unwrap()
            .mode,
        Mode::Hold
    );
}

#[test]
fn unregistering_a_key_pressed_during_configuration_does_not_leave_a_stuck_key() {
    let (mut selection, mut registrar) = (Selection::default(), FakeRegistrar::default());
    selection
        .select(Preset::ControlAlt, Mode::Toggle, &mut registrar)
        .unwrap();
    registrar.press_on_unregister = true;
    selection
        .select(Preset::AltShift, Mode::Hold, &mut registrar)
        .unwrap();
    assert_eq!(registrar.callbacks, [false]);
    selection
        .select(Preset::AltShift, Mode::Toggle, &mut registrar)
        .unwrap();
    assert_eq!(
        registrar.bindings[&Preset::AltShift]
            .accept(true)
            .unwrap()
            .mode,
        Mode::Toggle
    );
}
