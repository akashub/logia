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
fn unavailable_selection_cannot_activate_an_offline_placeholder() {
    let dir = Temp::new();
    std::fs::write(dir.0.join("selected-model"), "parakeet-v3").unwrap();
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
    let gate = Arc::new(AtomicBool::new(false));
    assert!(begin_mutation(true, &gate).is_err()); assert!(!gate.load(Ordering::Acquire));
    let downloading = begin_mutation(false, &gate).unwrap();
    assert!(gate.load(Ordering::Acquire)); assert!(begin_mutation(false, &gate).is_err());
    drop(downloading); assert!(!gate.load(Ordering::Acquire));
    assert!(begin_mutation(false, &gate).is_ok());
}

#[test]
fn failed_download_preserves_installed_bytes_and_selection() {
    use crate::model_staging::StagedModel;
    use std::io::Write;
    let dir = Temp::new(); let model = fixture(); let destination = dir.0.join(model.file);
    std::fs::write(&destination, b"abc").unwrap(); select(&dir.0, &model).unwrap();
    let gate = Arc::new(AtomicBool::new(false));
    let (staged, mut file) = StagedModel::create(&destination).unwrap();
    let partial = staged.path().to_owned();
    file.write_all(b"abd").unwrap(); drop(file);
    assert!(staged.install(&destination, &model, begin_mutation(false, &gate).unwrap()).is_err());
    assert_eq!(std::fs::read(&destination).unwrap(), b"abc");
    assert_eq!(selected(&dir.0).id, model.id);
    assert!(!partial.exists()); assert!(!gate.load(Ordering::Acquire));
}

#[test]
fn retries_use_distinct_staging_and_only_verified_bytes_are_installed() {
    use crate::model_staging::StagedModel;
    use std::io::Write;
    let dir = Temp::new(); let model = fixture(); let destination = dir.0.join(model.file);
    let (first, mut old_file) = StagedModel::create(&destination).unwrap();
    let (next, mut new_file) = StagedModel::create(&destination).unwrap();
    assert_ne!(first.path(), next.path());
    old_file.write_all(b"bad").unwrap(); new_file.write_all(b"abc").unwrap();
    drop(old_file); drop(new_file); drop(first);
    let gate = Arc::new(AtomicBool::new(false));
    next.install(&destination, &model, begin_mutation(false, &gate).unwrap()).unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), b"abc");
    assert!(!dir.0.join("selected-model").exists(), "downloading does not select");
}

#[test]
fn blocking_operation_owns_mutation_after_its_caller_drops_the_handle() {
    let gate = Arc::new(AtomicBool::new(false));
    let mutation = begin_mutation(false, &gate).unwrap();
    let (release, wait) = std::sync::mpsc::channel();
    let (finished, done) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || { wait.recv().unwrap(); drop(mutation); finished.send(()).unwrap(); });
    drop(handle);
    assert!(begin_mutation(false, &gate).is_err());
    release.send(()).unwrap(); done.recv().unwrap();
    assert!(begin_mutation(false, &gate).is_ok());
}
