import { vi } from 'vitest';

// Mock Tauri core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));

// Helper to mock specific invoke calls
export async function mockInvoke(command: string, returnValue: unknown) {
  const mod = await import('@tauri-apps/api/core');
  const invoke = mod.invoke as ReturnType<typeof vi.fn>;
  invoke.mockImplementation((cmd: string) => {
    if (cmd === command) return Promise.resolve(returnValue);
    return Promise.resolve({});
  });
}

export async function mockInvokeMultiple(mocks: Record<string, unknown>) {
  const mod = await import('@tauri-apps/api/core');
  const invoke = mod.invoke as ReturnType<typeof vi.fn>;
  invoke.mockImplementation((cmd: string) => {
    if (cmd in mocks) return Promise.resolve(mocks[cmd]);
    return Promise.resolve({});
  });
}
