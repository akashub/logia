#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio_source;
mod background;
mod catalog;
mod capture;
mod capture_audio;
mod desktop_window;
mod dictionary;
mod final_delivery;
mod inference_audio;
mod messages;
mod model_file;
mod permissions;
mod session;
mod session_control;
mod session_output;
mod shortcut;
mod shortcut_edge;
mod target;
mod warmup;
mod worker;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments
        .get(1)
        .is_some_and(|arg| arg == "--recognizer" || arg == "--warmup")
    {
        let Some(model) = arguments.get(2) else {
            std::process::exit(64);
        };
        let audio = arguments.get(3).map(std::path::Path::new);
        let result = if arguments[1] == "--warmup" {
            warmup::run(std::path::Path::new(model))
        } else {
            worker::run(std::path::Path::new(model), audio)
        };
        if let Err(message) = result {
            let _ = messages::emit(&messages::WorkerEvent::Error { message });
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(model_file::DownloadState(
            std::sync::atomic::AtomicBool::new(false),
        ))
        .manage(session::Sessions::default())
        .manage(dictionary::Dictionary::default())
        .setup(|app| {
            desktop_window::setup_overlay(app)?;
            background::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            permissions::microphone_status,
            permissions::request_microphone,
            permissions::open_permission_settings,
            model_file::model_ready,
            model_file::download_model,
            model_file::list_models,
            model_file::remove_model,
            session::start_recording,
            dictionary::set_dictionary,
            dictionary::preview_dictionary,
            session::warmup_recognizer,
            session::stop_recording,
            session_control::cancel_recording,
            desktop_window::show_overlay,
            desktop_window::resize_overlay,
            desktop_window::hide_overlay,
            desktop_window::set_overlay_editing,
            desktop_window::overlay_editing_token,
            shortcut::register_shortcut,
            background::show_main_window,
            background::hide_main_window,
            target::capture_target,
            target::discard_target,
            target::accessibility_permission
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Focused(true)) {
                use tauri::Emitter;
                let _ = window.emit("permissions-refresh", ());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                use tauri::Emitter;
                api.prevent_close();
                let _ = window.emit("dictation-dismiss", ());
            }
            if matches!(event, tauri::WindowEvent::Destroyed) {
                use tauri::Manager;
                let _ = session::terminate(&window.state::<session::Sessions>());
            }
        })
        .build(tauri::generate_context!())
        .expect("Could not launch Logia")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::RunEvent::Reopen { .. }) {
                let _ = background::show_main_window(app.clone());
            }
            if matches!(event, tauri::RunEvent::Exit) {
                use tauri::Manager;
                let _ = session::terminate(&app.state::<session::Sessions>());
            }
        });
}
