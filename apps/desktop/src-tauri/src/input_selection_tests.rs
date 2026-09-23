use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "logia-input-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn input(id: &str) -> InputDevice {
    InputDevice {
        id: id.into(),
        name: "Same microphone name".into(),
    }
}

#[test]
fn saved_identity_survives_reordering_and_never_matches_by_name() {
    let first = input("first");
    let second = input("second");
    let devices = [second.clone(), first.clone()];
    assert_eq!(
        resolve(Some(&first), &devices, Some("second")).unwrap().id,
        "first"
    );
    assert!(resolve(Some(&first), &[second], Some("second")).is_err());
}
#[test]
fn default_is_resolved_once_and_unknown_defaults_do_not_pick_an_arbitrary_input() {
    let devices = [input("first"), input("second")];
    let snapshot = resolve(None, &devices, Some("second")).unwrap().clone();
    assert_eq!(
        resolve(Some(&snapshot), &devices, Some("first"))
            .unwrap()
            .id,
        "second"
    );
    assert!(resolve(None, &devices, None).is_err());
    assert!(resolve(None, &devices, Some("missing")).is_err());
    assert!(resolve(None, &[], Some("second")).is_err());
}
#[test]
fn unavailable_and_ambiguous_ids_are_refused() {
    let selected = input("first");
    assert!(resolve(Some(&selected), &[], None).is_err());
    assert!(resolve(Some(&selected), &[selected.clone(), selected.clone()], None).is_err());
}
#[test]
fn choice_persists_and_explicit_system_default_can_repair_corrupt_config() {
    let dir = Temp::new();
    assert_eq!(read(&dir.0).unwrap(), None);
    let chosen = input("second");
    save(&dir.0, Some(&chosen)).unwrap();
    assert_eq!(read(&dir.0).unwrap(), Some(chosen));
    std::fs::write(dir.0.join(FILE), b"{broken}").unwrap();
    assert!(read(&dir.0).is_err());
    save(&dir.0, None).unwrap();
    assert_eq!(read(&dir.0).unwrap(), None);
}
#[test]
fn malformed_oversized_and_future_settings_never_silently_become_system_default() {
    let dir = Temp::new();
    for bytes in [
        vec![b'x'; 9000],
        br#"{"version":9,"selected":null}"#.to_vec(),
        br#"{"version":1}"#.to_vec(),
        br#"{"version":1,"selected":{"id":"","name":"Mic"}}"#.to_vec(),
    ] {
        std::fs::write(dir.0.join(FILE), bytes).unwrap();
        assert!(read(&dir.0).is_err());
    }
}
#[test]
fn failed_save_preserves_the_previous_choice() {
    let dir = Temp::new();
    let first = input("first");
    save(&dir.0, Some(&first)).unwrap();
    std::fs::create_dir(dir.0.join("input-device.json.partial")).unwrap();
    assert!(save(&dir.0, Some(&input("second"))).is_err());
    assert_eq!(read(&dir.0).unwrap(), Some(first));
}
