use super::*;
use std::sync::atomic::AtomicUsize;
use std::path::PathBuf;
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("logia-model-store-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap(); Self(dir)
    }
}
impl Drop for Temp { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
fn fixture() -> Model {
    Model { bytes: 3, sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", ..catalog::MODELS[1] }
}

#[test]
fn missing_and_invalid_selection_fall_back_to_the_known_default() {
    let dir = Temp::new();
    assert_eq!(selected(&dir.0).id, catalog::DEFAULT_ID);
    std::fs::write(dir.0.join("selected-model"), "../../other.gguf").unwrap();
    assert_eq!(selected(&dir.0).id, catalog::DEFAULT_ID);
}
#[test]
fn selection_persists_the_verified_model_and_cannot_activate_corrupt_bytes() {
    let dir = Temp::new(); let model = fixture();
    std::fs::write(dir.0.join(model.file), b"abc").unwrap();
    select(&dir.0, &model).unwrap();
    assert_eq!(selected(&dir.0).id, model.id);
    std::fs::write(dir.0.join(model.file), b"abd").unwrap();
    assert!(select(&dir.0, &model).is_err());
    assert_eq!(selected(&dir.0).id, model.id);
}
#[test]
fn integrity_checks_reject_missing_wrong_size_and_same_size_substitution() {
    let dir = Temp::new(); let model = fixture(); let path = dir.0.join(model.file);
    assert!(verify_model(&path, &model).is_err());
    std::fs::write(&path, b"ab").unwrap(); assert!(verify_model(&path, &model).is_err());
    std::fs::write(&path, b"abd").unwrap(); assert!(verify_model(&path, &model).is_err());
    std::fs::write(&path, b"abc").unwrap(); assert!(verify_model(&path, &model).is_ok());
}
#[test]
fn selected_model_cannot_be_removed_but_an_inactive_model_can() {
    let dir = Temp::new(); let model = fixture();
    std::fs::write(dir.0.join(model.file), b"abc").unwrap(); select(&dir.0, &model).unwrap();
    assert!(remove(&dir.0, &model).is_err()); assert!(dir.0.join(model.file).exists());
    std::fs::write(dir.0.join("selected-model"), catalog::DEFAULT_ID).unwrap();
    remove(&dir.0, &model).unwrap(); assert!(!dir.0.join(model.file).exists());
    remove(&dir.0, &model).unwrap();
}
#[test]
fn model_operations_exclude_active_workers_and_each_other_until_completion() {
    let gate = AtomicBool::new(false);
    assert!(begin_mutation(true, &gate).is_err()); assert!(!gate.load(Ordering::Acquire));
    let downloading = begin_mutation(false, &gate).unwrap();
    assert!(gate.load(Ordering::Acquire)); assert!(begin_mutation(false, &gate).is_err());
    drop(downloading); assert!(!gate.load(Ordering::Acquire));
    assert!(begin_mutation(false, &gate).is_ok());
}
