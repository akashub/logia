use std::sync::Mutex;
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Clone)]
struct Geometry {
    size: PhysicalSize<u32>,
    position: PhysicalPosition<i32>,
    maximized: bool,
    #[cfg(target_os = "macos")]
    collection: objc2_app_kit::NSWindowCollectionBehavior,
}

#[derive(Default)]
pub struct WindowMode(Mutex<Option<Geometry>>);

// Native window operations and AppKit access run together on the main thread.
// The command resolves before the UI can start microphone capture.
#[tauri::command]
pub async fn set_floating(app: AppHandle, floating: bool, reveal: bool) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let result = handle
            .get_webview_window("main")
            .ok_or_else(|| "The dictation window is unavailable.".to_string())
            .and_then(|window| change(&window, &handle.state::<WindowMode>(), floating, reveal));
        let _ = sender.send(result);
    })
    .map_err(|e| e.to_string())?;
    receiver
        .await
        .map_err(|_| "The dictation window closed.".to_string())??;
    #[cfg(target_os = "macos")]
    if reveal {
        wait_until_visible(&app).await?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
async fn wait_until_visible(app: &AppHandle) -> Result<(), String> {
    // A previously hidden window joins the active Space asynchronously. Let
    // AppKit/window-server events run; never sleep on the main thread.
    for _ in 0..20 {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let visible = handle
                .get_webview_window("main")
                .and_then(|window| {
                    let pointer = window.ns_window().ok()?;
                    // SAFETY: live Tauri window, on its AppKit main thread.
                    let native = unsafe { &*pointer.cast::<objc2_app_kit::NSWindow>() };
                    Some(native.isVisible() && native.isOnActiveSpace())
                })
                .unwrap_or(false);
            let _ = tx.send(visible);
        })
        .map_err(|e| e.to_string())?;
        if rx.await.unwrap_or(false) {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    Err("Logia could not appear in this Space. Open its window before recording.".into())
}

fn geometry(window: &WebviewWindow) -> tauri::Result<Geometry> {
    Ok(Geometry {
        size: window.inner_size()?,
        position: window.outer_position()?,
        maximized: window.is_maximized()?,
        #[cfg(target_os = "macos")]
        // SAFETY: all geometry calls run on the main thread with a live window.
        collection: unsafe { (&*window.ns_window()?.cast::<objc2_app_kit::NSWindow>()).collectionBehavior() },
    })
}

fn restore(window: &WebviewWindow, saved: &Geometry, floating: bool) -> tauri::Result<()> {
    window.set_always_on_top(floating)?;
    window.set_min_size(Some(if floating {
        LogicalSize::new(480., 460.)
    } else {
        LogicalSize::new(600., 600.)
    }))?;
    window.unmaximize()?;
    window.set_size(saved.size)?;
    window.set_position(saved.position)?;
    if saved.maximized {
        window.maximize()?;
    }
    #[cfg(target_os = "macos")]
    // SAFETY: restore is synchronous on the AppKit main thread.
    unsafe {
        (&*window.ns_window()?.cast::<objc2_app_kit::NSWindow>())
            .setCollectionBehavior(saved.collection);
    }
    Ok(())
}

fn change(
    window: &WebviewWindow,
    mode: &WindowMode,
    floating: bool,
    reveal: bool,
) -> Result<(), String> {
    let mut saved = mode
        .0
        .lock()
        .map_err(|_| "The window controls are unavailable.")?;
    let was_floating = saved.is_some();
    let before = geometry(window).map_err(|e| e.to_string())?;
    let apply = || -> tauri::Result<()> {
        if floating && !was_floating {
            window.unmaximize()?;
            window.set_min_size(Some(LogicalSize::new(480., 460.)))?;
            window.set_size(LogicalSize::new(560., 520.))?;
            // Keep the smaller window on the current display, clear of its dock.
            if let Some(monitor) = window.current_monitor()? {
                let area = monitor.work_area();
                let size = window.outer_size()?;
                let x = area.position.x + area.size.width.saturating_sub(size.width + 24) as i32;
                let y = area.position.y + area.size.height.saturating_sub(size.height + 24) as i32;
                window.set_position(PhysicalPosition::new(x, y))?;
            }
            window.set_always_on_top(true)?;
        } else if let Some(original) = saved.as_ref().filter(|_| !floating) {
            restore(window, original, false)?;
        }
        if reveal {
            reveal_without_focus(window)?;
        }
        Ok(())
    };
    if let Err(error) = apply() {
        let rollback = restore(window, &before, was_floating);
        return Err(if rollback.is_err() {
            "Could not restore the window. Expand or reopen Logia before recording.".into()
        } else {
            format!("Could not position the dictation window: {error}")
        });
    }
    if floating && !was_floating {
        *saved = Some(before);
    }
    if !floating {
        *saved = None;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn reveal_without_focus(window: &WebviewWindow) -> tauri::Result<()> {
    let pointer = window.ns_window()?;
    // SAFETY: Tauri owns this live NSWindow; caller runs on the AppKit main
    // thread. Unlike WebviewWindow::show, neither call makes it key or activates
    // NSApplication. Pointer lifetime is bounded by this synchronous callback.
    unsafe {
        let native = &*pointer.cast::<objc2_app_kit::NSWindow>();
        use objc2_app_kit::NSWindowCollectionBehavior as Behavior;
        native.setCollectionBehavior(
            (native.collectionBehavior() & !Behavior::FullScreenPrimary)
                | Behavior::CanJoinAllSpaces
                | Behavior::FullScreenAuxiliary,
        );
        if native.isMiniaturized() {
            native.deminiaturize(None);
        }
        native.orderFrontRegardless();
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn reveal_without_focus(window: &WebviewWindow) -> tauri::Result<()> {
    // Other desktop platforms have not passed focus-preservation acceptance.
    window.unminimize()?;
    window.show()
}
