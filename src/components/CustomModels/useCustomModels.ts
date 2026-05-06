import { useCallback, useEffect, useState } from 'react';
import type {
  CustomModelStatus,
  CustomModelScanResult,
  CustomDepsCheckResult,
} from './types';

type ElectronAPI = {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
};

function getElectronAPI(): ElectronAPI {
  const api = (window as unknown as { electronAPI?: ElectronAPI }).electronAPI;
  if (!api) {
    throw new Error('window.electronAPI is unavailable — custom models require the Electron host');
  }
  return api;
}

/**
 * Aggregates list-custom-models + custom-check-deps + custom-is-downloaded
 * into a CustomModelStatus[] for the settings section.
 *
 * Each scan-error becomes an `invalid` entry; each ok spec is then probed
 * for deps and download state in order. Errors during the per-spec probes
 * are surfaced as failures (no silent fallback) — the hook re-throws via
 * its containing promise so callers can react.
 */
export function useCustomModels() {
  const [items, setItems] = useState<CustomModelStatus[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const api = getElectronAPI();
      const scan = (await api.invoke('list-custom-models')) as CustomModelScanResult;
      const next: CustomModelStatus[] = [];

      for (const e of scan.errors) {
        next.push({ kind: 'invalid', error: e.error, sourcePath: e.path });
      }

      for (const spec of scan.ok) {
        const deps = (await api.invoke(
          'custom-check-deps',
          spec.sourcePath,
        )) as CustomDepsCheckResult;
        if (!deps.allInstalled) {
          next.push({ kind: 'depsMissing', spec, missing: deps.missing });
          continue;
        }
        const downloaded = (await api.invoke(
          'custom-is-downloaded',
          spec.id,
        )) as boolean;
        next.push(
          downloaded ? { kind: 'ready', spec } : { kind: 'notDownloaded', spec },
        );
      }

      setItems(next);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { items, loading, refresh };
}
