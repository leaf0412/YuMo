import { useEffect, useState, useRef, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '../lib/logger';
import SpriteAnimation, { type SpriteManifest } from '../components/SpriteAnimation';

type PipelineState = 'recording' | 'transcribing' | 'enhancing' | 'pasting' | 'idle';

export default function RecorderFloat() {
  const [state, setState] = useState<PipelineState>('idle');
  const [duration, setDuration] = useState(0);
  const timerRef = useRef<number | null>(null);

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

  // Listen for state changes
  useEffect(() => {
    const unlisten = listen<{ state: string }>('recording-state', (e) => {
      const s = e.payload.state as PipelineState;
      setState(s);

      // Reset timer when a new recording starts
      if (s === 'recording') {
        setDuration(0);
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Timer — only runs while state === 'recording'
  useEffect(() => {
    // Always clear any existing timer first
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    if (state === 'recording') {
      timerRef.current = window.setInterval(() => {
        setDuration(d => d + 1);
      }, 1000);
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

  const stateLabel: Record<string, string> = {
    recording: '录音中',
    transcribing: '转录中...',
    enhancing: '增强中...',
    pasting: '粘贴中...',
  };

  const hasSprite = spriteManifest && spriteImageSrc;
  const isRecording = state === 'recording';

  // Don't render anything when idle (window is hidden anyway)
  if (state === 'idle') return null;

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
        WebkitAppRegion: 'drag' as never,
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
          background: isRecording ? '#ff4d4f' : '#1890ff',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          animation: isRecording ? 'pulse 1.5s infinite' : 'none',
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
          background: isRecording ? '#ff4d4f' : '#1890ff',
          animation: isRecording ? 'pulse 1.5s infinite' : 'none',
        }} />
        <span>{stateLabel[state] || state}</span>
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
