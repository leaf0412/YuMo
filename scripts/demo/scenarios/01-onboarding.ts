import { connect, clickAndPause, demoPause } from './helpers.js';

export async function run() {
  const browser = await connect();
  await browser.url('tauri://localhost/onboarding');
  await demoPause(2000);

  await clickAndPause(browser, '[data-testid="onboarding-start"]', 1500);
  await demoPause(2000);

  await clickAndPause(browser, '[data-testid="model-whisper-tiny"]', 1500);

  await clickAndPause(browser, '[data-testid="hotkey-record-btn"]', 1000);
  await browser.keys(['Control', 'Shift', 'Space']);
  await demoPause(1000);
  await clickAndPause(browser, '[data-testid="hotkey-confirm-btn"]', 1500);

  await clickAndPause(browser, '[data-testid="onboarding-done"]', 2000);
}
