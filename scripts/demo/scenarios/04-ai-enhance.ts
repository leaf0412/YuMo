import { connect, clickAndPause, typeAndPause, demoPause } from './helpers.js';

export async function run() {
  const browser = await connect();
  await browser.url('tauri://localhost/enhance');
  await demoPause(1500);

  await clickAndPause(browser, '[data-testid="llm-provider-select"]', 500);
  await clickAndPause(browser, '[data-testid="llm-provider-openai"]', 1000);
  await typeAndPause(browser, '[data-testid="api-key-input"]', 'sk-demo-xxxx', 1000);

  await clickAndPause(browser, '[data-testid="create-prompt-btn"]', 500);
  await typeAndPause(browser, '[data-testid="prompt-name-input"]', '修正语法', 500);
  await typeAndPause(browser, '[data-testid="prompt-template-input"]', '请修正以下文本的语法错误：\n{{text}}', 1000);
  await clickAndPause(browser, '[data-testid="save-prompt-btn"]', 1000);

  await clickAndPause(browser, '[data-testid="enhance-toggle"]', 1500);
}
