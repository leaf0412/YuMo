/**
 * Cross-window broadcast via BroadcastChannel API.
 * All Tauri windows on the same origin can communicate instantly.
 */

const CHANNEL_NAME = 'voiceink';

let channel: BroadcastChannel | null = null;

function getChannel(): BroadcastChannel {
  if (!channel) {
    channel = new BroadcastChannel(CHANNEL_NAME);
  }
  return channel;
}

/** Broadcast a message to all windows (including self). */
export function broadcast(type: string, payload?: unknown) {
  getChannel().postMessage({ type, payload });
}

/** Listen for broadcast messages. Returns cleanup function. */
export function onBroadcast(
  type: string,
  handler: (payload: unknown) => void,
): () => void {
  const ch = getChannel();
  const listener = (e: MessageEvent) => {
    if (e.data?.type === type) {
      handler(e.data.payload);
    }
  };
  ch.addEventListener('message', listener);
  return () => ch.removeEventListener('message', listener);
}
