#!/bin/bash
# Re-sign the built app with correct bundle identifier
APP="src-tauri/target/release/bundle/macos/voiceink-tauri.app"
if [ -d "$APP" ]; then
  codesign --force --deep --sign - --identifier "com.voiceink.app" "$APP"
  echo "Re-signed $APP with identifier com.voiceink.app"
fi
