use crate::catalog::{self, Model};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) fn selected(folder: &Path) -> &'static Model {
    let mut id = String::new();
    if let Ok(file) = std::fs::File::open(folder.join("selected-model")) {
        let _ = file.take(128).read_to_string(&mut id);
    }
    catalog::find(id.trim()).unwrap_or_else(catalog::default_model)
}
pub(super) fn select(folder: &Path, model: &Model) -> Result<(), String> {
    verify_model(&folder.join(model.file), model)?;
    let partial = folder.join("selected-model.partial");
    std::fs::write(&partial, model.id).map_err(|_| "Could not save your model choice")?;
    std::fs::rename(partial, folder.join("selected-model")).map_err(|_| "Could not save your model choice".into())
}
pub(super) fn remove(folder: &Path, model: &Model) -> Result<(), String> {
    if selected(folder).id == model.id { return Err("Choose another installed model before removing this one.".into()); }
    match std::fs::remove_file(folder.join(model.file)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Could not remove that model.".into()),
    }
}
pub(super) fn verify_model(path: &Path, model: &Model) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|_| "Download this model first")?;
    if file.metadata().map_err(|_| "Could not inspect model")?.len() != model.bytes {
        return Err("Model file is incomplete. Download it again.".into());
    }
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 65536];
    loop {
        let count = file.read(&mut chunk).map_err(|_| "Could not read model")?;
        if count == 0 { break; }
        hash.update(&chunk[..count]);
    }
    if format!("{:x}", hash.finalize()) != model.sha256 {
        return Err("Model verification failed. Download it again.".into());
    }
    Ok(())
}
pub(super) struct Mutation<'a>(&'a AtomicBool);
impl Drop for Mutation<'_> { fn drop(&mut self) { self.0.store(false, Ordering::Release); } }
pub(super) fn begin_mutation(active: bool, gate: &AtomicBool) -> Result<Mutation<'_>, String> {
    if active { return Err("Stop recognition before changing model files.".into()); }
    gate.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "A model download or change is already running")?;
    Ok(Mutation(gate))
}

#[cfg(test)]
#[path = "model_store_tests.rs"]
mod tests;
