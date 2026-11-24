#!/usr/bin/env bash
# Install Hangman for the current user (no sudo): binary, icon, and app-launcher entry.
# Usage: ./install.sh [--uninstall]
set -euo pipefail

cd "$(dirname "$0")"
PREFIX="${PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin/hangman"
ICON="$PREFIX/share/icons/hicolor/scalable/apps/hangman.svg"
DESKTOP="$PREFIX/share/applications/hangman.desktop"

refresh_caches() {
  command -v update-desktop-database >/dev/null && update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
  command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
}

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -fv "$BIN" "$ICON" "$DESKTOP"
  refresh_caches
  echo "Hangman uninstalled."
  exit 0
fi

echo "Building release binary..."
cargo build --release -p hangman

install -Dm755 target/release/hangman "$BIN"
install -Dm644 packaging/linux/hangman.svg "$ICON"
install -Dm644 packaging/linux/hangman.desktop "$DESKTOP"
refresh_caches

echo "Installed to $PREFIX. Launch 'Hangman' from your app launcher, or run: hangman"
case ":$PATH:" in *":$PREFIX/bin:"*) ;; *) echo "Note: $PREFIX/bin is not on your PATH." ;; esac
