#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/output"
SCENARIO_DIR="$SCRIPT_DIR/scenarios"

# PIDs for cleanup
FFMPEG_PID=""
APP_PID=""
DRIVER_PID=""

cleanup() {
    echo "[record] cleaning up..."
    [ -n "$FFMPEG_PID" ] && kill "$FFMPEG_PID" 2>/dev/null || true
    [ -n "$APP_PID" ] && kill "$APP_PID" 2>/dev/null || true
    [ -n "$DRIVER_PID" ] && kill "$DRIVER_PID" 2>/dev/null || true
    wait 2>/dev/null || true
    echo "[record] cleanup done"
}
trap cleanup EXIT

# --- 0. Parse args ---
SCENARIO_FILTER="${1:-all}"

# --- 1. Dependency check ---
for cmd in ffmpeg tauri-driver npx; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "[record] ERROR: $cmd not found. Install it first."
        exit 1
    fi
done

# --- 2. Build app (Tauri dev build bundles frontend + Rust) ---
echo "[record] building app..."
cd "$PROJECT_DIR"
npm run build
cd src-tauri && cargo build && cd ..
echo "[record] build complete"

# --- 3. Prepare output dir ---
mkdir -p "$OUTPUT_DIR"

# --- 4. Copy demo audio assets to data dir ---
DATA_DIR="$HOME/.voiceink"
mkdir -p "$DATA_DIR"
if [ -f "$SCRIPT_DIR/assets/demo-audio-zh.wav" ]; then
    cp "$SCRIPT_DIR/assets/demo-audio-zh.wav" "$DATA_DIR/"
    echo "[record] demo audio copied to $DATA_DIR"
else
    echo "[record] ERROR: demo-audio-zh.wav not found in $SCRIPT_DIR/assets/"
    echo "[record] Please record a 5-second Chinese speech WAV (16kHz, mono) and place it there."
    exit 1
fi

# --- 5. Launch app in demo mode ---
echo "[record] launching app in demo mode..."
APP_BIN="$PROJECT_DIR/src-tauri/target/debug/yumo"
if [ ! -f "$APP_BIN" ]; then
    echo "[record] ERROR: binary not found at $APP_BIN"
    echo "[record] Check 'cargo build' output and verify binary name."
    exit 1
fi
YUMO_DEMO=1 "$APP_BIN" &
APP_PID=$!
sleep 3

# --- 6. Position window via AppleScript ---
osascript -e '
tell application "System Events"
    tell process "yumo"
        set position of window 1 to {0, 0}
        set size of window 1 to {1280, 800}
    end tell
end tell
' 2>/dev/null || echo "[record] WARNING: could not position window"

# --- 7. Start tauri-driver ---
echo "[record] starting tauri-driver..."
tauri-driver --port 4444 &
DRIVER_PID=$!
sleep 2

# --- 8. Start FFmpeg screen recording ---
SCREEN_INDEX=$(ffmpeg -f avfoundation -list_devices true -i "" 2>&1 | grep -n "Capture screen" | head -1 | cut -d: -f1)
SCREEN_INDEX=${SCREEN_INDEX:-1}
echo "[record] using screen capture device index: $SCREEN_INDEX"

RAWFILE="$OUTPUT_DIR/raw.mp4"
ffmpeg -y -f avfoundation -framerate 30 -i "${SCREEN_INDEX}:none" \
    -c:v libx264 -pix_fmt yuv420p -preset ultrafast \
    "$RAWFILE" &
FFMPEG_PID=$!
sleep 1

# --- 9. Run scenarios ---
echo "[record] running scenarios (filter=$SCENARIO_FILTER)..."
if [ "$SCENARIO_FILTER" = "all" ]; then
    npx tsx "$SCENARIO_DIR/run-all.ts" || echo "[record] WARNING: some scenarios failed"
else
    npx tsx "$SCENARIO_DIR/${SCENARIO_FILTER}"*.ts || echo "[record] WARNING: scenario failed"
fi

# --- 10. Stop FFmpeg ---
sleep 1
kill -INT "$FFMPEG_PID" 2>/dev/null || true
wait "$FFMPEG_PID" 2>/dev/null || true
FFMPEG_PID=""
echo "[record] recording stopped"

# --- 11. Post-process ---
echo "[record] post-processing..."
bash "$SCRIPT_DIR/postprocess.sh" "$RAWFILE" "$OUTPUT_DIR"

echo "[record] done! Output:"
ls -lh "$OUTPUT_DIR"/*.{mp4,gif} 2>/dev/null
