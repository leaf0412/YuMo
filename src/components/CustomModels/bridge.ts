/**
 * Tauri bridge for the CustomModels feature.
 *
 * Components in this folder call `getCustomBridge().invoke('custom-...', ...positional)`.
 * This module maps the kebab channel name + positional args to:
 *   - Tauri command name (snake_case)
 *   - Named-args map expected by `@tauri-apps/api/core::invoke(name, args_obj)`
 *
 * Channel name remains the source of truth. The mapping table below records
 * the Tauri command name + the arg-key names (in positional order) for each
 * channel. Adding a new channel requires entering it here once.
 */

export interface Bridge {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
}

interface ChannelConfig {
  /** Tauri command name (snake_case, matches `#[tauri::command]` fn name). */
  tauri: string;
  /** Names of the args in positional order; mapped to a keyed object for Tauri invoke. */
  argNames: string[];
}

const CHANNEL_MAP: Record<string, ChannelConfig> = {
  'list-custom-models': { tauri: 'list_custom_models', argNames: [] },
  'custom-check-deps': { tauri: 'custom_check_deps', argNames: ['specPath'] },
  'custom-install-deps': { tauri: 'custom_install_deps', argNames: ['specPath'] },
  'custom-download': { tauri: 'custom_download', argNames: ['specPath'] },
  'custom-open-dir': { tauri: 'custom_open_dir', argNames: [] },
  'custom-import-example': { tauri: 'custom_import_example', argNames: ['fileName'] },
  'custom-remove': { tauri: 'custom_remove', argNames: ['specPath'] },
  'custom-is-downloaded': { tauri: 'custom_is_downloaded', argNames: ['id'] },
  'custom-is-trusted': { tauri: 'custom_is_trusted', argNames: ['id'] },
  'custom-set-trusted': { tauri: 'custom_set_trusted', argNames: ['id'] },
};

async function tauriInvoke(channel: string, ...args: unknown[]): Promise<unknown> {
  const cfg = CHANNEL_MAP[channel];
  if (!cfg) {
    throw new Error(`Unknown custom-models IPC channel: ${channel}`);
  }
  const { invoke } = await import('@tauri-apps/api/core');
  const argsObj: Record<string, unknown> = {};
  cfg.argNames.forEach((name, i) => {
    argsObj[name] = args[i];
  });
  return invoke(cfg.tauri, argsObj);
}

const tauriBridge: Bridge = { invoke: tauriInvoke };

export function getCustomBridge(): Bridge {
  return tauriBridge;
}
