#!/bin/bash
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
check_dir="$(mktemp -d "${TMPDIR:-/tmp}/logia-browser-check.XXXXXX")"
trap 'rm -rf "$check_dir"' EXIT
xcrun swiftc -swift-version 5 \
  "$repo_dir"/spikes/target-identity/Sources/TargetProbeCore/*.swift \
  "$repo_dir/apps/desktop/src-tauri/native/TargetBridge.swift" \
  "$repo_dir/apps/desktop/src-tauri/native/TargetPreparation.swift" \
  "$repo_dir/apps/desktop/src-tauri/examples/support/BrowserDeliveryCheck.swift" \
  -o "$check_dir/browser-delivery-check"
node "$repo_dir/apps/desktop/tests/native-browser-delivery.cjs" "$check_dir/browser-delivery-check"
