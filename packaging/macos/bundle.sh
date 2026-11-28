#!/usr/bin/env bash
# Wrap the built binary in Hangman.app and archive it. Run on macOS.
# Usage: packaging/macos/bundle.sh <binary> <output.tar.gz>
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
binary="$1"
output="$2"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/../../Cargo.toml" | head -1)"

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
app="$staging/Hangman.app"

mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$(dirname "$output")"
install -m755 "$binary" "$app/Contents/MacOS/hangman"
cp "$here/hangman.icns" "$app/Contents/Resources/hangman.icns"
sed "s/@VERSION@/$version/g" "$here/Info.plist" > "$app/Contents/Info.plist"

# Ad-hoc signature so the bundle is internally consistent. This is not notarization.
codesign --force --sign - "$app"

tar -czf "$output" -C "$staging" Hangman.app
