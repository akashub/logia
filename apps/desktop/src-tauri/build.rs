fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        build_target_bridge();
    }
    tauri_build::build()
}

fn build_target_bridge() {
    use std::{path::PathBuf, process::Command};
    let core = PathBuf::from("../../../spikes/target-identity/Sources/TargetProbeCore");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        arch => panic!("Unsupported macOS architecture: {arch}"),
    };
    println!("cargo:rerun-if-changed={}", core.display());
    println!("cargo:rerun-if-changed=native/TargetBridge.swift");
    let sources = std::fs::read_dir(core)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "swift"));
    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-emit-library",
            "-static",
            "-O",
            "-swift-version",
            "5",
            "-module-name",
            "LogiaTarget",
            "-target",
        ])
        .arg(format!("{arch}-apple-macosx14.0"))
        .args(sources)
        .arg("native/TargetBridge.swift")
        .arg("-o")
        .arg(out.join("liblogia_target.a"))
        .status()
        .expect("Swift compiler is required for the macOS target bridge");
    assert!(status.success(), "Could not build native target bridge");
    let info = Command::new("xcrun")
        .args(["swiftc", "-print-target-info"])
        .output()
        .unwrap();
    assert!(info.status.success(), "Could not locate Swift runtime");
    let info: serde_json::Value = serde_json::from_slice(&info.stdout).unwrap();
    for path in info["paths"]["runtimeLibraryPaths"].as_array().unwrap() {
        println!("cargo:rustc-link-search=native={}", path.as_str().unwrap());
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=logia_target");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=ApplicationServices");
}
