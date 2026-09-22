//! Real pointer focus checks against Logia's owned synthetic AppKit fixture.
//! No recognition modules or microphone commands are compiled into this example.
#[path = "../../src/desktop_window.rs"]
mod desktop_window;
#[path = "../../src/shortcut.rs"]
mod shortcut;
mod overlay_smoke_assets;
// claude 2026-09-10: `shortcut_edge` is declared at the example's crate root
// (window_smoke.rs) instead of here. `shortcut_registration.rs` resolves it as
// `crate::shortcut_edge`, which only works from the root — declaring it inside
// this module made it `native::shortcut_edge` and broke the example build.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tauri::{Listener, Manager};
static EXIT_CODE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(2);

pub fn run() {
    let fixture_path = std::env::args().nth(1).expect("pass target-fixture path");
    let pointer_path = std::env::args().nth(2).expect("pass compiled OverlayPointerCheck.swift path");
    let mut context = tauri::generate_context!();
    context.config_mut().build.dev_url = None;
    context.config_mut().app.security.csp = None;
    context.set_assets(Box::new(overlay_smoke_assets::SmokeAssets));
    context.config_mut().app.windows[0].url = tauri::WebviewUrl::External("about:blank".parse().unwrap());
    context.config_mut().app.windows[0].title = "Logia window checks — no microphone".into();
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(crate::session::Sessions::default())
        .invoke_handler(tauri::generate_handler![
            desktop_window::show_overlay,
            desktop_window::resize_overlay,
            desktop_window::hide_overlay,
            desktop_window::set_overlay_editing,
            desktop_window::overlay_editing_token
        ])
        .setup(move |app| {
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(45));
                eprintln!("FAIL: native fixture exceeded 45-second deadline");
                std::process::exit(124);
            });
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            eprintln!("CHECK: create dedicated overlay test page");
            desktop_window::setup_overlay(app)?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let result = check(&handle, &fixture_path, &pointer_path);
                match &result {
                    Ok(()) => println!("PASS: separate native panel; actual Stop/Cancel/Copy clicks and transcript scroll preserve fixture app, focused field, selection and content; main geometry unchanged; hide/reveal and top/bottom resize."),
                    Err(error) => eprintln!("FAIL: {error}"),
                }
                let code = if result.is_ok() { 0 } else { 1 };
                EXIT_CODE.store(code, std::sync::atomic::Ordering::SeqCst);
                handle.exit(code);
            });
            Ok(())
        })
        .build(context)
        .expect("launch window checks")
        .run_return(|_, _| {});
    std::process::exit(EXIT_CODE.load(std::sync::atomic::Ordering::SeqCst));
}

fn check(app: &tauri::AppHandle, fixture_path: &str, pointer_path: &str) -> Result<(), String> {
    eprintln!("CHECK: launch synthetic field fixture");
    let mut fixture = Command::new(fixture_path).stdin(Stdio::piped()).stdout(Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;
    let result = (|| {
        let mut input = fixture.stdin.take().unwrap();
        let mut output = BufReader::new(fixture.stdout.take().unwrap());
        writeln!(input, "delivery-select").map_err(|e| e.to_string())?;
        let mut ready = String::new();
        output.read_line(&mut ready).map_err(|e| e.to_string())?;
        if ready.trim() != "ready" { return Err("Fixture not ready".into()); }
        // claude 2026-09-10: register_shortcut now takes the chosen preset;
        // None keeps the user's default. Called twice to assert idempotence.
        shortcut::register_shortcut(app.clone(), None)?;
        shortcut::register_shortcut(app.clone(), None)?;
        let original = snapshot(app)?;
        if original.0 != fixture.id() as i32 { return Err("Fixture not foreground".into()); }
        let (tx, rx) = std::sync::mpsc::channel();
        app.listen("overlay-smoke", move |event| { let _ = tx.send(event.payload().to_string()); });
        let window = app.get_webview_window("overlay").ok_or("Missing separate overlay")?;
        std::thread::sleep(std::time::Duration::from_millis(1500));
        window.eval(include_str!("overlay-smoke.js")).map_err(|e| e.to_string())?;
        expect_event(&rx, "ready")?;
        eprintln!("CHECK: dedicated page ready; test pointer delivery and focus");
        for position in ["bottom", "top"] {
            tauri::async_runtime::block_on(desktop_window::resize_overlay(app.clone(), 464., 120., position.into()))?;
            tauri::async_runtime::block_on(desktop_window::show_overlay(app.clone()))?;
            std::thread::sleep(std::time::Duration::from_millis(350));
            let revealed = snapshot(app)?;
            if revealed != original { return Err(format!("Reveal changed owned fixture focus/main geometry: before {original:?}, after {revealed:?}")); }
            for action in ["stop", "cancel", "copy", "scroll"] {
                let status = Command::new(pointer_path).arg(std::process::id().to_string())
                    .arg(fixture.id().to_string()).arg(action).status().map_err(|e| e.to_string())?;
                if !status.success() { return Err(format!("Pointer {action} focus check failed")); }
                expect_event(&rx, action)?;
                if snapshot(app)? != original { return Err(format!("Pointer {action} changed foreground/main")); }
            }
            tauri::async_runtime::block_on(desktop_window::hide_overlay(app.clone()))?;
        }
        for (width, height) in [(38., 38.), (200., 56.), (464., 192.)] {
            tauri::async_runtime::block_on(desktop_window::resize_overlay(app.clone(), width, height, "bottom".into()))?;
            tauri::async_runtime::block_on(desktop_window::show_overlay(app.clone()))?;
            if snapshot(app)? != original { return Err("Lifecycle resize changed main/focus".into()); }
            tauri::async_runtime::block_on(desktop_window::hide_overlay(app.clone()))?;
        }
        for (width, height, position) in [(f64::NAN, 120., "bottom"), (464., 800., "bottom"), (464., 120., "wrong")] {
            if tauri::async_runtime::block_on(desktop_window::resize_overlay(app.clone(), width, height, position.into())).is_ok() {
                return Err("Invalid panel geometry was accepted".into());
            }
        }
        app.state::<crate::session::Sessions>().0.store(true, std::sync::atomic::Ordering::SeqCst);
        if tauri::async_runtime::block_on(desktop_window::set_overlay_editing(app.clone(), true, None)).is_ok() {
            return Err("Editing activation accepted during active session".into());
        }
        app.state::<crate::session::Sessions>().0.store(false, std::sync::atomic::Ordering::SeqCst);
        tauri::async_runtime::block_on(desktop_window::resize_overlay(app.clone(), 464., 120., "bottom".into()))?;
        tauri::async_runtime::block_on(desktop_window::show_overlay(app.clone()))?;
        let stale = tauri::async_runtime::block_on(desktop_window::overlay_editing_token(app.clone()))?;
        tauri::async_runtime::block_on(desktop_window::hide_overlay(app.clone()))?;
        if tauri::async_runtime::block_on(desktop_window::set_overlay_editing(app.clone(), true, Some(stale))).is_ok() {
            return Err("Delayed editing reopened a dismissed overlay".into());
        }
        tauri::async_runtime::block_on(desktop_window::show_overlay(app.clone()))?;
        if tauri::async_runtime::block_on(desktop_window::set_overlay_editing(app.clone(), true, Some(stale))).is_ok() {
            return Err("Delayed editing focused a newer overlay".into());
        }
        if snapshot(app)? != original { return Err("Stale edit permit changed native focus".into()); }
        let epoch = tauri::async_runtime::block_on(desktop_window::overlay_editing_token(app.clone()))?;
        tauri::async_runtime::block_on(desktop_window::set_overlay_editing(app.clone(), true, Some(epoch)))?;
        let status = Command::new(pointer_path).arg(std::process::id().to_string())
            .arg(fixture.id().to_string()).arg("edit").status().map_err(|e| e.to_string())?;
        if !status.success() { return Err("Deliberate editing failed".into()); }
        expect_event(&rx, "edit")?;
        tauri::async_runtime::block_on(desktop_window::set_overlay_editing(app.clone(), false, None))?;
        writeln!(input, "assert-unsent").map_err(|e| e.to_string())?;
        let mut result = String::new();
        output.read_line(&mut result).map_err(|e| e.to_string())?;
        if result.trim() != "pass" { return Err("Fixture content changed".into()); }
        Ok(())
    })();
    let _ = fixture.kill();
    let _ = fixture.wait();
    result
}

fn expect_event(rx: &std::sync::mpsc::Receiver<String>, expected: &str) -> Result<(), String> {
    let wanted = serde_json::to_string(expected).unwrap();
    for _ in 0..8 {
        let event = rx.recv_timeout(std::time::Duration::from_secs(3)).map_err(|_| format!("No real webview {expected} event"))?;
        if event == wanted { return Ok(()); }
    }
    Err(format!("Missing {expected} event"))
}

type Snapshot = (i32, bool, (f64, f64));
fn snapshot(app: &tauri::AppHandle) -> Result<Snapshot, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let window = handle.get_webview_window("main").unwrap();
        let size = window.inner_size().unwrap().to_logical::<f64>(window.scale_factor().unwrap());
        let foreground = objc2_app_kit::NSWorkspace::sharedWorkspace().frontmostApplication()
            .map(|application| application.processIdentifier()).unwrap_or(0);
        let _ = tx.send((foreground, window.is_focused().unwrap(), (size.width, size.height)));
    }).map_err(|e| e.to_string())?;
    rx.recv_timeout(std::time::Duration::from_secs(5)).map_err(|e| e.to_string())
}
