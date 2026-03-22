import { describe, it, expect, vi, beforeEach, afterAll } from 'vitest';

// Simulate Tauri environment so platformInvoke uses the mocked @tauri-apps/api/core
(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};

// Must use factory function (no external refs) since vi.mock is hoisted
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

import { invoke, formatError } from '../lib/logger';

// Get the mock after import
async function getMockInvoke() {
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke as ReturnType<typeof vi.fn>;
}

afterAll(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe('formatError', () => {
  it('returns string errors as-is', () => {
    expect(formatError('some error', 'fallback')).toBe('some error');
  });

  it('extracts message from Tauri AppError object', () => {
    expect(formatError({ Recording: 'Not recording' }, 'fallback')).toBe('Not recording');
  });

  it('extracts first string value from error object', () => {
    expect(formatError({ Transcription: 'Model not found' }, 'fallback')).toBe('Model not found');
  });

  it('returns fallback for null', () => {
    expect(formatError(null, 'fallback')).toBe('fallback');
  });

  it('returns fallback for undefined', () => {
    expect(formatError(undefined, 'fallback')).toBe('fallback');
  });

  it('returns fallback for empty object', () => {
    expect(formatError({}, 'fallback')).toBe('fallback');
  });

  it('returns fallback for numeric values', () => {
    expect(formatError(42, 'fallback')).toBe('fallback');
  });

  it('returns fallback for object with non-string values', () => {
    expect(formatError({ code: 500 }, 'fallback')).toBe('fallback');
  });
});

describe('invoke', () => {
  beforeEach(async () => {
    const mock = await getMockInvoke();
    mock.mockReset();
    mock.mockResolvedValue({});
  });

  it('calls tauri invoke with correct command and args', async () => {
    const mock = await getMockInvoke();
    mock.mockResolvedValue('result');
    const result = await invoke<string>('test_cmd', { key: 'val' });

    expect(result).toBe('result');
    expect(mock).toHaveBeenCalledWith('test_cmd', { key: 'val' });
  });

  it('sends frontend_log for non-log commands', async () => {
    const mock = await getMockInvoke();
    mock.mockResolvedValue('ok');
    await invoke('some_command');

    const logCalls = mock.mock.calls.filter((c: unknown[]) => c[0] === 'frontend_log');
    expect(logCalls.length).toBeGreaterThanOrEqual(1);
  });

  it('does not log recursively for frontend_log command', async () => {
    const mock = await getMockInvoke();
    mock.mockResolvedValue(undefined);
    await invoke('frontend_log', { level: 'info', message: 'test' });

    // Only the direct call, no recursive logging
    expect(mock).toHaveBeenCalledTimes(1);
    expect(mock.mock.calls[0][0]).toBe('frontend_log');
  });

  it('rethrows errors from invoke', async () => {
    const mock = await getMockInvoke();
    mock.mockImplementation((cmd: string) => {
      if (cmd === 'frontend_log') return Promise.resolve();
      return Promise.reject({ Recording: 'failed' });
    });

    await expect(invoke('start_recording')).rejects.toEqual({ Recording: 'failed' });
  });
});
