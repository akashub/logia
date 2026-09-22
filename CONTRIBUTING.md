# Contributing

Logia is in early development. Keep changes focused and describe the behavior, motivation, verification, and remaining limitations.

Use Node.js 24.15.0+, pnpm 10.33.2 and Rust 1.95.0 for the current checked baseline. Install native dependencies from [Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/), plus CMake and the platform audio development libraries (ALSA on Linux). macOS native code requires Swift 6 and the Apple developer tools. Install desktop dependencies with `pnpm install --frozen-lockfile` in `apps/desktop`.

From `apps/desktop`, run `pnpm build`, `pnpm test:unit` and `pnpm test:install`. For UI checks run `pnpm exec playwright install chromium`, start `pnpm dev`, and run `pnpm test:ui` in another terminal. These use synthetic native IPC and recognition events; they do not open a microphone or establish native focus behavior.

From the repository root, run `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --all-targets` and `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --all-targets -- -D warnings`. Build frontend assets first. `.github/workflows/ci.yml` runs frontend checks and three-OS Rust build/tests, plus Swift unit tests on macOS. Signing, installers, microphone and Accessibility interaction are intentionally separate from CI; compilation is not a desktop compatibility claim.

For native macOS overlay acceptance, `bash scripts/test-overlay.sh` uses owned synthetic windows to check Stop/Cancel/Copy, scrolling and deliberate idle editing. It needs Accessibility access for its test helper and briefly controls focus. `bash scripts/test-global-paste.sh` checks the current-cursor delivery bridge with synthetic fields. Run a native check when a related implementation changes; do not repeat unrelated application tests.

For the macOS target probe, run `bash scripts/test-target-probe.sh`. Native observations require Accessibility permission and deliberate interaction with sample fields; unit-test success is not native compatibility evidence.

Run `bash scripts/check-target-identity.sh` for automatic native controls. It opens and focuses an owned synthetic AppKit window; leave it focused until it closes. Run Rust checks with `cargo test --manifest-path spikes/recognition/Cargo.toml --locked` and strict Clippy. Keep native checks separate from headless unit-test results.

Add meaningful tests for safety boundaries before implementation. Prioritize changes during final dispatch checks, malformed inputs, bounded queues, worker termination, stale output, and private-session persistence. The product chooses the current cursor when delivering; the strict recording-start identity probe is a separate experiment.

Do not commit personal recordings, transcripts, model files, credentials, generated build output, or session handovers. Use synthetic or appropriately licensed fixtures. Pin new dependencies, retain license notices, and describe their purpose.

Performance reports must identify hardware, OS, dependency/model revisions, workload, definitions, and failures. Distinguish process memory from total application footprint, batch throughput from live latency, and compilation from desktop validation.
