import { useEffect } from 'react';
import { listen } from '../lib/events';

export function useTauriEvent<T>(event: string, handler: (payload: T) => void) {
  useEffect(() => {
    const unlisten = listen<T>(event, (e) => handler(e.payload));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [event, handler]);
}
