import { connect, demoPause } from './helpers.js';

export async function run() {
  const browser = await connect();
  await browser.url('tauri://localhost/statistics');
  await demoPause(3000);

  await browser.execute(() => window.scrollBy(0, 400));
  await demoPause(2000);
}
