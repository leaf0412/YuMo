/**
 * Electron stub — placeholder for future Electron shell support.
 *
 * All methods throw until an actual Electron IPC implementation is wired in.
 * Using a Proxy means no boilerplate per-method, and any new Bridge method
 * added to the interface is automatically handled.
 */
import type { Bridge } from './types';

export const electronBridge: Bridge = new Proxy({} as Bridge, {
  get(_target, prop: string) {
    return (..._args: unknown[]) => {
      return Promise.reject(
        new Error(`Electron bridge not implemented: ${prop}`),
      );
    };
  },
});
