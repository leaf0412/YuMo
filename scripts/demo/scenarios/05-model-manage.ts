import { connect, clickAndPause, demoPause } from './helpers.js';

export async function run() {
  const browser = await connect();
  await browser.url('tauri://localhost/models');
  await demoPause(2000);

  await browser.execute(() => {
    document.querySelector('[data-testid="model-list"]')?.scrollBy(0, 300);
  });
  await demoPause(1500);

  await clickAndPause(browser, '[data-testid="cloud-models-tab"]', 1500);
  await clickAndPause(browser, '[data-testid="local-models-tab"]', 1000);
  await clickAndPause(browser, '[data-testid="model-whisper-base"]', 1500);
}
