#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio_source;
mod background;
mod capture;
mod capture_audio;
mod desktop_window;
mod final_delivery;
mod inference_audio;
mod messages;
mod model_file;
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
        .manage(desktop_window::WindowMode::default())
        .manage(model_file::DownloadState(
            std::sync::atomic::AtomicBool::new(false),
        ))
        .manage(session::Sessions::default())
        .setup(|app| {
            background::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            model_file::model_ready,
            model_file::download_model,
            session::start_recording,
            session::warmup_recognizer,
            session::stop_recording,
            session_control::cancel_recording,
            desktop_window::set_floating,
            shortcut::register_shortcut,
            background::show_main_window,
            background::hide_main_window,
            target::capture_target,
            target::discard_target,
            target::accessibility_permission
        ])
        .on_window_event(|window, event| {
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
            if matches!(event, tauri::RunEvent::Exit) {
                use tauri::Manager;
                let _ = session::terminate(&app.state::<session::Sessions>());
            }
        });
}
