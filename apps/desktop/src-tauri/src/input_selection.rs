use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};

const FILE: &str = "input-device.json";
const INVALID: &str = "Microphone choice could not be read. Choose a microphone again in Settings.";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct InputDevice {
    pub id: String,
    pub name: String,
}
#[derive(Deserialize, Serialize)]
struct Setting {
    version: u8,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    selected: Option<InputDevice>,
}

fn valid(device: &InputDevice) -> bool {
    !device.id.is_empty()
        && device.id.len() <= 2048
        && !device.id.chars().any(char::is_control)
        && !device.name.is_empty()
        && device.name.len() <= 512
}

pub fn read(folder: &Path) -> Result<Option<InputDevice>, String> {
    let file = match std::fs::File::open(folder.join(FILE)) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(INVALID.into()),
    };
    let mut bytes = Vec::new();
    file.take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| INVALID)?;
    if bytes.len() > 8192 {
        return Err(INVALID.into());
    }
    let setting: Setting = serde_json::from_slice(&bytes).map_err(|_| INVALID)?;
    if setting.version != 1
        || setting
            .selected
            .as_ref()
            .is_some_and(|device| !valid(device))
    {
        return Err(INVALID.into());
    }
    Ok(setting.selected)
}

/// Called while the session lock excludes another selection or worker launch.
pub fn save(folder: &Path, selected: Option<&InputDevice>) -> Result<(), String> {
    if selected.is_some_and(|device| !valid(device)) {
        return Err("Invalid microphone choice".into());
    }
    std::fs::create_dir_all(folder).map_err(|_| "Could not save your microphone choice")?;
    let bytes = serde_json::to_vec(&Setting {
        version: 1,
        selected: selected.cloned(),
    })
    .map_err(|_| "Invalid microphone choice")?;
    let partial = folder.join("input-device.json.partial");
    std::fs::write(&partial, bytes).map_err(|_| "Could not save your microphone choice")?;
    std::fs::rename(partial, folder.join(FILE))
        .map_err(|_| "Could not save your microphone choice".into())
}

/// A missing explicit ID never falls back to a same-name or default device.
pub fn resolve<'a>(
    selected: Option<&InputDevice>,
    devices: &'a [InputDevice],
    default_id: Option<&str>,
) -> Result<&'a InputDevice, String> {
    let id = selected
        .map(|device| device.id.as_str())
        .or(default_id)
        .ok_or("No default microphone is available. Choose a connected microphone in Settings.")?;
    let mut matches = devices.iter().filter(|device| device.id == id);
    let device = matches
        .next()
        .ok_or("Your microphone is unavailable. Reconnect it or choose another in Settings.")?;
    if matches.next().is_some() {
        return Err("Microphone identity is ambiguous. Choose another input in Settings.".into());
    }
    Ok(device)
}

#[cfg(test)]
#[path = "input_selection_tests.rs"]
mod tests;
