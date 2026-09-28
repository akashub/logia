#[path = "shortcut_smoke_driver.rs"]
mod driver;
use crate::{session::Sessions, shortcut, shortcut_gesture::GESTURES};
use driver::Driver;
use std::{
    sync::{
        atomic::{AtomicI32, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};
use tauri::{Listener, Manager};

const PRESET: &str = "Control+Alt+Space";
static EXIT_CODE: AtomicI32 = AtomicI32::new(2);
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    id: String,
    phase: String,
    mode: String,
}

pub fn run() {
    let driver = std::env::args()
        .nth(1)
        .expect("pass compiled ShortcutKeyCheck.swift path");
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "com.akashub.logia.shortcut-smoke".into();
    context.config_mut().build.dev_url = None;
    let window = &mut context.config_mut().app.windows[0];
    window.url = tauri::WebviewUrl::External("about:blank".parse().unwrap());
    window.title = "Logia shortcut checks — no microphone".into();
    window.visible = true;
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Sessions::default())
        .setup(move |app| {
            app.set_activation_policy(tauri::ActivationPolicy::Regular);
            app.get_webview_window("main").unwrap().set_focus()?;
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_secs(35));
                eprintln!("FAIL: shortcut smoke exceeded 35-second deadline");
                std::process::exit(124);
            });
            let app = app.handle().clone();
            std::thread::spawn(move || {
                let result = check(&app, &driver);
                let code = if result.is_ok() { 0 } else { 1 };
                match result {
                    Ok(()) => println!("PASS: native OS press/release, repeat suppression, toggle/hold payloads, held-key configuration rejection, release-invalidated permits, and native hold-finish callback; no microphone code."),
                    Err(error) => eprintln!("FAIL: {error}"),
                }
                EXIT_CODE.store(code, Ordering::SeqCst);
                app.exit(code);
            });
            Ok(())
        })
        .build(context).expect("launch shortcut checks")
        .run_return(|_, _| {});
    std::process::exit(EXIT_CODE.load(Ordering::SeqCst));
}
fn register(app: &tauri::AppHandle, mode: &str) -> Result<(), String> {
    shortcut::register_shortcut(app.clone(), Some(PRESET.into()), Some(mode.into())).map(|_| ())
}
fn next(events: &Receiver<String>, phase: &str, mode: &str) -> Result<Event, String> {
    let payload = events
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| format!("No native {mode}/{phase} event"))?;
    let event: Event =
        serde_json::from_str(&payload).map_err(|_| "Malformed native shortcut payload")?;
    if event.phase != phase || event.mode != mode || event.id.parse::<u64>().unwrap_or(0) == 0 {
        return Err(format!("Unexpected native shortcut payload: {payload}"));
    }
    Ok(event)
}
fn quiet(events: &Receiver<String>) -> Result<(), String> {
    match events.recv_timeout(Duration::from_millis(200)) {
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(()),
        _ => Err("Repeated or unpaired key edge emitted another gesture".into()),
    }
}
fn check(app: &tauri::AppHandle, path: &str) -> Result<(), String> {
    let (tx, events) = mpsc::channel();
    app.listen_any("dictation-shortcut", move |event| {
        let _ = tx.send(event.payload().to_owned());
    });
    // Driver refuses any installed Logia process and activates only this window.
    let mut driver = Driver::start(path, PRESET)?;
    register(app, "toggle")?;
    driver.command("press")?;
    let first = next(&events, "pressed", "toggle")?;
    driver.command("repeat")?;
    quiet(&events)?;
    driver.command("release")?;
    if next(&events, "released", "toggle")?.id != first.id {
        return Err("Toggle release changed gesture ID".into());
    }
    GESTURES.claim(&first.id)?.check()?;
    if GESTURES.claim(&first.id).is_ok() {
        return Err("Native gesture was reusable".into());
    }
    if app.state::<Sessions>().0.load(Ordering::SeqCst) != 0 {
        return Err("Toggle release requested native hold finish".into());
    }
    register(app, "hold")?;
    driver.command("press")?;
    let hold = next(&events, "pressed", "hold")?;
    if hold.id == first.id {
        return Err("New hold reused the toggle gesture".into());
    }
    let permit = GESTURES.claim(&hold.id)?;
    if register(app, "toggle").is_ok() {
        return Err("Mode changed while keys were held".into());
    }
    driver.command("repeat")?;
    quiet(&events)?;
    driver.command("release")?;
    if next(&events, "released", "hold")?.id != hold.id || permit.check().is_ok() {
        return Err("Hold release did not revoke its permit".into());
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    while app.state::<Sessions>().0.load(Ordering::SeqCst).to_string() != hold.id {
        if Instant::now() >= deadline {
            return Err("Native release did not dispatch finish".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    driver.command("press")?;
    let pending = next(&events, "pressed", "hold")?;
    driver.command("release")?;
    if next(&events, "released", "hold")?.id != pending.id || GESTURES.claim(&pending.id).is_ok() {
        return Err("Released pending hold remained claimable".into());
    }
    driver.command("release")?;
    quiet(&events)?;
    register(app, "toggle")?;
    Ok(())
}
