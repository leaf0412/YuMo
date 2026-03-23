import { useEffect, useState, useRef, useCallback } from 'react';
import { listen } from '../lib/events';
import { useTranslation } from 'react-i18next';
import i18n from '../i18n';
import { invoke } from '../lib/logger';
import { onBroadcast } from '../lib/broadcast';
import {
  type PipelineState,
  PIPELINE_IDLE,
  PIPELINE_RECORDING,
  CMD_GET_PIPELINE_STATE,
  PIPELINE_LABEL_KEYS,
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

  const { t } = useTranslation();

  // ESC hint
  type EscHintType = 'cancelled' | 'pressAgain' | null;
  const [escHintType, setEscHintType] = useState<EscHintType>(null);
  const escHintTimer = useRef<number | null>(null);

  // Sprite
  const [spriteManifest, setSpriteManifest] = useState<SpriteManifest | null>(null);
  const [spriteImageSrc, setSpriteImageSrc] = useState<string | null>(null);
  const [spriteSize, setSpriteSize] = useState(180);

  const loadSprite = useCallback(async () => {
    invoke('frontend_log', { level: 'info', message: '[recorder] loadSprite start' });
    try {
      const sprites = await invoke<(SpriteManifest & { dirId: string })[]>('list_sprites');
      invoke('frontend_log', { level: 'info', message: `[recorder] sprites found: ${sprites.length}` });
      if (sprites.length === 0) return;

      // Use selected sprite from settings, fallback to first
      let settings: { selected_sprite_id?: string; sprite_size?: number } = {};
      try {
        settings = await invoke<typeof settings>('get_settings');
      } catch { /* use default */ }

      if (settings.sprite_size) setSpriteSize(settings.sprite_size);
      const selectedId = settings.selected_sprite_id;
      const target = (selectedId && sprites.find(s => s.dirId === selectedId)) || sprites[0];

      setSpriteManifest(target);
      try {
        const uri = await invoke<string>('get_sprite_image', { dirId: target.dirId, fileName: 'sprite_processed.png' });
        invoke('frontend_log', { level: 'info', message: `[recorder] sprite image loaded, len=${uri.length}` });
        setSpriteImageSrc(uri);
      } catch {
        const uri = await invoke<string>('get_sprite_image', { dirId: target.dirId, fileName: target.spriteFile });
        setSpriteImageSrc(uri);
      }
    } catch (e) { invoke('frontend_log', { level: 'error', message: `[recorder] loadSprite failed: ${e}` }); }
  }, []);

  useEffect(() => { loadSprite(); }, [loadSprite]);

  // Reload sprite when settings change
  useEffect(() => {
    const cleanup = onBroadcast('settings-changed', (key) => {
      if (key === 'selected_sprite_id' || key === 'sprite_size') {
        loadSprite();
      }
    });
    return cleanup;
  }, [loadSprite]);

  // Listen for ESC hints from both BroadcastChannel (Tauri) and IPC (Electron)
  useEffect(() => {
    const handleHint = (hint: EscHintType) => {
      setEscHintType(hint);
      if (hint === 'pressAgain') {
        if (escHintTimer.current) clearTimeout(escHintTimer.current);
        escHintTimer.current = window.setTimeout(() => setEscHintType(null), 2000);
      }
    };

    // BroadcastChannel: from App.tsx (same-origin, works in both Tauri and Electron)
    const cleanupBroadcast = onBroadcast('escape-hint', (payload) => {
      handleHint(payload as EscHintType);
    });

    // IPC: from Electron main process (audio.ts sends escape-hint directly)
    const unlistenIpc = listen<string>('escape-hint', (event) => {
      handleHint(event.payload as EscHintType);
    });

    return () => {
      cleanupBroadcast();
      unlistenIpc.then(fn => fn());
      if (escHintTimer.current) clearTimeout(escHintTimer.current);
    };
  }, []);

  // Sync language when changed from main window
  useEffect(() => {
    const unlisten = listen<string>('language-changed', (event) => {
      i18n.changeLanguage(event.payload);
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

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

  // Dragging handled natively via NSWindow setMovableByWindowBackground

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
        position: 'relative',
        width: '100%',
        height: '100%',
        userSelect: 'none',
        cursor: 'grab',
        // @ts-expect-error Electron uses this for window dragging
        WebkitAppRegion: 'drag',
      }}
    >
      {hasSprite ? (
        <SpriteAnimation
          manifest={spriteManifest}
          imageSrc={spriteImageSrc}
          isPlaying={isRecording}
          width={spriteSize}
          height={spriteSize}
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
        <span>{PIPELINE_LABEL_KEYS[state] ? t(PIPELINE_LABEL_KEYS[state]) : ''}</span>
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
            // @ts-expect-error Electron: make button clickable (not draggable)
            WebkitAppRegion: 'no-drag',
            marginLeft: 4,
            opacity: 0.7,
            fontSize: 14,
            lineHeight: 1,
          }}
          title={t('recorder.cancel')}
        >
          ✕
        </span>
      </div>

      {escHintType && (
        <div style={{
          position: 'absolute',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          padding: '6px 16px',
          borderRadius: 12,
          background: escHintType === 'cancelled' ? 'rgba(255,77,79,0.9)' : 'rgba(0,0,0,0.75)',
          color: '#fff',
          fontSize: 13,
          fontWeight: 500,
          fontFamily: '-apple-system, BlinkMacSystemFont, sans-serif',
          pointerEvents: 'none',
          whiteSpace: 'nowrap',
          zIndex: 10,
          transition: 'opacity 0.2s',
        }}>
          {escHintType === 'cancelled' ? t('recorder.cancelledHint') : t('recorder.escHint')}
        </div>
      )}

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }
      `}</style>
    </div>
  );
}
