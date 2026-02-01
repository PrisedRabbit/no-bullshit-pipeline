#!/bin/bash
set -e

echo "🚀 Starting build process..."

# 1. Run Tauri Build
bun tauri build

# 2. Extract Name and Version from tauri.conf.json
NAME=$(grep '"productName":' src-tauri/tauri.conf.json | cut -d'"' -f4)
VERSION=$(grep '"version":' src-tauri/tauri.conf.json | cut -d'"' -f4)

# 3. Prepare builds directory
mkdir -p builds

# 4. Find generated DMG
# Searching in the bundle output directory
SEARCH_PATH="src-tauri/target/release/bundle"
# Finding the most recent .dmg file in the bundle path
DMG_PATH=$(find "$SEARCH_PATH" -name "*.dmg" -not -name "rw.*" | head -n 1)

# Fallback: if tauri used a temporary "rw.XXXX" name, take it anyway
if [ -z "$DMG_PATH" ]; then
  DMG_PATH=$(find "$SEARCH_PATH" -name "*.dmg" | head -n 1)
fi

if [ -n "$DMG_PATH" ]; then
  TARGET_FILENAME="${NAME}_v${VERSION}.dmg"
  cp "$DMG_PATH" "builds/$TARGET_FILENAME"

  # 5. Find and verify the .app bundle
  APP_PATH=$(find "$SEARCH_PATH" -name "*.app" -type d | head -n 1)

  if [ -n "$APP_PATH" ]; then
    echo ""
    echo "🔐 Verifying code signature and entitlements..."
    echo ""

    # Show signature info
    echo "Signature:"
    codesign -dvvv "$APP_PATH" 2>&1 | grep -E "(Authority|Identifier|TeamIdentifier)" || echo "  (unsigned or ad-hoc signed)"
    echo ""

    # Show entitlements
    echo "Entitlements:"
    codesign -d --entitlements :- "$APP_PATH" 2>&1 || echo "  (no entitlements or unsigned)"
    echo ""

    # Check Gatekeeper (if signed)
    echo "Gatekeeper check:"
    spctl -a -vvv "$APP_PATH" 2>&1 || echo "  (Gatekeeper check skipped - may need proper signing)"
  fi

  echo "--------------------------------------------------"
  echo "✅ BUILD COMPLETE!"
  echo "📦 Artifact: builds/$TARGET_FILENAME"
  echo "--------------------------------------------------"
else
  echo "❌ Error: DMG file not found in $SEARCH_PATH"
  exit 1
fi
