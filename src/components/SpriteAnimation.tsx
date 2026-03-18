/**
 * SpriteAnimation — Plays a sprite sheet animation on a canvas.
 *
 * Compatible with VoiceInk native manifest.json format.
 * Features: grid slicing, 12.5fps default, 3s wind-down after stop.
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

export default function SpriteAnimation({
  manifest,
  imageSrc,
  isPlaying,
  width = 120,
  height = 120,
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
    if (!image || !canvasRef.current) return;
    drawFrame(canvasRef.current, image, manifest, 0);
  }, [image, manifest]);

  // Animation loop
  useEffect(() => {
    if (!image || !canvasRef.current) return;

    const canvas = canvasRef.current;
    let frame = 0;
    let lastTime = performance.now();
    let running = true;
    let rafId = 0;
    const intervalMs = timePerFrame * 1000;

    const tick = (timestamp: number) => {
      if (!running) return;
      if (timestamp - lastTime >= intervalMs) {
        lastTime = timestamp;
        frame = (frame + 1) % manifest.frameCount;
        drawFrame(canvas, image, manifest, frame);
      }
      rafId = requestAnimationFrame(tick);
    };

    // Always start the animation loop immediately
    rafId = requestAnimationFrame(tick);

    // If not playing, schedule stop after wind-down
    let windDownTimer: ReturnType<typeof setTimeout> | null = null;
    if (!isPlaying) {
      windDownTimer = setTimeout(() => {
        running = false;
        cancelAnimationFrame(rafId);
        drawFrame(canvas, image, manifest, 0);
      }, windDownMs);
    }

    return () => {
      running = false;
      cancelAnimationFrame(rafId);
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
    col * manifest.frameWidth,
    row * manifest.frameHeight,
    manifest.frameWidth,
    manifest.frameHeight,
    0, 0,
    canvas.width,
    canvas.height,
  );
}
