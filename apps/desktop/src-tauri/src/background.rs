use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

pub fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "logia-open", "Open Logia", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "logia-quit", "Quit Logia", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    TrayIconBuilder::with_id("logia")
        .icon(icon())
        .icon_as_template(true)
        .tooltip("Logia — local dictation")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "logia-open" => {
                let _ = show_main_window(app.clone());
            }
            "logia-quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    Ok(())
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Logia's window is unavailable")?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_main_window(
    app: AppHandle,
    sessions: tauri::State<'_, crate::session::Sessions>,
) -> Result<(), String> {
    // The UI guard gives immediate feedback; the native guard closes the race
    // with an in-flight worker launch. Check and hide under the session lock.
    sessions.when_idle(|| {
        app.get_webview_window("main")
            .ok_or("Logia's window is unavailable")?
            .hide()
            .map_err(|e| e.to_string())
    })
}

fn icon() -> tauri::image::Image<'static> {
    // Retina source; macOS scales the template to the menu bar and tints it.
    let rgba: &[u8; 44 * 44 * 4] = include_bytes!("../icons/tray.rgba");
    tauri::image::Image::new(rgba, 44, 44)
}
