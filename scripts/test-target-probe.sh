#!/bin/bash
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
developer_dir="$(xcode-select -p)"
# Swift 6.0 ships Testing but lacks --disable-xctest (added in later tools).
# Let SwiftPM select its supported runner rather than rejecting the command.
test_flags=()
if [[ "$(swift test --help)" == *"--disable-xctest"* ]]; then
    test_flags+=(--disable-xctest)
fi

# Some Command Line Tools releases ship Testing outside SwiftPM's search path.
framework_dir="$developer_dir/Library/Developer/Frameworks"
interop_dir="$developer_dir/Library/Developer/usr/lib"
if [[ -d "$framework_dir/Testing.framework" ]]; then
    test_flags+=(-Xswiftc -F -Xswiftc "$framework_dir")
    test_flags+=(-Xlinker -F -Xlinker "$framework_dir")
    test_flags+=(-Xlinker -rpath -Xlinker "$framework_dir")
    test_flags+=(-Xlinker -rpath -Xlinker "$interop_dir")
fi

exec swift test --package-path "$repo_dir/spikes/target-identity" ${test_flags[@]+"${test_flags[@]}"} "$@"
