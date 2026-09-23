use crate::input_selection::{self, InputDevice};
use cpal::traits::{DeviceTrait, HostTrait};
use std::path::Path;

pub const WORKER_INPUT: &str = "LOGIA_CAPTURE_INPUT_ID";

pub fn enumerate() -> Result<(Vec<InputDevice>, Option<String>), String> {
    let host = cpal::default_host();
    let default = host
        .default_input_device()
        .and_then(|device| device.id().ok())
        .map(|id| id.to_string());
    let mut devices = Vec::new();
    for device in host
        .input_devices()
        .map_err(|_| "Could not list microphones. Try refreshing the list.")?
    {
        let Ok(id) = device.id() else { continue };
        let name = device
            .description()
            .map(|description| description.name().to_owned())
            .unwrap_or_else(|_| "Microphone".into());
        devices.push(InputDevice {
            id: id.to_string(),
            name,
        });
    }
    devices.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok((devices, default))
}

/// Called inside the same session lock as choice persistence and worker spawn.
pub fn snapshot(folder: &Path) -> Result<String, String> {
    let selected = input_selection::read(folder)?;
    let (devices, default) = enumerate()?;
    input_selection::resolve(selected.as_ref(), &devices, default.as_deref())
        .map(|device| device.id.clone())
}

/// Exact ID lookup at the last possible moment before opening capture. Both
/// explicit choices and System default snapshots stay fixed for this session.
pub fn capture_device(id: Option<&str>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    let id = match id {
        Some(id) => id
            .parse::<cpal::DeviceId>()
            .map_err(|_| "Invalid microphone choice. Choose an input in Settings.")?,
        None => host
            .default_input_device()
            .ok_or("No microphone found. Connect one and try again.")?
            .id()
            .map_err(|_| "Could not identify the default microphone")?,
    };
    let mut devices = host
        .input_devices()
        .map_err(|_| "Could not list microphones")?
        .filter(|device| device.id().is_ok_and(|candidate| candidate == id));
    let device = devices
        .next()
        .ok_or("Your microphone is unavailable. Reconnect it or choose another in Settings.")?;
    if devices.next().is_some() {
        return Err("Microphone identity is ambiguous. Choose another input in Settings.".into());
    }
    Ok(device)
}

pub fn worker_input() -> Result<Option<String>, String> {
    match std::env::var(WORKER_INPUT) {
        Ok(id) => Ok(Some(id)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => Err("Invalid microphone choice. Choose an input in Settings.".into()),
    }
}
