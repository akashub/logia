use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};
use tokio::io::AsyncWriteExt;

// claude 2026-09-14: every artifact detail now comes from the catalogue, so a
// second model is a table entry rather than a code change. Size and digest are
// still checked on every load: an artifact that does not match exactly is never
// activated, because a truncated or substituted model shows up as mysteriously
// poor transcription rather than as a download failure.
use crate::catalog::Model;
#[path = "model_store.rs"]
#[cfg(test)] // Selection helpers remain experimental until M05 wires the command path.
mod store;

pub fn folder(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("models"))
        .map_err(|_| "Could not locate Logia's model folder".into())
}

pub fn path_for(app: &tauri::AppHandle, model: &Model) -> Result<PathBuf, String> {
    folder(app).map(|dir| dir.join(model.file))
}

pub fn path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    path_for(app, crate::catalog::default_model())
}

pub fn verify_model(path: &Path, model: &Model) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|_| "Download this model first")?;
    if file
        .metadata()
        .map_err(|_| "Could not inspect model")?
        .len()
        != model.bytes
    {
        return Err("Model file is incomplete. Download it again.".into());
    }
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 65536];
    loop {
        let count = file.read(&mut chunk).map_err(|_| "Could not read model")?;
        if count == 0 {
            break;
        }
        hash.update(&chunk[..count]);
    }
    if format!("{:x}", hash.finalize()) != model.sha256 {
        return Err("Model verification failed. Download it again.".into());
    }
    Ok(())
}

pub fn verify(path: &Path) -> Result<(), String> {
    verify_model(path, crate::catalog::default_model())
}

#[tauri::command]
pub async fn model_ready(app: tauri::AppHandle) -> Result<bool, String> {
    let path = path(&app)?;
    tauri::async_runtime::spawn_blocking(move || verify(&path).is_ok())
        .await
        .map_err(|_| "Model check stopped".into())
}

#[derive(serde::Serialize)]
pub struct CatalogEntry {
    #[serde(flatten)]
    model: crate::catalog::Model,
    /// Verified present on disk, not merely downloaded.
    installed: bool,
    /// Pinned well enough to offer. Listed entries that are not are shown as
    /// coming soon rather than hidden, so the reader knows what is planned.
    available: bool,
    streams: bool,
}

#[tauri::command]
pub async fn list_models(app: tauri::AppHandle) -> Result<Vec<CatalogEntry>, String> {
    let folder = folder(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::catalog::MODELS
            .iter()
            .map(|model| CatalogEntry {
                model: *model,
                installed: !model.file.is_empty()
                    && verify_model(&folder.join(model.file), model).is_ok(),
                available: crate::catalog::offerable().any(|entry| entry.id == model.id),
                streams: model.streams(),
            })
            .collect()
    })
    .await
    .map_err(|_| "Could not read the model list".into())
}

#[tauri::command]
pub async fn remove_model(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let model = crate::catalog::find(&id).ok_or("Unknown model")?;
    if model.id == crate::catalog::DEFAULT_ID {
        return Err("The default model cannot be removed while it is the only one.".into());
    }
    let path = path_for(&app, model)?;
    match tokio::fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Could not remove that model.".into()),
    }
}

pub struct DownloadState(pub AtomicBool);
struct Reset<'a>(&'a AtomicBool);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub async fn download_model(
    app: tauri::AppHandle,
    state: tauri::State<'_, DownloadState>,
    id: Option<String>,
) -> Result<(), String> {
    // An unpinned catalogue entry is never downloadable: without an exact size
    // and digest a bad artifact would surface as poor transcription instead of
    // a failed download.
    let model = match id.as_deref() {
        Some(id) => crate::catalog::find(id).ok_or("Unknown model")?,
        None => crate::catalog::default_model(),
    };
    if !crate::catalog::offerable().any(|entry| entry.id == model.id) {
        return Err("That model is listed but not yet available to download.".into());
    }
    if state
        .0
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("A download is already running".into());
    }
    let _reset = Reset(&state.0);
    let destination = path_for(&app, model)?;
    let parent = destination.parent().ok_or("Model folder unavailable")?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|_| "Could not create model folder")?;
    let partial = destination.with_extension("partial");
    let mut file = tokio::fs::File::create(&partial)
        .await
        .map_err(|_| "Could not save model")?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|_| "Could not start download")?;
    let mut response = client
        .get(model.url)
        .send()
        .await
        .map_err(|_| "Download connection failed. Try again.")?
        .error_for_status()
        .map_err(|_| "Model server could not provide the file. Try again.")?;
    let mut total = 0u64;
    let mut hash = Sha256::new();
    let mut last_percent = 0;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Download interrupted. Try again.")?
    {
        total += chunk.len() as u64;
        if total > model.bytes {
            return Err("Downloaded model has an unexpected size".into());
        }
        hash.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|_| "Could not save model. Check free disk space.")?;
        let percent = total * 100 / model.bytes;
        if percent != last_percent {
            let _ = app.emit("model-progress", percent);
            last_percent = percent;
        }
    }
    file.flush()
        .await
        .map_err(|_| "Could not finish saving model")?;
    drop(file);
    if total != model.bytes || format!("{:x}", hash.finalize()) != model.sha256 {
        return Err("Model verification failed. Please retry the download.".into());
    }
    tokio::fs::rename(partial, destination)
        .await
        .map_err(|_| "Could not install verified model".into())
}
