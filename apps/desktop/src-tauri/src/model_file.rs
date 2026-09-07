use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};
use tokio::io::AsyncWriteExt;

pub const FILE: &str = "moonshine-streaming-small-Q8_0.gguf";
pub const HASH: &str = "d03670f69629b649085d0f44a63d97668b4119117cc9611a4e4ad94341713dfc";
pub const SIZE: u64 = 198_506_848;
const URL: &str = "https://huggingface.co/handy-computer/moonshine-streaming-small-gguf/resolve/41444173ed8210852a883e046fadcfba3e7bfbae/moonshine-streaming-small-Q8_0.gguf";

pub fn path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("models").join(FILE))
        .map_err(|_| "Could not locate Logia's model folder".into())
}

pub fn verify(path: &Path) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|_| "Download the English model first")?;
    if file
        .metadata()
        .map_err(|_| "Could not inspect model")?
        .len()
        != SIZE
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
    if format!("{:x}", hash.finalize()) != HASH {
        return Err("Model verification failed. Download it again.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn model_ready(app: tauri::AppHandle) -> Result<bool, String> {
    let path = path(&app)?;
    tauri::async_runtime::spawn_blocking(move || verify(&path).is_ok())
        .await
        .map_err(|_| "Model check stopped".into())
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
) -> Result<(), String> {
    if state
        .0
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("A download is already running".into());
    }
    let _reset = Reset(&state.0);
    let destination = path(&app)?;
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
        .get(URL)
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
        if total > SIZE {
            return Err("Downloaded model has an unexpected size".into());
        }
        hash.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|_| "Could not save model. Check free disk space.")?;
        let percent = total * 100 / SIZE;
        if percent != last_percent {
            let _ = app.emit("model-progress", percent);
            last_percent = percent;
        }
    }
    file.flush()
        .await
        .map_err(|_| "Could not finish saving model")?;
    drop(file);
    if total != SIZE || format!("{:x}", hash.finalize()) != HASH {
        return Err("Model verification failed. Please retry the download.".into());
    }
    tokio::fs::rename(partial, destination)
        .await
        .map_err(|_| "Could not install verified model".into())
}
