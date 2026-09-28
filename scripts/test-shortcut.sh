#!/bin/bash
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
check_dir="/private/tmp/logia-shortcut-smoke"
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This native shortcut smoke requires macOS." >&2
  exit 64
fi
if [[ "${1:-}" != "--build-only" && "${1:-}" != "" ]]; then
  echo "Usage: bash scripts/test-shortcut.sh [--build-only]" >&2
  exit 64
fi
if [[ "${1:-}" != "--build-only" ]] && pgrep -f '/Logia.app/Contents/MacOS/logia-desktop' >/dev/null; then
  echo "Quit installed Logia before running this shortcut smoke." >&2
  exit 1
fi
mkdir -p "$check_dir/module-cache"
swiftc -module-cache-path "$check_dir/module-cache" \
  "$repo_dir/apps/desktop/src-tauri/examples/support/ShortcutKeyCheck.swift" -o "$check_dir/shortcut-key-check"
cargo build --manifest-path "$repo_dir/apps/desktop/src-tauri/Cargo.toml" --locked --example shortcut_smoke
if [[ "${1:-}" == "--build-only" ]]; then
  echo "Built shortcut_smoke and $check_dir/shortcut-key-check; no keys posted."
  exit 0
fi
# The example enforces a 35-second deadline. The driver refuses installed
# Logia, checks owned foreground before keydown, and releases keys on failure.
cargo run --manifest-path "$repo_dir/apps/desktop/src-tauri/Cargo.toml" --locked \
  --example shortcut_smoke -- "$check_dir/shortcut-key-check"
