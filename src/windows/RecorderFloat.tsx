import { useEffect, useState, useRef, useCallback } from 'react';
import { invoke } from '../lib/logger';
import {
  type PipelineState,
  PIPELINE_IDLE,
  PIPELINE_RECORDING,
  CMD_GET_PIPELINE_STATE,
  PIPELINE_LABELS,
  PIPELINE_COLORS,
  PIPELINE_ANIMATIONS,
  parsePipelineState,
} from '../lib/pipeline';
import SpriteAnimation, { type SpriteManifest } from '../components/SpriteAnimation';

const TIMER_INTERVAL_MS = 1000;

export default function RecorderFloat() {
  const [state, setState] = useState<PipelineState>(PIPELINE_IDLE);
  const [duration, setDuration] = useState(0);
  const timerRef = useRef<number | null>(null);
  const prevStateRef = useRef<PipelineState>(PIPELINE_IDLE);

  // Sprite
  const [spriteManifest, setSpriteManifest] = useState<SpriteManifest | null>(null);
  const [spriteImageSrc, setSpriteImageSrc] = useState<string | null>(null);

  const loadSprite = useCallback(async () => {
    try {
      const sprites = await invoke<(SpriteManifest & { dirId: string })[]>('list_sprites');
      if (sprites.length === 0) return;
      const first = sprites[0];
      setSpriteManifest(first);
      try {
        const uri = await invoke<string>('get_sprite_image', { dirId: first.dirId, fileName: 'sprite_processed.png' });
        setSpriteImageSrc(uri);
      } catch {
        const uri = await invoke<string>('get_sprite_image', { dirId: first.dirId, fileName: first.spriteFile });
        setSpriteImageSrc(uri);
      }
    } catch { /* no sprites */ }
  }, []);

  useEffect(() => { loadSprite(); }, [loadSprite]);

  // Shared state-transition handler
  const applyState = useCallback((next: PipelineState) => {
    if (next === PIPELINE_RECORDING && prevStateRef.current !== PIPELINE_RECORDING) {
      setDuration(0);
    }
    prevStateRef.current = next;
    setState(next);
  }, []);

  // Poll pipeline state — cross-window events are unreliable in Tauri
  useEffect(() => {
    const poll = () => {
      invoke<{ state: string }>(CMD_GET_PIPELINE_STATE)
        .then((result) => applyState(parsePipelineState(result.state)))
        .catch(() => {});
    };
    poll();
    const id = setInterval(poll, 500);
    return () => clearInterval(id);
  }, [applyState]);

  // Timer — only runs while recording
  useEffect(() => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    if (state === PIPELINE_RECORDING) {
      timerRef.current = window.setInterval(() => {
        setDuration(d => d + 1);
      }, TIMER_INTERVAL_MS);
    }

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [state]);

  const formatTime = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
  };

  const hasSprite = spriteManifest && spriteImageSrc;
  const isRecording = state === PIPELINE_RECORDING;
  const color = PIPELINE_COLORS[state];
  const animation = PIPELINE_ANIMATIONS[state];

  return (
    <div
      data-tauri-drag-region
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        width: '100%',
        height: '100%',
        userSelect: 'none',
      }}
    >
      {hasSprite ? (
        <SpriteAnimation
          manifest={spriteManifest}
          imageSrc={spriteImageSrc}
          isPlaying={isRecording}
          width={160}
          height={160}
        />
      ) : (
        <div style={{
          width: 80,
          height: 80,
          borderRadius: '50%',
          background: color,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          animation,
        }}>
          <span style={{ fontSize: 32, color: '#fff' }}>🎙</span>
        </div>
      )}

      <div style={{
        marginTop: 4,
        padding: '2px 12px',
        borderRadius: 12,
        background: 'rgba(0,0,0,0.7)',
        color: '#fff',
        fontSize: 12,
        fontFamily: '-apple-system, BlinkMacSystemFont, sans-serif',
        display: 'flex',
        alignItems: 'center',
        gap: 6,
      }}>
        <div style={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          background: color,
          animation,
        }} />
        <span>{PIPELINE_LABELS[state]}</span>
        {isRecording && (
          <span style={{ fontVariantNumeric: 'tabular-nums' }}>{formatTime(duration)}</span>
        )}
      </div>

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }
      `}</style>
    </div>
  );
}
