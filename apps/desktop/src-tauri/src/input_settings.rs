use crate::{
    input_devices,
    input_selection::{self, InputDevice},
    session::Sessions,
};
use std::path::PathBuf;
use tauri::Manager;

pub fn folder(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|_| "Could not locate Logia's settings".into())
}

#[derive(serde::Serialize)]
pub struct InputSettings {
    devices: Vec<InputDevice>,
    selected: Option<InputDevice>,
    default_id: Option<String>,
    // Keep the device picker usable so a corrupt preference can be repaired.
    error: Option<String>,
}

fn list(folder: &std::path::Path) -> Result<InputSettings, String> {
    let (devices, default_id) = input_devices::enumerate()?;
    let (selected, error) = match input_selection::read(folder) {
        Ok(selected) => (selected, None),
        Err(error) => (None, Some(error)),
    };
    Ok(InputSettings {
        devices,
        selected,
        default_id,
        error,
    })
}

#[tauri::command]
pub async fn list_audio_inputs(app: tauri::AppHandle) -> Result<InputSettings, String> {
    let folder = folder(&app)?;
    tauri::async_runtime::spawn_blocking(move || list(&folder))
        .await
        .map_err(|_| "Microphone list stopped")?
}

#[tauri::command]
pub async fn select_audio_input(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
    id: Option<String>,
) -> Result<Option<InputDevice>, String> {
    let folder = folder(&app)?;
    let sessions = sessions.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        sessions.when_idle(|| {
            let selected = if let Some(id) = id {
                let (devices, _) = input_devices::enumerate()?;
                let request = InputDevice {
                    id,
                    name: String::new(),
                };
                Some(input_selection::resolve(Some(&request), &devices, None)?.clone())
            } else {
                None
            };
            input_selection::save(&folder, selected.as_ref())?;
            Ok(selected)
        })
    })
    .await
    .map_err(|_| "Microphone selection stopped")?
}
