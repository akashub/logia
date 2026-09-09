//! Real selected-text writes, exclusively into an owned synthetic fixture.
//! No audio or microphone modules are compiled into this executable.
#[allow(dead_code)]
#[path = "../../src/target.rs"]
mod target;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    time::Duration,
};
static EXIT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(2);
const TEXT: &str = "Hello, café.\nAnother thought.";

pub fn run() {
    let fixture = std::env::args()
        .nth(1)
        .expect("pass the target-fixture executable path");
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows[0].url =
        tauri::WebviewUrl::External("about:blank".parse().unwrap());
    tauri::Builder::default().setup(move |app| {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        let handle = app.handle().clone();
        std::thread::spawn(move || {
            let result = check(&handle, &fixture);
            match &result {
                Ok(()) => println!("PASS: real selected-text replacement; changed field/selection, stale handle, secure/read-only, canceled, duplicate and invalid attempts refused; stale cancel preserves new target."),
                Err(error) => eprintln!("FAIL: {error}"),
            }
            let code = i32::from(result.is_err());
            EXIT.store(code, std::sync::atomic::Ordering::SeqCst);
            handle.exit(code);
        });
        Ok(())
    }).run(context).expect("launch native delivery checks");
    std::process::exit(EXIT.load(std::sync::atomic::Ordering::SeqCst));
}

fn on_main<T: Send + 'static>(
    app: &tauri::AppHandle,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(work());
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(10))
        .map_err(|_| "Native check timed out".into())
}
fn capture(app: &tauri::AppHandle) -> Result<target::CapturedTarget, String> {
    tauri::async_runtime::block_on(target::capture_target(app.clone()))
}
fn armed(app: &tauri::AppHandle) -> Result<u64, String> {
    let target = capture(app)?;
    if target.status != "armed" {
        return Err(format!("Synthetic target unavailable ({})", target.status));
    }
    target
        .token
        .parse()
        .map_err(|_| "Invalid native token".into())
}
fn send(app: &tauri::AppHandle, token: u64, text: &str, expected: &str) -> Result<(), String> {
    let text = text.to_owned();
    let status = on_main(app, move || target::send(token, &text))?;
    if status != expected {
        return Err(format!("Expected {expected}, got {status}"));
    }
    Ok(())
}

fn check(app: &tauri::AppHandle, path: &str) -> Result<(), String> {
    if !tauri::async_runtime::block_on(target::accessibility_permission(app.clone(), false))? {
        return Err("Accessibility access is required for this native check; no permission was requested automatically.".into());
    }
    let mut fixture = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let mut input = fixture.stdin.take().unwrap();
        let mut output = BufReader::new(fixture.stdout.take().unwrap());
        let mut command = |name: &str| -> Result<(), String> {
            writeln!(input, "{name}").map_err(|e| e.to_string())?;
            let mut result = String::new();
            output.read_line(&mut result).map_err(|e| e.to_string())?;
            let expected = if name.starts_with("assert-") {
                "pass"
            } else {
                "ready"
            };
            if result.trim() != expected {
                return Err(format!("Fixture check failed: {name}"));
            }
            Ok(())
        };
        command("delivery-select")?;
        let token = armed(app)?;
        send(app, token, TEXT, "sent")?;
        send(app, token, TEXT, "copy")?;
        command("assert-sent")?;
        for change in ["delivery-move", "second", "recreate", "readonly", "secure"] {
            command("delivery-select")?;
            let token = armed(app)?;
            command(change)?;
            send(app, token, TEXT, "copy")?;
            command(if change == "recreate" {
                "assert-recreated"
            } else {
                "assert-original"
            })?;
        }
        for unsupported in ["readonly", "secure"] {
            command("delivery-select")?;
            command(unsupported)?;
            if capture(app)?.token != "0" {
                return Err("Unsupported field was armed".into());
            }
        }
        command("delivery-select")?;
        let old = armed(app)?;
        let new = armed(app)?;
        on_main(app, move || target::discard(old))?;
        send(app, new, TEXT, "sent")?;
        command("assert-sent")?;
        command("delivery-select")?;
        let token = armed(app)?;
        on_main(app, move || target::discard(token))?;
        send(app, token, TEXT, "copy")?;
        command("assert-original")?;
        for text in [
            "".into(),
            " \n".into(),
            "bad\0text".into(),
            "x".repeat(65_537),
        ] {
            command("delivery-select")?;
            let token = armed(app)?;
            send(app, token, &text, "copy")?;
            send(app, token, TEXT, "copy")?;
            command("assert-original")?;
        }
        Ok(())
    })();
    let _ = fixture.kill();
    let _ = fixture.wait();
    result
}
