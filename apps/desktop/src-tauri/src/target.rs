use serde::Serialize;

#[cfg(target_os = "macos")]
extern "C" {
    fn logia_target_capture(status: *mut i32) -> u64;
    fn logia_target_cancel(token: u64);
    fn logia_target_send(token: u64, bytes: *const u8, count: usize) -> i32;
    fn logia_target_permission(prompt: bool) -> bool;
}

#[derive(Serialize)]
pub struct CapturedTarget {
    pub token: String,
    pub status: &'static str,
}

async fn on_main<T: Send + 'static>(
    app: tauri::AppHandle,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = send.send(work());
    })
    .map_err(|_| "Could not check the destination")?;
    receive
        .await
        .map_err(|_| "Destination check interrupted".into())
}

#[tauri::command]
pub async fn capture_target(app: tauri::AppHandle) -> Result<CapturedTarget, String> {
    on_main(app, || {
        #[cfg(target_os = "macos")]
        {
            let mut status = 1;
            let token = unsafe { logia_target_capture(&mut status) };
            CapturedTarget {
                token: token.to_string(),
                status: match status {
                    0 => "armed",
                    2 => "permission",
                    _ => "copy",
                },
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            CapturedTarget {
                token: "0".into(),
                status: "copy",
            }
        }
    })
    .await
}

#[tauri::command]
pub async fn discard_target(app: tauri::AppHandle, token: String) -> Result<(), String> {
    let token = token.parse::<u64>().map_err(|_| "Invalid destination")?;
    on_main(app, move || discard(token)).await
}

#[tauri::command]
pub async fn accessibility_permission(app: tauri::AppHandle, prompt: bool) -> Result<bool, String> {
    on_main(app, move || {
        #[cfg(target_os = "macos")]
        {
            unsafe { logia_target_permission(prompt) }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = prompt;
            false
        }
    })
    .await
}

// These functions are only called on the application's main thread.
pub fn discard(token: u64) {
    #[cfg(target_os = "macos")]
    unsafe {
        logia_target_cancel(token);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = token;
}

pub fn send(token: u64, text: &str) -> &'static str {
    if token == 0 {
        return "copy";
    }
    if text.len() > 65_536 {
        discard(token);
        return "copy";
    }
    #[cfg(target_os = "macos")]
    {
        match unsafe { logia_target_send(token, text.as_ptr(), text.len()) } {
            0 => "sent",
            2 => "uncertain",
            _ => "copy",
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        "copy"
    }
}
