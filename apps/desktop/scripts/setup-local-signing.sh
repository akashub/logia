#!/bin/bash
# Explicit, one-time developer setup. Builds never create certificates themselves.
# Local identity only: not an Apple Developer ID and not a trusted root CA.
set -euo pipefail
SIGNING_NAME='Logia Local Development'
if /usr/bin/security find-identity -p codesigning | /usr/bin/grep -Fq "\"$SIGNING_NAME\""; then
  echo 'Logia local signing identity already exists. No changes made.'
  exit 0
fi
if [ "${1:---check}" != '--install' ]; then
  echo 'No persistent Logia signing identity. Run pnpm signing:setup to create one in your login keychain.'
  exit 0
fi
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"
[ -f "$KEYCHAIN" ] || { echo 'Login keychain not found; no changes made.' >&2; exit 1; }
# Do not replace or duplicate a certificate whose private key is missing.
if /usr/bin/security find-certificate -c "$SIGNING_NAME" "$KEYCHAIN" >/dev/null 2>&1; then
  echo 'A Logia certificate exists without a usable signing key. Inspect it in Keychain Access first.' >&2
  exit 1
fi
umask 077
SIGNING_TMP="$(mktemp -d "${TMPDIR:-/tmp}/logia-signing.XXXXXX")"
trap 'rm -rf "$SIGNING_TMP"' EXIT
cat > "$SIGNING_TMP/certificate.cnf" <<'CONFIG'
[req]
distinguished_name = subject
x509_extensions = codesign
prompt = no
[subject]
CN = Logia Local Development
[codesign]
basicConstraints = critical,CA:FALSE
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
subjectKeyIdentifier = hash
CONFIG
/usr/bin/openssl req -new -x509 -newkey rsa:3072 -nodes -days 3650 \
  -config "$SIGNING_TMP/certificate.cnf" -keyout "$SIGNING_TMP/key.pem" \
  -out "$SIGNING_TMP/cert.pem" 2> "$SIGNING_TMP/openssl.log"
# macOS's PKCS#12 importer rejects an empty wrapping password on some systems.
# Generate a temporary password, never print it, and remove it with the key files.
/usr/bin/openssl rand -hex 32 > "$SIGNING_TMP/wrapping-password"
/usr/bin/openssl pkcs12 -export -inkey "$SIGNING_TMP/key.pem" -in "$SIGNING_TMP/cert.pem" \
  -name "$SIGNING_NAME" -out "$SIGNING_TMP/identity.p12" -passout "file:$SIGNING_TMP/wrapping-password"
/usr/bin/security import "$SIGNING_TMP/identity.p12" -k "$KEYCHAIN" -f pkcs12 \
  -P "$(cat "$SIGNING_TMP/wrapping-password")" -x -T /usr/bin/codesign
# Restrict key use to codesign; do not add trust settings, reset TCC, or grant access.
SIGNING_HASH="$(/usr/bin/openssl x509 -in "$SIGNING_TMP/cert.pem" -noout -fingerprint -sha1 | cut -d= -f2 | tr -d ':')"
# Native proof: finding a certificate alone does not prove its key can sign.
/usr/bin/clang -x c -o "$SIGNING_TMP/signing-check" - <<'C'
int main(void) { return 0; }
C
/usr/bin/codesign --force --sign "$SIGNING_HASH" --keychain "$KEYCHAIN" \
  --identifier com.akashub.logia.signing-check "$SIGNING_TMP/signing-check"
/usr/bin/codesign --verify --strict "$SIGNING_TMP/signing-check"
/usr/bin/codesign -d -r- "$SIGNING_TMP/signing-check" 2>&1
echo 'Local signing verified. Future pnpm app builds use this identity automatically.'
