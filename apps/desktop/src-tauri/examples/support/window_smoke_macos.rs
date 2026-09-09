//! Native window/shortcut checks against Logia's synthetic AppKit fixture.
//! No recognition modules or microphone commands are compiled into this example.
#[path = "../../src/desktop_window.rs"]
mod desktop_window;
#[path = "../../src/shortcut.rs"]
mod shortcut;
#[path = "../../src/shortcut_edge.rs"]
mod shortcut_edge;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tauri::Manager;

pub fn run() {
    let fixture_path = std::env::args()
        .nth(1)
        .expect("pass the target-fixture executable path");
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows[0].url =
        tauri::WebviewUrl::External("about:blank".parse().unwrap());
    context.config_mut().app.windows[0].title = "Logia window checks — no microphone".into();
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(desktop_window::WindowMode::default())
        .setup(move |app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let result = check(&handle, &fixture_path);
                match &result {
                    Ok(()) => println!("PASS: native shortcut registration; background and minimized reveal preserve foreground; floating geometry restores."),
                    Err(error) => eprintln!("FAIL: {error}"),
                }
                handle.exit(if result.is_ok() { 0 } else { 1 });
            });
            Ok(())
        })
        .run(context)
        .expect("launch window checks");
}

fn check(app: &tauri::AppHandle, fixture_path: &str) -> Result<(), String> {
    let mut fixture = Command::new(fixture_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let mut input = fixture.stdin.take().unwrap();
        let mut output = BufReader::new(fixture.stdout.take().unwrap());
        let mut focus = || -> Result<(), String> {
            writeln!(input, "first").map_err(|e| e.to_string())?;
            let mut ready = String::new();
            output.read_line(&mut ready).map_err(|e| e.to_string())?;
            if ready.trim() != "ready" {
                return Err("Fixture did not become ready".into());
            }
            Ok(())
        };
        shortcut::register_shortcut(app.clone())?;
        shortcut::register_shortcut(app.clone())?;
        focus()?;
        let original = snapshot(app)?;
        if original.0 != fixture.id() as i32 {
            return Err("Synthetic fixture is not foreground".into());
        }
        tauri::async_runtime::block_on(desktop_window::set_floating(app.clone(), true, true))?;
        std::thread::sleep(std::time::Duration::from_millis(250));
        let floating = snapshot(app)?;
        if floating.0 != original.0 || floating.1 {
            return Err("Floating reveal stole focus".into());
        }
        if floating.2 != (560., 520.) {
            return Err(format!("Wrong floating inner size: {:?}", floating.2));
        }
        app.get_webview_window("main")
            .unwrap()
            .minimize()
            .map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(400));
        focus()?;
        tauri::async_runtime::block_on(desktop_window::set_floating(app.clone(), true, true))?;
        std::thread::sleep(std::time::Duration::from_millis(400));
        let revealed = snapshot(app)?;
        if revealed.0 != original.0 || revealed.1 || revealed.3 {
            return Err("Minimized reveal did not preserve foreground and visibility".into());
        }
        tauri::async_runtime::block_on(desktop_window::set_floating(app.clone(), false, false))?;
        let restored = snapshot(app)?;
        if restored.2 != original.2 {
            return Err("Expanded geometry was not restored".into());
        }
        Ok(())
    })();
    let _ = fixture.kill();
    let _ = fixture.wait();
    result
}

type Snapshot = (i32, bool, (f64, f64), bool);
fn snapshot(app: &tauri::AppHandle) -> Result<Snapshot, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let window = handle.get_webview_window("main").unwrap();
        let size = window
            .inner_size()
            .unwrap()
            .to_logical::<f64>(window.scale_factor().unwrap());
        let foreground = objc2_app_kit::NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|application| application.processIdentifier())
            .unwrap_or(0);
        let _ = tx.send((
            foreground,
            window.is_focused().unwrap(),
            (size.width, size.height),
            window.is_minimized().unwrap(),
        ));
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())
}
