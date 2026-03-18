/**
 * SpriteAnimation — Plays a sprite sheet animation via CSS steps().
 *
 * Uses CSS background-position stepping instead of canvas —
 * works reliably in Tauri transparent windows on macOS.
 */
import { useEffect, useState } from 'react';

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
  /** Seconds per frame (default 0.08) */
  timePerFrame?: number;
  windDownMs?: number;
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
  const [animating, setAnimating] = useState(false);

  useEffect(() => {
    if (isPlaying) {
      setAnimating(true);
    } else if (animating) {
      // Wind-down: keep animating then stop
      const timer = setTimeout(() => setAnimating(false), windDownMs);
      return () => clearTimeout(timer);
    }
  }, [isPlaying, windDownMs]); // eslint-disable-line react-hooks/exhaustive-deps

  const { frameCount, columns, rows } = manifest;
  const totalDuration = frameCount * timePerFrame;
  const bgWidth = columns * width;
  const bgHeight = rows * height;
  const gridKeyframes = Array.from({ length: frameCount }, (_, i) => {
    const col = i % columns;
    const row = Math.floor(i / columns);
    const pct = (i / frameCount) * 100;
    const x = -(col * width);
    const y = -(row * height);
    return `${pct.toFixed(2)}% { background-position: ${x}px ${y}px; }`;
  }).join('\n');

  const gridAnimName = `sprite-grid-${manifest.id.replace(/[^a-zA-Z0-9]/g, '')}`;
  const gridKeyframesCss = `
    @keyframes ${gridAnimName} {
      ${gridKeyframes}
      100% { background-position: 0px 0px; }
    }
  `;

  return (
    <>
      <style>{gridKeyframesCss}</style>
      <div
        style={{
          width,
          height,
          backgroundImage: `url(${imageSrc})`,
          backgroundSize: `${bgWidth}px ${bgHeight}px`,
          backgroundRepeat: 'no-repeat',
          backgroundPosition: '0 0',
          animation: animating
            ? `${gridAnimName} ${totalDuration}s steps(1) infinite`
            : 'none',
        }}
      />
    </>
  );
}
