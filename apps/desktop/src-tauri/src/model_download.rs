use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use crate::model_file::{path_for, store};

#[derive(Default)]
pub struct DownloadState(pub Arc<AtomicBool>);
#[tauri::command]
pub async fn download_model(
    app: tauri::AppHandle,
    state: tauri::State<'_, DownloadState>,
    sessions: tauri::State<'_, crate::session::Sessions>,
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
    let mutation = sessions.when_idle(|| store::begin_mutation(false, &state.0))?;
    let destination = path_for(&app, model)?;
    let parent = destination.parent().ok_or("Model folder unavailable")?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|_| "Could not create model folder")?;
    let (staged, file) = crate::model_staging::StagedModel::create(&destination)?;
    let mut file = tokio::fs::File::from_std(file);
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
    tauri::async_runtime::spawn_blocking(move || staged.install(&destination, model, mutation))
        .await
        .map_err(|_| "Model installation stopped")?
}
