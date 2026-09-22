#!/bin/bash
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
check_dir="$(mktemp -d "${TMPDIR:-/tmp}/logia-overlay-check.XXXXXX")"
trap 'rm -rf "$check_dir"' EXIT
swift build --package-path "$repo_dir/spikes/target-identity"
fixture_dir="$(swift build --package-path "$repo_dir/spikes/target-identity" --show-bin-path)"
swiftc "$repo_dir/apps/desktop/src-tauri/examples/support/OverlayPointerCheck.swift" -o "$check_dir/overlay-pointer-check"
# Native synthetic pointer/keyboard checks only. No microphone modules are
# linked into this example. Its own deadline closes the check on failure.
cargo run --manifest-path "$repo_dir/apps/desktop/src-tauri/Cargo.toml" --locked \
  --example window_smoke -- "$fixture_dir/target-fixture" "$check_dir/overlay-pointer-check"
