import { useEffect, useState, useRef, useCallback } from 'react';
import { invoke } from '../lib/logger';
import { onBroadcast } from '../lib/broadcast';
import { getCurrentWindow } from '@tauri-apps/api/window';
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
    invoke('frontend_log', { level: 'info', message: '[recorder] loadSprite start' });
    try {
      const sprites = await invoke<(SpriteManifest & { dirId: string })[]>('list_sprites');
      invoke('frontend_log', { level: 'info', message: `[recorder] sprites found: ${sprites.length}` });
      if (sprites.length === 0) return;
      const first = sprites[0];
      setSpriteManifest(first);
      try {
        const uri = await invoke<string>('get_sprite_image', { dirId: first.dirId, fileName: 'sprite_processed.png' });
        invoke('frontend_log', { level: 'info', message: `[recorder] sprite image loaded, len=${uri.length}` });
        setSpriteImageSrc(uri);
      } catch {
        const uri = await invoke<string>('get_sprite_image', { dirId: first.dirId, fileName: first.spriteFile });
        setSpriteImageSrc(uri);
      }
    } catch (e) { invoke('frontend_log', { level: 'error', message: `[recorder] loadSprite failed: ${e}` }); }
  }, []);

  useEffect(() => { loadSprite(); }, [loadSprite]);

  // Shared state-transition handler
  const applyState = useCallback((next: PipelineState) => {
    invoke('frontend_log', { level: 'info', message: `[recorder] state: ${prevStateRef.current} -> ${next}` });
    if (next === PIPELINE_RECORDING && prevStateRef.current !== PIPELINE_RECORDING) {
      setDuration(0);
    }
    prevStateRef.current = next;
    setState(next);
  }, []);

  // Sync state: initial query + BroadcastChannel from main window
  useEffect(() => {
    // One-time sync on mount
    invoke<{ state: string }>(CMD_GET_PIPELINE_STATE)
      .then((result) => applyState(parsePipelineState(result.state)))
      .catch(() => {});

    // Listen for state changes broadcast from main window
    const cleanup = onBroadcast('pipeline-state', (payload) => {
      applyState(parsePipelineState(payload as string));
    });
    return cleanup;
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

  // Window dragging — attach at document level
  useEffect(() => {
    const onMouseDown = (e: MouseEvent) => {
      if ((e.target as HTMLElement)?.closest?.('[data-cancel]')) return;
      invoke('frontend_log', { level: 'info', message: '[recorder] mousedown -> startDragging' });
      getCurrentWindow().startDragging().catch((err) => {
        invoke('frontend_log', { level: 'error', message: `[recorder] startDragging failed: ${err}` });
      });
    };
    document.addEventListener('mousedown', onMouseDown);
    return () => document.removeEventListener('mousedown', onMouseDown);
  }, []);

  const hasSprite = spriteManifest && spriteImageSrc;
  const isRecording = state === PIPELINE_RECORDING;
  const color = PIPELINE_COLORS[state];
  const animation = PIPELINE_ANIMATIONS[state];

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        width: '100%',
        height: '100%',
        userSelect: 'none',
        cursor: 'grab',
      }}
    >
      {hasSprite ? (
        <SpriteAnimation
          manifest={spriteManifest}
          imageSrc={spriteImageSrc}
          isPlaying={isRecording}
          width={180}
          height={180}
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
          pointerEvents: 'none',
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
        pointerEvents: 'none',
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
        {/* Cancel button — no drag region so it's clickable */}
        <span
          data-cancel
          onClick={(e) => {
            e.stopPropagation();
            invoke('cancel_recording').catch(() => {});
          }}
          style={{
            cursor: 'pointer',
            pointerEvents: 'auto',
            marginLeft: 4,
            opacity: 0.7,
            fontSize: 14,
            lineHeight: 1,
          }}
          title="取消"
        >
          ✕
        </span>
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
