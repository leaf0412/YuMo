import { remote, type Browser } from 'webdriverio';
import * as fs from 'fs';
import * as path from 'path';

const OUTPUT_DIR = path.resolve(import.meta.dirname, '..', 'output');

export interface ScenarioTimestamp {
  name: string;
  start: number;
  end: number;
}

/**
 * 连接到 tauri-driver（已在运行的语墨实例）
 */
export async function connect(): Promise<Browser> {
  const browser = await remote({
    hostname: 'localhost',
    port: 4444,
    capabilities: {
      'tauri:options': {
        application: '',
      },
    },
  });
  return browser;
}

/**
 * 演示停顿 — 让观众看清当前操作结果
 */
export async function demoPause(ms: number = 1500): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * 等待元素出现并返回
 */
export async function waitAndFind(
  browser: Browser,
  selector: string,
  timeout: number = 5000
): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector);
  await el.waitForExist({ timeout });
  return el;
}

/**
 * 点击元素 + 演示停顿
 */
export async function clickAndPause(
  browser: Browser,
  selector: string,
  pauseMs: number = 1000
): Promise<void> {
  const el = await waitAndFind(browser, selector);
  await el.click();
  await demoPause(pauseMs);
}

/**
 * 输入文本 + 演示停顿
 */
export async function typeAndPause(
  browser: Browser,
  selector: string,
  text: string,
  pauseMs: number = 1000
): Promise<void> {
  const el = await waitAndFind(browser, selector);
  await el.setValue(text);
  await demoPause(pauseMs);
}

/**
 * 保存时间戳文件（用于 postprocess.sh 分割 GIF）
 */
export function saveTimestamps(timestamps: ScenarioTimestamp[]): void {
  const filePath = path.join(OUTPUT_DIR, 'timestamps.json');
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  fs.writeFileSync(filePath, JSON.stringify(timestamps, null, 2));
  console.log(`[scenario] timestamps saved to ${filePath}`);
}
