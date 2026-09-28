//! Native focus checks; this example contains no microphone commands.
#[cfg(target_os = "macos")]
#[path = "support/window_smoke_macos.rs"]
mod native;

// claude 2026-09-10: must sit at the crate root. `src/shortcut_registration.rs`
// imports `crate::shortcut_edge`, matching `main.rs` where it is a root module.
#[cfg(target_os = "macos")]
#[path = "../src/shortcut_edge.rs"]
mod shortcut_edge;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../src/shortcut_gesture.rs"]
mod shortcut_gesture;

// This fixture deliberately has no recording implementation or microphone API.
#[cfg(target_os = "macos")]
mod session {
    #[derive(Default)]
    pub struct Sessions(pub std::sync::atomic::AtomicBool);
    pub struct ShortcutConfiguration;
    impl Sessions {
        pub fn reserve_shortcut(&self) -> Result<ShortcutConfiguration, String> { self.when_idle(|| Ok(ShortcutConfiguration)) }
        pub fn finish_hold(&self, _id: u64) -> Result<(), String> { Ok(()) }
        pub fn when_idle<T>(&self, action: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
            if self.0.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("Fixture has an active session".into());
            }
            action()
        }
    }
}

#[cfg(target_os = "macos")]
fn main() {
    native::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This focus check requires macOS and the synthetic AppKit target-fixture.");
    std::process::exit(64);
}
