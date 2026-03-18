import { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

interface AudioLevel {
  average: number;
  peak: number;
}

type RecordingState = 'recording' | 'transcribing' | 'enhancing' | 'pasting';

export default function RecorderFloat() {
  const [state, setState] = useState<RecordingState>('recording');
  const [duration, setDuration] = useState(0);
  const [audioLevel, setAudioLevel] = useState<AudioLevel>({ average: 0, peak: 0 });
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const timerRef = useRef<number | null>(null);
  const levelsRef = useRef<number[]>([]);

  // Listen for state changes
  useEffect(() => {
    const unlisten = listen<{ state: string }>('recording-state', (e) => {
      setState(e.payload.state as RecordingState);
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Listen for audio levels
  useEffect(() => {
    const unlisten = listen<AudioLevel>('audio-level', (e) => {
      setAudioLevel(e.payload);
      levelsRef.current.push(e.payload.average);
      if (levelsRef.current.length > 50) levelsRef.current.shift();
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Timer
  useEffect(() => {
    if (state === 'recording') {
      setDuration(0);
      timerRef.current = window.setInterval(() => {
        setDuration(d => d + 1);
      }, 1000);
    } else if (timerRef.current) {
      clearInterval(timerRef.current);
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [state]);

  // Draw waveform
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const draw = () => {
      const { width, height } = canvas;
      ctx.clearRect(0, 0, width, height);

      const levels = levelsRef.current;
      const barWidth = width / 50;

      ctx.fillStyle = '#1890ff';
      levels.forEach((level, i) => {
        const barHeight = Math.max(2, level * height * 3);
        const x = i * barWidth;
        const y = (height - barHeight) / 2;
        ctx.fillRect(x, y, barWidth - 1, barHeight);
      });

      requestAnimationFrame(draw);
    };
    const id = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(id);
  }, []);

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

  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      padding: '8px 16px',
      background: 'rgba(0,0,0,0.85)',
      borderRadius: 40,
      color: '#fff',
      fontFamily: '-apple-system, BlinkMacSystemFont, sans-serif',
      fontSize: 13,
      userSelect: 'none',
      WebkitAppRegion: 'drag' as never,
    }}>
      {/* Red dot indicator */}
      <div style={{
        width: 10,
        height: 10,
        borderRadius: '50%',
        background: state === 'recording' ? '#ff4d4f' : '#1890ff',
        animation: state === 'recording' ? 'pulse 1.5s infinite' : 'none',
      }} />

      {/* Status text */}
      <span>{stateLabel[state] || state}</span>

      {/* Waveform canvas */}
      {state === 'recording' && (
        <canvas
          ref={canvasRef}
          width={150}
          height={30}
          style={{ display: 'block' }}
        />
      )}

      {/* Timer */}
      {state === 'recording' && (
        <span style={{ fontVariantNumeric: 'tabular-nums', minWidth: 36 }}>
          {formatTime(duration)}
        </span>
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
