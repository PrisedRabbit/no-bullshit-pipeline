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
  echo "--------------------------------------------------"
  echo "✅ BUILD COMPLETE!"
  echo "📦 Artifact: builds/$TARGET_FILENAME"
  echo "--------------------------------------------------"
else
  echo "❌ Error: DMG file not found in $SEARCH_PATH"
  exit 1
fi
