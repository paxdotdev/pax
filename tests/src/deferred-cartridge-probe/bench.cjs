// Interleave modes in one browser page to reduce clock/load drift between runs.
const {chromium} = require('playwright');
const fs = require('fs');
const assert = require('assert/strict');
(async () => {
  const base = process.argv[2] || 'http://127.0.0.1:18750';
  const browser = await chromium.launch({headless: true,
    executablePath: process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});
  try {
    const page = await browser.newPage();
    await page.goto(base);
    await page.evaluate(base => {
      document.body.innerHTML = '';
      for (const mode of ['wasm', 'js', 'monolithic']) {
        const frame = document.createElement('iframe');
        frame.name = mode; frame.width = 1100; frame.height = 800;
        frame.style.cssText = 'position:absolute;left:-12000px;top:0';
        frame.src = `${base}/${mode === 'monolithic' ? mode : 'split'}/`;
        document.body.append(frame);
      }
    }, base);
    await page.waitForFunction(() => [...document.querySelectorAll('iframe')]
      .every(frame => frame.contentWindow?.probe));
    for (const mode of ['wasm', 'js', 'monolithic']) {
      const frame = page.frame({name: mode});
      await frame.waitForFunction(() => window.probe);
      if (mode !== 'monolithic') await frame.evaluate(async mode => { await probe.prefetch({bridge: mode}); probe.activate(); }, mode);
      await frame.waitForFunction(() => probe.raw.probe_stats(0) === 1);
      await frame.evaluate(() => probe.raw.probe_write(2, 7));
    }
    const result = await page.evaluate(() => {
      const modes = ['wasm', 'js', 'monolithic'];
      const hosts = Object.fromEntries(modes.map(mode => [mode, document.querySelector(`iframe[name="${mode}"]`).contentWindow.probe.raw]));
      const rows = [];
      for (const r of Object.values(hosts)) { r.probe_bench(200000, 0, 17); r.probe_bench(200000, 1, 17); r.probe_event_bench(1000); }
      for (let trial = 0; trial < 12; trial++) {
        for (let j = 0; j < modes.length; j++) {
          const mode = modes[(j + trial) % modes.length], r = hosts[mode];
          const row = {trial, mode};
          for (const path of trial % 2 ? [1, 0] : [0, 1]) {
            const start = performance.now();
            row[`checksum${path}`] = r.probe_bench(2000000, path, trial + 17);
            row[`math${path}Ms`] = performance.now() - start;
          }
          r.probe_write(1, 0);
          const start = performance.now();
          row.value = r.probe_event_bench(20000);
          row.eventsMs = performance.now() - start;
          rows.push(row);
        }
      }
      return rows;
    });
    for (const row of result) { assert.equal(row.checksum0, row.checksum1); assert.equal(row.value, 140000); }
    const data = {browser: browser.version(), arithmeticIterations: 2000000, eventIterations: 20000, rows: result};
    fs.writeFileSync(process.argv[3] || '/tmp/pax-probe-bench.json', JSON.stringify(data, null, 2));
    console.log('Interleaved benchmark passed:', result.length, 'samples');
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
