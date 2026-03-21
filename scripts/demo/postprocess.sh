#!/bin/bash
set -euo pipefail

RAW_FILE="${1:?Usage: postprocess.sh <raw.mp4> <output_dir>}"
OUTPUT_DIR="${2:?Usage: postprocess.sh <raw.mp4> <output_dir>}"

echo "[postprocess] input: $RAW_FILE"

# --- 1. Crop to app window area (1280x800 from top-left) ---
CROPPED="$OUTPUT_DIR/demo.mp4"
ffmpeg -y -i "$RAW_FILE" \
    -filter:v "crop=1280:800:0:0" \
    -c:v libx264 -crf 23 -preset medium \
    -an \
    "$CROPPED"
echo "[postprocess] cropped MP4: $CROPPED ($(du -h "$CROPPED" | cut -f1))"

# --- 2. Generate full GIF ---
FULL_GIF="$OUTPUT_DIR/demo.gif"
PALETTE="$OUTPUT_DIR/_palette.png"

ffmpeg -y -i "$CROPPED" \
    -vf "fps=15,scale=800:-1:flags=lanczos,palettegen=stats_mode=diff" \
    "$PALETTE"

ffmpeg -y -i "$CROPPED" -i "$PALETTE" \
    -lavfi "fps=15,scale=800:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=3" \
    "$FULL_GIF"

rm -f "$PALETTE"

GIF_SIZE=$(stat -f%z "$FULL_GIF" 2>/dev/null || stat --printf="%s" "$FULL_GIF")
GIF_SIZE_MB=$((GIF_SIZE / 1024 / 1024))
echo "[postprocess] full GIF: $FULL_GIF (${GIF_SIZE_MB}MB)"

if [ "$GIF_SIZE_MB" -gt 10 ]; then
    echo "[postprocess] WARNING: GIF is ${GIF_SIZE_MB}MB (>10MB), consider splitting by scenario"
fi

# --- 3. Split per-scenario GIFs if timestamps exist ---
TIMESTAMPS="$OUTPUT_DIR/timestamps.json"
if [ -f "$TIMESTAMPS" ] && command -v jq &>/dev/null; then
    echo "[postprocess] splitting by scenario timestamps..."
    jq -c '.[]' "$TIMESTAMPS" | while read -r entry; do
        NAME=$(echo "$entry" | jq -r '.name')
        START=$(echo "$entry" | jq -r '.start')
        END=$(echo "$entry" | jq -r '.end')
        DURATION=$(echo "$END - $START" | bc)

        SCENE_GIF="$OUTPUT_DIR/demo-${NAME}.gif"
        SCENE_PALETTE="$OUTPUT_DIR/_palette_${NAME}.png"

        ffmpeg -y -i "$CROPPED" -ss "$START" -t "$DURATION" \
            -vf "fps=15,scale=800:-1:flags=lanczos,palettegen=stats_mode=diff" \
            "$SCENE_PALETTE"

        ffmpeg -y -i "$CROPPED" -ss "$START" -t "$DURATION" -i "$SCENE_PALETTE" \
            -lavfi "fps=15,scale=800:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=3" \
            "$SCENE_GIF"

        rm -f "$SCENE_PALETTE"
        echo "[postprocess] scene GIF: $SCENE_GIF ($(du -h "$SCENE_GIF" | cut -f1))"
    done
fi

# --- 4. Clean up raw file ---
rm -f "$RAW_FILE"

echo "[postprocess] done!"
