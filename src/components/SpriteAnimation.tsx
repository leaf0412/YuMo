/**
 * SpriteAnimation — Canvas sprite sheet animation.
 *
 * Uses setInterval instead of requestAnimationFrame so it works
 * in unfocused/transparent Tauri windows on macOS.
 */
import { useEffect, useRef, useState } from 'react';

export interface SpriteManifest {
  id: string;
  name: string;
  frameCount: number;
  frameWidth: number;
  frameHeight: number;
  columns: number;
  rows: number;
  spriteFile: string;
}

interface Props {
  manifest: SpriteManifest;
  imageSrc: string;
  isPlaying: boolean;
  width?: number;
  height?: number;
  timePerFrame?: number;
  windDownMs?: number;
}

function drawFrame(
  canvas: HTMLCanvasElement,
  img: HTMLImageElement,
  manifest: SpriteManifest,
  frameIndex: number,
) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const col = frameIndex % manifest.columns;
  const row = Math.floor(frameIndex / manifest.columns);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(
    img,
    col * manifest.frameWidth, row * manifest.frameHeight,
    manifest.frameWidth, manifest.frameHeight,
    0, 0, canvas.width, canvas.height,
  );
}

export default function SpriteAnimation({
  manifest,
  imageSrc,
  isPlaying,
  width = 160,
  height = 160,
  timePerFrame = 0.08,
  windDownMs = 3000,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);

  // Load image
  useEffect(() => {
    const img = new Image();
    img.onload = () => setImage(img);
    img.src = imageSrc;
    return () => { img.onload = null; };
  }, [imageSrc]);

  // Draw first frame when image loads
  useEffect(() => {
    if (image && canvasRef.current) {
      drawFrame(canvasRef.current, image, manifest, 0);
    }
  }, [image, manifest]);

  // Animation loop using setInterval (works in unfocused windows, unlike rAF)
  useEffect(() => {
    if (!image || !canvasRef.current) return;

    const canvas = canvasRef.current;
    let frame = 0;
    const intervalMs = timePerFrame * 1000;

    // Always start animating immediately
    const id = setInterval(() => {
      frame = (frame + 1) % manifest.frameCount;
      drawFrame(canvas, image, manifest, frame);
    }, intervalMs);

    // If not playing, stop after wind-down
    let windDownTimer: ReturnType<typeof setTimeout> | null = null;
    if (!isPlaying) {
      windDownTimer = setTimeout(() => {
        clearInterval(id);
        drawFrame(canvas, image, manifest, 0);
      }, windDownMs);
    }

    return () => {
      clearInterval(id);
      if (windDownTimer) clearTimeout(windDownTimer);
    };
  }, [isPlaying, image, manifest, timePerFrame, windDownMs]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      style={{ width, height }}
    />
  );
}
