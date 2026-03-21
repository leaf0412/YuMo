import { connect, clickAndPause, demoPause } from './helpers.js';

export async function run() {
  const browser = await connect();
  await browser.url('tauri://localhost/settings');
  await demoPause(1500);

  await clickAndPause(browser, '[data-testid="settings-recording"]', 1500);
  await clickAndPause(browser, '[data-testid="settings-transcription"]', 1500);
  await clickAndPause(browser, '[data-testid="settings-general"]', 1500);

  await clickAndPause(browser, '[data-testid="language-select"]', 500);
  await clickAndPause(browser, '[data-testid="language-en"]', 2000);
  await clickAndPause(browser, '[data-testid="language-select"]', 500);
  await clickAndPause(browser, '[data-testid="language-zh"]', 1500);
}
