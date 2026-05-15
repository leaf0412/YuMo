/**
 * Bridge entry point — Tauri runtime.
 *
 * Usage:
 *   const b = await getBridge();   // async, safe to call multiple times
 *   const b = getInitializedBridge(); // sync, only after getBridge() resolved
 */
import type { Bridge } from './types';

let _bridge: Bridge | null = null;

/**
 * Resolve and cache the Tauri Bridge implementation.
 * Safe to call multiple times; initialization only happens once.
 */
export async function getBridge(): Promise<Bridge> {
  if (_bridge) return _bridge;
  const { tauriBridge } = await import('./tauri');
  _bridge = tauriBridge;
  return _bridge;
}

/**
 * Synchronous accessor — returns the bridge if it has been initialized,
 * throws otherwise. Call `getBridge()` at least once on app startup first.
 */
export function getInitializedBridge(): Bridge {
  if (!_bridge) {
    throw new Error('Bridge not initialized. Call getBridge() first.');
  }
  return _bridge;
}

export type { Bridge } from './types';
export * from './types';
