#!/bin/bash
# Deploy built app to /Applications without re-signing
# This preserves the existing signature hash so macOS accessibility
# permissions don't need to be re-granted.

set -e

APP_SRC="src-tauri/target/release/bundle/macos/voiceink-tauri.app"
APP_DST="/Applications/voiceink-tauri.app"

if [ ! -d "$APP_SRC" ]; then
  echo "Build first: CMAKE_OSX_DEPLOYMENT_TARGET=11.0 npx tauri build"
  exit 1
fi

# First install: copy everything
if [ ! -d "$APP_DST" ]; then
  echo "First install: copying to /Applications/"
  cp -R "$APP_SRC" "$APP_DST"
  echo "Done. Grant accessibility permission in System Settings."
  exit 0
fi

# Subsequent: only update binary and resources, keep existing signature
echo "Updating binary and resources (preserving signature)..."
cp "$APP_SRC/Contents/MacOS/voiceink-tauri" "$APP_DST/Contents/MacOS/voiceink-tauri"
rsync -a "$APP_SRC/Contents/Resources/" "$APP_DST/Contents/Resources/"
echo "Updated. No need to re-grant permissions."
