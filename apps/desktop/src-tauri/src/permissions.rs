#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MicrophoneStatus {
    #[cfg(any(target_os = "macos", test))]
    NotDetermined,
    #[cfg(any(target_os = "macos", test))]
    Restricted,
    #[cfg(any(target_os = "macos", test))]
    Denied,
    #[cfg(any(target_os = "macos", test))]
    Authorized,
    #[cfg(any(target_os = "macos", test))]
    Unknown,
    #[cfg(not(target_os = "macos"))]
    Unsupported,
}

#[cfg(target_os = "macos")]
extern "C" {
    fn logia_microphone_status() -> i32;
    fn logia_microphone_request(callback: extern "C" fn(i32, *mut std::ffi::c_void), context: *mut std::ffi::c_void);
    fn logia_permission_settings(microphone: bool) -> bool;
}
#[cfg(any(target_os = "macos", test))]
fn status(code: i32) -> MicrophoneStatus {
    match code { 0 => MicrophoneStatus::NotDetermined, 1 => MicrophoneStatus::Restricted,
        2 => MicrophoneStatus::Denied, 3 => MicrophoneStatus::Authorized, _ => MicrophoneStatus::Unknown }
}

#[tauri::command]
pub fn microphone_status() -> MicrophoneStatus {
    #[cfg(target_os = "macos")]
    { status(unsafe { logia_microphone_status() }) }
    #[cfg(not(target_os = "macos"))]
    { MicrophoneStatus::Unsupported }
}

pub fn require_microphone() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if microphone_status() != MicrophoneStatus::Authorized {
        return Err("Microphone access is not allowed. Open Logia's permission setup before recording.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn request_microphone() -> Result<MicrophoneStatus, String> {
    #[cfg(target_os = "macos")]
    {
        type Sender = tokio::sync::oneshot::Sender<MicrophoneStatus>;
        extern "C" fn complete(code: i32, context: *mut std::ffi::c_void) {
            // Swift calls exactly once, including already-decided permissions.
            let sender = unsafe { Box::from_raw(context.cast::<Sender>()) };
            let _ = sender.send(status(code));
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        let context = Box::into_raw(Box::new(tx)).cast();
        unsafe { logia_microphone_request(complete, context) };
        rx.await.map_err(|_| "The microphone permission request was interrupted.".into())
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(MicrophoneStatus::Unsupported) }
}

#[tauri::command]
pub async fn open_permission_settings(app: tauri::AppHandle, kind: String) -> Result<(), String> {
    if !matches!(kind.as_str(), "microphone" | "accessibility") { return Err("Unknown permission.".into()); }
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(unsafe { logia_permission_settings(kind == "microphone") });
        }).map_err(|_| "Could not open System Settings.")?;
        if rx.await.unwrap_or(false) { Ok(()) } else { Err("Open Privacy & Security in System Settings.".into()) }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = app; Err("Open microphone access in your operating system settings.".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_authorization_is_granted() {
        for code in [-1, 0, 1, 2, 4, 99] { assert_ne!(status(code), MicrophoneStatus::Authorized); }
        assert_eq!(status(3), MicrophoneStatus::Authorized);
        assert_eq!(status(2), MicrophoneStatus::Denied);
    }
}
