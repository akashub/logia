#!/bin/bash
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
check_dir="$(mktemp -d "${TMPDIR:-/tmp}/logia-global-paste.XXXXXX")"
trap 'rm -rf "$check_dir"' EXIT
xcrun swiftc -swift-version 5 -parse-as-library \
  "$repo_dir/apps/desktop/src-tauri/examples/support/GlobalPasteFixture.swift" -o "$check_dir/fixture"
xcrun swiftc -swift-version 5 \
  "$repo_dir"/spikes/target-identity/Sources/TargetProbeCore/*.swift \
  "$repo_dir/apps/desktop/src-tauri/native/TargetBridge.swift" \
  "$repo_dir/apps/desktop/src-tauri/native/TargetPreparation.swift" \
  "$repo_dir/apps/desktop/src-tauri/examples/support/GlobalPasteCheck.swift" -o "$check_dir/check"
"$check_dir/check" "$check_dir/fixture"
