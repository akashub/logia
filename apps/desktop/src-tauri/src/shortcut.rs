#[path = "shortcut_registration.rs"]
mod registration;

use crate::shortcut_gesture::{Mode, Phase, GESTURES};
use registration::{Binding, Preset, Registrar, Selection};
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

static SELECTION: LazyLock<Mutex<Selection>> =
    LazyLock::new(|| Mutex::new(Selection::new(GESTURES.clone())));

struct NativeRegistrar(AppHandle);
impl Registrar for NativeRegistrar {
    fn register(&mut self, preset: Preset, binding: Binding) -> Result<(), ()> {
        // Failed rollback can leave this owned registration in place.
        if self.0.global_shortcut().is_registered(preset.shortcut()) {
            return Ok(());
        }
        self.0
            .global_shortcut()
            .on_shortcut(preset.shortcut(), move |app, _, event| {
                if let Some(event) = binding.accept(event.state == ShortcutState::Pressed) {
                    if event.phase == Phase::Released && event.mode == Mode::Hold {
                        let id = event.id.parse().expect("native gesture id");
                        let app = app.clone();
                        // The plugin holds its map lock in this callback. Neither
                        // Sessions nor SELECTION may be acquired here: registration
                        // owns Selection while waiting for the main thread/plugin.
                        tauri::async_runtime::spawn_blocking(move || {
                            let _ = app.state::<crate::session::Sessions>().finish_hold(id);
                        });
                    }
                    let _ = app.emit_to("main", "dictation-shortcut", event);
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
pub fn register_shortcut(
    app: AppHandle,
    shortcut: Option<String>,
    mode: Option<String>,
) -> Result<&'static str, String> {
    let preset = Preset::parse(shortcut.as_deref())?;
    let mode = Mode::parse(mode.as_deref())?;
    // Never wait for SELECTION on main: another registration may await main.
    let mut selection = SELECTION
        .try_lock()
        .map_err(|_| "A shortcut change is already in progress. Retry shortly.")?;
    let _reservation = app.state::<crate::session::Sessions>().reserve_shortcut()?;
    selection.select(preset, mode, &mut NativeRegistrar(app.clone()))?;
    Ok(preset.label(cfg!(target_os = "macos")))
}
