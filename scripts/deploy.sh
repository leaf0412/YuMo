#!/bin/bash
# Deploy built app to /Applications, re-sign, and restore permissions.

set -e

APP_SRC="src-tauri/target/release/bundle/macos/voiceink-tauri.app"
APP_DST="/Applications/voiceink-tauri.app"
BUNDLE_ID="com.voiceink.app"
ENTITLEMENTS="src-tauri/entitlements.plist"

if [ ! -d "$APP_SRC" ]; then
  echo "Build first: npm run package"
  exit 1
fi

# Copy app
if [ -d "$APP_DST" ]; then
  rm -rf "$APP_DST"
fi
cp -R "$APP_SRC" "$APP_DST"

# Sign with consistent identity + entitlements
codesign --force --deep --sign - --identifier "$BUNDLE_ID" \
  --entitlements "$ENTITLEMENTS" "$APP_DST"

echo "Deployed to $APP_DST"
echo ""
echo "If permissions were lost, run:"
echo "  open 'x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility'"
echo ""
echo "Or grant via command line (requires sudo):"
echo "  sudo sqlite3 ~/Library/Application\\ Support/com.apple.TCC/TCC.db \\"
echo "    \"INSERT OR REPLACE INTO access VALUES('kTCCServiceAccessibility','$BUNDLE_ID',0,2,4,1,NULL,NULL,0,'UNUSED',NULL,0,$(date +%s));\""
