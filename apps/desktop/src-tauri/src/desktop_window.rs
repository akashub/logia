use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use std::sync::atomic::{AtomicU64, Ordering};

// Each reveal/dismiss invalidates edit permits before main-thread work queues.
static FOCUS_EPOCH: AtomicU64 = AtomicU64::new(0);

pub fn setup_overlay(app: &tauri::App) -> tauri::Result<()> {
    setup_with_url(app, WebviewUrl::App("index.html?overlay".into()))
}

pub fn setup_with_url(app: &tauri::App, url: WebviewUrl) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, "overlay", url)
        .title("Logia overlay")
        .inner_size(38., 38.)
        .visible(false)
        // Both webviews own the shortcut/acknowledgment bridge while hidden.
        // Suspending either one can defer Start/Stop until Settings is opened.
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
        .focused(false)
        .focusable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .accept_first_mouse(true)
        .build()?;
    #[cfg(target_os = "macos")]
    window.with_webview(|view| {
        // SAFETY: Tauri supplies its live WKWebView on the AppKit main thread.
        // Swift reparents only that view into a real NSPanel; no class mutation.
        unsafe { logia_overlay_attach(view.inner()) };
    })?;
    #[cfg(not(target_os = "macos"))]
    let _ = window;
    Ok(())
}

#[tauri::command]
pub async fn show_overlay(app: AppHandle) -> Result<(), String> {
    FOCUS_EPOCH.fetch_add(1, Ordering::SeqCst);
    native(&app, |app| {
        #[cfg(target_os = "macos")]
        {
            let _ = app;
            // SAFETY: native() dispatches all bridge entry points to main.
            if unsafe { logia_overlay_show() } { Ok(()) } else { Err("The recording panel is unavailable.".into()) }
        }
        #[cfg(not(target_os = "macos"))]
        app.get_webview_window("overlay").ok_or("The recording panel is unavailable.")?
            .show().map_err(|e| e.to_string())
    }).await?;
    for _ in 0..40 {
        if native(&app, |app| {
            #[cfg(target_os = "macos")]
            { let _ = app; Ok(unsafe { logia_overlay_visible() }) }
            #[cfg(not(target_os = "macos"))]
            { app.get_webview_window("overlay").ok_or("The recording panel is unavailable.")?
                .is_visible().map_err(|e| e.to_string()) }
        }).await? { return Ok(()); }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    Err("Logia could not appear in this Space. Open its window before recording.".into())
}

#[tauri::command]
pub async fn resize_overlay(app: AppHandle, width: f64, height: f64, position: String) -> Result<(), String> {
    if !width.is_finite() || !height.is_finite() || !(38. ..=900.).contains(&width)
        || !(38. ..=700.).contains(&height) || !matches!(position.as_str(), "top" | "bottom") {
        return Err("Invalid recording panel geometry.".into());
    }
    native(&app, move |app| {
        let window = app.get_webview_window("overlay").ok_or("The recording panel is unavailable.")?;
        // Keep Tauri's hidden backing window bounds consistent with its webview.
        window.set_size(tauri::LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        unsafe { logia_overlay_resize(width, height, position == "top") };
        #[cfg(not(target_os = "macos"))]
        if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
            let area = monitor.work_area();
            let size = window.outer_size().map_err(|e| e.to_string())?;
            let x = area.position.x + (area.size.width.saturating_sub(size.width) / 2) as i32;
            let y = area.position.y + if position == "top" { 24 } else { area.size.height.saturating_sub(size.height + 24) as i32 };
            window.set_position(tauri::PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }).await
}

#[tauri::command]
pub async fn hide_overlay(app: AppHandle) -> Result<(), String> {
    FOCUS_EPOCH.fetch_add(1, Ordering::SeqCst);
    native(&app, |app| {
        app.state::<crate::session::Sessions>().when_idle(|| {
            #[cfg(target_os = "macos")]
            unsafe { logia_overlay_hide() };
            #[cfg(not(target_os = "macos"))]
            app.get_webview_window("overlay").ok_or("The recording panel is unavailable.")?
                .hide().map_err(|e| e.to_string())?;
            Ok(())
        })
    }).await
}

#[tauri::command]
pub async fn overlay_editing_token(app: AppHandle) -> Result<u64, String> {
    let epoch = FOCUS_EPOCH.load(Ordering::SeqCst);
    native(&app, move |app| {
        app.state::<crate::session::Sessions>().when_idle(|| {
            check_edit_permit(&app, Some(epoch))?;
            Ok(epoch)
        })
    }).await
}

fn check_edit_permit(app: &AppHandle, epoch: Option<u64>) -> Result<(), String> {
    if epoch != Some(FOCUS_EPOCH.load(Ordering::SeqCst)) {
        return Err("That recovery session is no longer visible.".into());
    }
    #[cfg(target_os = "macos")]
    let visible = { let _ = app; unsafe { logia_overlay_visible() } };
    #[cfg(not(target_os = "macos"))]
    let visible = app.get_webview_window("overlay").is_some_and(|window| window.is_visible().unwrap_or(false));
    if !visible { return Err("That recovery session is no longer visible.".into()); }
    Ok(())
}

#[tauri::command]
pub async fn set_overlay_editing(app: AppHandle, editing: bool, epoch: Option<u64>) -> Result<(), String> {
    native(&app, move |app| {
        let change = || {
            if editing { check_edit_permit(&app, epoch)?; }
            #[cfg(target_os = "macos")]
            // SAFETY: native() dispatches to main; the bridge owns the NSPanel.
            if !unsafe { logia_overlay_editing(editing) } {
                return Err("The recording panel is unavailable.".into());
            }
            #[cfg(not(target_os = "macos"))]
            {
                let window = app.get_webview_window("overlay").ok_or("The recording panel is unavailable.")?;
                window.set_focusable(editing).map_err(|e| e.to_string())?;
                if editing { window.set_focus().map_err(|e| e.to_string())?; }
            }
            Ok(())
        };
        if editing { app.state::<crate::session::Sessions>().when_idle(change) } else { change() }
    }).await
}

async fn native<T: Send + 'static>(app: &AppHandle, action: impl FnOnce(AppHandle) -> Result<T, String> + Send + 'static) -> Result<T, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || { let _ = tx.send(action(handle)); }).map_err(|e| e.to_string())?;
    rx.await.map_err(|_| "The recording panel closed.".to_string())?
}

#[cfg(target_os = "macos")]
extern "C" {
    fn logia_overlay_attach(webview: *mut std::ffi::c_void);
    fn logia_overlay_show() -> bool;
    fn logia_overlay_visible() -> bool;
    fn logia_overlay_resize(width: f64, height: f64, top: bool);
    fn logia_overlay_hide();
    fn logia_overlay_editing(editing: bool) -> bool;
}
