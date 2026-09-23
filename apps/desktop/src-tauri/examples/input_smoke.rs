// Read-only device identity check. No stream construction or microphone capture.
#[allow(dead_code)]
#[path = "../src/input_devices.rs"]
mod input_devices;
#[allow(dead_code)]
#[path = "../src/input_selection.rs"]
mod input_selection;
use cpal::traits::DeviceTrait;

fn main() -> Result<(), String> {
    let (devices, default) = input_devices::enumerate()?;
    if devices.is_empty() {
        return Err("No inputs visible; native identity checks could not run".into());
    }
    for listed in &devices {
        let resolved = input_devices::capture_device(Some(&listed.id))?;
        let actual = resolved
            .id()
            .map_err(|_| "Could not read resolved input identity")?;
        assert_eq!(actual.to_string(), listed.id);
    }
    if let Some(default) = &default {
        input_selection::resolve(None, &devices, Some(default))?;
        input_devices::capture_device(Some(default))?;
    }
    assert!(input_devices::capture_device(Some("invalid-input-id")).is_err());
    println!(
        "PASS native input identity: {} devices, default present: {}; no stream opened",
        devices.len(),
        default.is_some()
    );
    Ok(())
}
