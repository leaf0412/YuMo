#!/bin/bash
# Deploy built app to /Applications
set -e

APP_SRC="target/release/bundle/macos/YuMo.app"
APP_DST="/Applications/YuMo.app"

if [ ! -d "$APP_SRC" ]; then
  echo "Build first: npm run package"
  exit 1
fi

[ -d "$APP_DST" ] && rm -rf "$APP_DST"
cp -R "$APP_SRC" "$APP_DST"
codesign --force --deep --sign - --identifier "com.yumo.app" \
  --entitlements "src-tauri/entitlements.plist" "$APP_DST"
echo "Deployed. Re-grant permissions if needed."
