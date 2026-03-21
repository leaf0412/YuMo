import { connect, demoPause, waitAndFind } from './helpers.js';

export async function run() {
  const browser = await connect();
  await browser.url('tauri://localhost/');
  await demoPause(1500);

  await browser.execute(() => {
    (window as any).__TAURI__.core.invoke('start_recording', { deviceId: null });
  });
  await demoPause(3000);

  await browser.execute(() => {
    (window as any).__TAURI__.core.invoke('stop_recording');
  });

  await waitAndFind(browser, '[data-testid="transcription-result"]', 15000);
  await demoPause(2000);
}
