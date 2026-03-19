#!/bin/bash
# Deploy strategy: first install copies everything and signs.
# Subsequent deploys only update frontend resources (not the binary),
# so the code signature stays valid and permissions are preserved.
#
# If backend Rust code changed, use --full flag to replace everything
# (will require re-granting permissions).

set -e

APP_SRC="src-tauri/target/release/bundle/macos/voiceink-tauri.app"
APP_DST="/Applications/voiceink-tauri.app"
BUNDLE_ID="com.voiceink.app"
ENTITLEMENTS="src-tauri/entitlements.plist"
FULL_DEPLOY=false

if [ "$1" = "--full" ]; then
  FULL_DEPLOY=true
fi

if [ ! -d "$APP_SRC" ]; then
  echo "Build first: npm run package"
  exit 1
fi

# First install or --full: copy everything and sign
if [ ! -d "$APP_DST" ] || [ "$FULL_DEPLOY" = true ]; then
  echo "Full deploy: copying app bundle..."
  [ -d "$APP_DST" ] && rm -rf "$APP_DST"
  cp -R "$APP_SRC" "$APP_DST"
  codesign --force --deep --sign - --identifier "$BUNDLE_ID" \
    --entitlements "$ENTITLEMENTS" "$APP_DST"
  echo "Done. Grant permissions in System Settings if first install."
  exit 0
fi

# Hot deploy: only update frontend resources (preserves binary + signature)
echo "Hot deploy: updating frontend resources only..."
rsync -a --delete \
  "$APP_SRC/Contents/Resources/" \
  "$APP_DST/Contents/Resources/" \
  --exclude "*.dylib"
echo "Updated. Binary unchanged, permissions preserved."
