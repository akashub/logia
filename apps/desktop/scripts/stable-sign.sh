#!/bin/bash
# A fixed identifier avoids identity-name churn. An ad-hoc designated requirement
# is still tied to the binary hash; it does NOT preserve TCC grants across updates.
# Use LOCAL_SIGNING_IDENTITY with an existing persistent signing certificate for
# repeatable local identity. This script never creates keys or changes TCC grants.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP="${1:-$SCRIPT_DIR/../src-tauri/target/debug/bundle/macos/Logia.app}"
IDENTITY="${LOCAL_SIGNING_IDENTITY:-}"
if [ -z "$IDENTITY" ]; then
  IDENTITY="$(security find-identity -p codesigning | awk '/"Logia Local Development"/ { seen[$2]=1 } END { for (hash in seen) print hash }')"
  if [[ "$IDENTITY" == *$'\n'* ]]; then
    echo 'Multiple Logia signing identities found. Set LOCAL_SIGNING_IDENTITY to the intended certificate hash.' >&2
    exit 1
  fi
  IDENTITY="${IDENTITY:--}"
fi
BUNDLE_ID="com.akashub.logia"
[ -d "$APP" ] || { echo "No bundle at $APP" >&2; exit 1; }
codesign --force --sign "$IDENTITY" --identifier "$BUNDLE_ID" \
  --options runtime --entitlements "$SCRIPT_DIR/../src-tauri/Entitlements.plist" "$APP"
codesign --verify --deep --strict "$APP"
echo "Signed $APP"
codesign -d -r- "$APP" 2>&1
if [ "$IDENTITY" = '-' ]; then
  echo 'Local ad-hoc build: macOS may require permission again after rebuilding.'
fi
