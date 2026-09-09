//! Native focus checks; this example contains no microphone commands.
#[cfg(target_os = "macos")]
#[path = "support/window_smoke_macos.rs"]
mod native;

#[cfg(target_os = "macos")]
fn main() {
    native::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This focus check requires macOS and the synthetic AppKit target-fixture.");
    std::process::exit(64);
}
