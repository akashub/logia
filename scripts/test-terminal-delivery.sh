#!/bin/bash
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
check_dir="$(mktemp -d "${TMPDIR:-/tmp}/logia-terminal-check.XXXXXX")"
cleanup() {
  touch "$check_dir/stop"
  if [ -f "$check_dir/ready" ]; then
    for _ in {1..30}; do
      [ -f "$check_dir/exited" ] && break
      sleep 0.1
    done
    [ -f "$check_dir/exited" ] || return
  fi
  rm -rf "$check_dir"
}
trap cleanup EXIT
printf '#!/bin/bash\nexec /usr/bin/python3 %q %q\n' \
  "$repo_dir/apps/desktop/src-tauri/examples/support/terminal_fixture.py" "$check_dir" > "$check_dir/Logia-check.command"
chmod +x "$check_dir/Logia-check.command"
xcrun swiftc -swift-version 5 \
  "$repo_dir"/spikes/target-identity/Sources/TargetProbeCore/*.swift \
  "$repo_dir/apps/desktop/src-tauri/native/TargetBridge.swift" \
  "$repo_dir/apps/desktop/src-tauri/native/TargetPreparation.swift" \
  "$repo_dir/apps/desktop/src-tauri/examples/support/TerminalDeliveryCheck.swift" \
  -o "$check_dir/terminal-delivery-check"
"$check_dir/terminal-delivery-check" "$check_dir"
