#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio_source;
mod capture;
mod messages;
mod model_file;
mod session;
mod worker;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.get(1).is_some_and(|arg| arg == "--recognizer") {
        let Some(model) = arguments.get(2) else {
            std::process::exit(64);
        };
        let audio = arguments.get(3).map(std::path::Path::new);
        if let Err(message) = worker::run(std::path::Path::new(model), audio) {
            let _ = messages::emit(&messages::WorkerEvent::Error { message });
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(model_file::DownloadState(
            std::sync::atomic::AtomicBool::new(false),
        ))
        .manage(session::Sessions::default())
        .invoke_handler(tauri::generate_handler![
            model_file::model_ready,
            model_file::download_model,
            session::start_recording,
            session::stop_recording,
            session::cancel_recording
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                use tauri::Manager;
                let _ = session::terminate(&window.state::<session::Sessions>());
            }
        })
        .run(tauri::generate_context!())
        .expect("Could not launch Logia");
}
