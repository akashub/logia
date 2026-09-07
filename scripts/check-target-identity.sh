#!/bin/bash
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
package_dir="$repo_dir/spikes/target-identity"
swift build --package-path "$package_dir"
bin_dir="$(swift build --package-path "$package_dir" --show-bin-path)"
exec python3 "$repo_dir/scripts/check-target-identity.py" --bin-dir "$bin_dir" "$@"
