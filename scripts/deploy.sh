#!/bin/bash
# Deploy built app to /Applications
set -e

APP_SRC="src-tauri/target/release/bundle/macos/voiceink-tauri.app"
APP_DST="/Applications/voiceink-tauri.app"

if [ ! -d "$APP_SRC" ]; then
  echo "Build first: npm run package"
  exit 1
fi

[ -d "$APP_DST" ] && rm -rf "$APP_DST"
cp -R "$APP_SRC" "$APP_DST"
codesign --force --deep --sign - --identifier "com.voiceink.app" \
  --entitlements "src-tauri/entitlements.plist" "$APP_DST"
echo "Deployed. Re-grant permissions if needed."
