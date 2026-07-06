#!/bin/bash
# Build the CortexIntel .app and package a drag-to-Applications .dmg.
# Tauri's own DMG step uses AppleScript to style the installer window, which
# fails in headless/no-GUI contexts; this script builds the .app and makes the
# .dmg with hdiutil (no AppleScript), producing a valid installer either way.
set -euo pipefail
cd "$(dirname "$0")/src-tauri"
export PATH="$HOME/.cargo/bin:$PATH"

echo "→ building .app …"
cargo tauri build || true   # .app builds even if the AppleScript dmg step errors

APP="target/release/bundle/macos/CortexIntel.app"
[ -d "$APP" ] || { echo "ERROR: $APP not found"; exit 1; }

OUT_DIR="target/release/bundle/dmg"
OUT="$OUT_DIR/CortexIntel_0.1.0_aarch64.dmg"
mkdir -p "$OUT_DIR"
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$OUT"
hdiutil create -volname "CortexIntel" -srcfolder "$STAGE" -ov -format UDZO "$OUT"
rm -rf "$STAGE"
echo "✓ installer: $OUT"
