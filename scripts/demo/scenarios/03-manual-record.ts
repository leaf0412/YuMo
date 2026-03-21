import { connect, clickAndPause, demoPause, waitAndFind } from './helpers.js';

export async function run() {
  const browser = await connect();
  await browser.url('tauri://localhost/');
  await demoPause(1500);

  await clickAndPause(browser, '[data-testid="record-btn"]', 3500);
  await clickAndPause(browser, '[data-testid="stop-btn"]', 500);

  await waitAndFind(browser, '[data-testid="transcription-result"]', 15000);
  await demoPause(2000);
}
