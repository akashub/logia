//! Real OS shortcut events in an owned blank window. No microphone modules.
#[cfg(target_os = "macos")]
#[path = "support/shortcut_smoke_macos.rs"]
mod native;
#[cfg(target_os = "macos")]
#[path = "../src/shortcut.rs"]
mod shortcut;
#[cfg(target_os = "macos")]
#[path = "../src/shortcut_edge.rs"]
mod shortcut_edge;
#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../src/shortcut_gesture.rs"]
mod shortcut_gesture;

// The production shortcut callback can signal this fixture, but no worker,
// capture, or recording command exists in this executable.
#[cfg(target_os = "macos")]
mod session {
    use std::sync::atomic::{AtomicU64, Ordering};
    #[derive(Default)]
    pub struct Sessions(pub AtomicU64);
    pub struct ShortcutConfiguration;
    impl Sessions {
        pub fn reserve_shortcut(&self) -> Result<ShortcutConfiguration, String> {
            Ok(ShortcutConfiguration)
        }
        pub fn finish_hold(&self, id: u64) -> Result<(), String> {
            self.0.store(id, Ordering::SeqCst);
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
fn main() {
    native::run();
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This shortcut smoke requires macOS.");
    std::process::exit(64);
}
