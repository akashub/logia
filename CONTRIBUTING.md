# Contributing

Logia is in early development. Keep changes focused and describe the behavior, motivation, verification, and remaining limitations.

For the macOS target probe, run `bash scripts/test-target-probe.sh`. Native observations require Accessibility permission and deliberate interaction with sample fields; unit-test success is not native compatibility evidence.

Run `bash scripts/check-target-identity.sh` for automatic native controls. It opens and focuses an owned synthetic AppKit window; leave it focused until it closes. Run Rust checks with `cargo test --manifest-path spikes/recognition/Cargo.toml --locked` and strict Clippy. Keep native checks separate from headless unit-test results.

Add meaningful tests for safety boundaries before implementation. Prioritize changed/unknown targets, malformed inputs, bounded queues, worker termination, stale output, and private-session persistence.

Do not commit personal recordings, transcripts, model files, credentials, generated build output, or session handovers. Use synthetic or appropriately licensed fixtures. Pin new dependencies, retain license notices, and describe their purpose.

Performance reports must identify hardware, OS, dependency/model revisions, workload, definitions, and failures. Distinguish process memory from total application footprint, batch throughput from live latency, and compilation from desktop validation.
