use crate::{catalog::Model, model_file::store::{self, Mutation}};
use std::{fs::{File, OpenOptions}, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Each attempt owns an exclusive file. Canceled async writes can never touch
/// the installed model or a later retry's staging file.
pub(crate) struct StagedModel(PathBuf);
impl StagedModel {
    pub(crate) fn create(destination: &Path) -> Result<(Self, File), String> {
        for _ in 0..1024 {
            let path = destination.with_extension(format!("{}-{}.partial", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((Self(path), file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err("Could not save model. Check free disk space.".into()),
            }
        }
        Err("Could not create a model download file. Restart Logia and try again.".into())
    }

    #[cfg(test)]
    pub(crate) fn path(&self) -> &Path { &self.0 }

    /// Runs in one owned blocking task. Its guard lives until verification and
    /// the atomic replacement finish, even if the command caller goes away.
    pub(crate) fn install(self, destination: &Path, model: &Model, _mutation: Mutation) -> Result<(), String> {
        store::verify_model(&self.0, model)?;
        std::fs::rename(&self.0, destination).map_err(|_| "Could not install verified model".into())
    }
}
impl Drop for StagedModel {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}
