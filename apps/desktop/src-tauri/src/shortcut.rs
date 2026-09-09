use super::shortcut_edge::ShortcutEdge;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const SHORTCUT: &str = "CommandOrControl+Shift+Space";

#[tauri::command]
pub fn register_shortcut(app: AppHandle) -> Result<&'static str, String> {
    // Registered after the webview subscribes. A failed registration leaves
    // Record/Stop available and can be retried without restarting recognition.
    if !app.global_shortcut().is_registered(SHORTCUT) {
        let edge = ShortcutEdge::default();
        app.global_shortcut()
            .on_shortcut(SHORTCUT, move |app, _, event| {
                if edge.accept(event.state == ShortcutState::Pressed) {
                    let _ = app.emit_to("main", "dictation-shortcut", ());
                }
            })
            .map_err(|_| "Could not register the shortcut. It may be in use by another app. Free it, then retry.".to_string())?;
    }
    Ok(if cfg!(target_os = "macos") {
        "⌘ ⇧ Space"
    } else {
        "Ctrl Shift Space"
    })
}
