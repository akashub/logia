#[cfg(target_os = "macos")]
#[path = "support/delivery_smoke_macos.rs"]
mod macos;
fn main() {
    #[cfg(target_os = "macos")]
    macos::run();
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Native delivery checks require macOS.");
        std::process::exit(1);
    }
}
