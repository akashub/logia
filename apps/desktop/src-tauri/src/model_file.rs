use std::path::{Path, PathBuf};
use tauri::Manager;
use crate::{catalog::{self, Model}, model_download::DownloadState, session::Sessions};
#[path = "model_store.rs"]
pub(crate) mod store;

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
    let folder = folder(app)?;
    Ok(folder.join(store::selected(&folder).file))
}

pub fn verified_model(path: &Path) -> Result<&'static Model, String> {
    let model = catalog::offerable().find(|model| Some(model.file) == path.file_name().and_then(|name| name.to_str()))
        .ok_or("Choose a supported live-caption model from Models")?;
    store::verify_model(path, model)?;
    Ok(model)
}

#[tauri::command]
pub async fn model_ready(app: tauri::AppHandle) -> Result<bool, String> {
    let path = path(&app)?;
    tauri::async_runtime::spawn_blocking(move || verified_model(&path).is_ok())
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
    active: bool,
    streams: bool,
}

#[tauri::command]
pub async fn list_models(app: tauri::AppHandle) -> Result<Vec<CatalogEntry>, String> {
    let folder = folder(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let active = store::selected(&folder).id;
        crate::catalog::MODELS
            .iter()
            .map(|model| CatalogEntry {
                model: *model,
                active: active == model.id,
                installed: !model.file.is_empty()
                    && store::verify_model(&folder.join(model.file), model).is_ok(),
                available: crate::catalog::offerable().any(|entry| entry.id == model.id),
                streams: model.streams(),
            })
            .collect()
    })
    .await
    .map_err(|_| "Could not read the model list".into())
}

#[tauri::command]
pub async fn select_model(app: tauri::AppHandle, sessions: tauri::State<'_, Sessions>, state: tauri::State<'_, DownloadState>, id: String) -> Result<(), String> {
    let model = catalog::offerable().find(|model| model.id == id).ok_or("That model is not available for live dictation")?;
    let folder = folder(&app)?;
    let mutation = sessions.when_idle(|| store::begin_mutation(false, &state.0))?;
    tauri::async_runtime::spawn_blocking(move || { let _mutation = mutation; store::select(&folder, model) }).await
        .map_err(|_| "Model selection stopped")?
}

#[tauri::command]
pub async fn remove_model(app: tauri::AppHandle, sessions: tauri::State<'_, Sessions>, state: tauri::State<'_, DownloadState>, id: String) -> Result<(), String> {
    let model = catalog::find(&id).ok_or("Unknown model")?;
    let folder = folder(&app)?;
    let mutation = sessions.when_idle(|| store::begin_mutation(false, &state.0))?;
    tauri::async_runtime::spawn_blocking(move || { let _mutation = mutation; store::remove(&folder, model) }).await
        .map_err(|_| "Model removal stopped")?
}
