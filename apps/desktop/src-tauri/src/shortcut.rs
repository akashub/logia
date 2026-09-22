#[path = "shortcut_registration.rs"]
mod registration;

use registration::{Binding, Preset, Registrar, Selection};
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

static SELECTION: LazyLock<Mutex<Selection>> = LazyLock::new(|| Mutex::new(Selection::default()));

struct NativeRegistrar(AppHandle);
impl Registrar for NativeRegistrar {
    fn register(&mut self, preset: Preset, binding: Binding) -> Result<(), ()> {
        // An unsuccessful rollback may have left this owned registration in
        // place. It already has the same atomic gate and must not be duplicated.
        if self.0.global_shortcut().is_registered(preset.shortcut()) {
            return Ok(());
        }
        self.0
            .global_shortcut()
            .on_shortcut(preset.shortcut(), move |app, _, event| {
                if binding.accept(event.state == ShortcutState::Pressed) {
                    let _ = app.emit_to("main", "dictation-shortcut", ());
                }
            })
            .map_err(|_| ())
    }
    fn unregister(&mut self, preset: Preset) -> Result<(), ()> {
        self.0
            .global_shortcut()
            .unregister(preset.shortcut())
            .map_err(|_| ())
    }
}

#[tauri::command]
pub fn register_shortcut(app: AppHandle, shortcut: Option<String>) -> Result<&'static str, String> {
    // The user chose Control+Alt+Space after CommandOrControl+Shift+Space
    // conflicted with Spotlight. Missing arguments retain that chosen default.
    let preset = Preset::parse(shortcut.as_deref())?;
    // Never wait for this lock on the main thread: the plugin may be waiting
    // for a main-thread registration operation from another command.
    SELECTION
        .try_lock()
        .map_err(|_| "A shortcut change is already in progress. Retry shortly.")?
        .select(preset, &mut NativeRegistrar(app))?;
    Ok(preset.label(cfg!(target_os = "macos")))
}
