import { type ScenarioTimestamp, saveTimestamps, demoPause } from './helpers.js';

const scenarios = [
  { name: '01-onboarding', module: './01-onboarding.js' },
  { name: '02-recording', module: './02-recording.js' },
  { name: '03-manual-record', module: './03-manual-record.js' },
  { name: '04-ai-enhance', module: './04-ai-enhance.js' },
  { name: '05-model-manage', module: './05-model-manage.js' },
  { name: '06-statistics', module: './06-statistics.js' },
  { name: '07-settings', module: './07-settings.js' },
];

async function main() {
  const timestamps: ScenarioTimestamp[] = [];
  let elapsed = 0;

  for (const scenario of scenarios) {
    console.log(`\n====== Running ${scenario.name} ======`);
    const startTime = Date.now();
    const start = elapsed;

    try {
      const mod = await import(scenario.module);
      await mod.run();
    } catch (err) {
      console.error(`[run-all] ${scenario.name} FAILED:`, err);
    }

    const duration = (Date.now() - startTime) / 1000;
    elapsed += duration;

    timestamps.push({
      name: scenario.name.replace(/^\d+-/, ''),
      start,
      end: elapsed,
    });

    await demoPause(2000);
    elapsed += 2;
  }

  saveTimestamps(timestamps);
  console.log(`\n[run-all] complete, total=${elapsed.toFixed(1)}s`);
}

main().catch(console.error);
