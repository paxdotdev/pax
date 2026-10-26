// Run with Playwright available through NODE_PATH. All measurements execute
// inside the page; browser-control round trips are excluded from timed loops.
const {chromium} = require('playwright');
const fs = require('fs');
const assert = require('assert/strict');
const base = process.argv[2] || 'http://127.0.0.1:18750';
const output = process.argv[3] || '/tmp/pax-probe-check.json';

(async () => {
  const browser = await chromium.launch({headless: true,
    executablePath: process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});
  const results = [];
  try {
    for (const mode of (process.env.PROBE_MODES || 'wasm,js,monolithic').split(',')) {
      const context = await browser.newContext({viewport: {width: 1100, height: 800}});
      const page = await context.newPage();
      const errors = [], childRequests = [];
      page.on('pageerror', e => errors.push(String(e)));
      page.on('request', r => { if (r.url().includes('child.wasm')) childRequests.push(r.url()); });
      await page.goto(`${base}/${mode === 'monolithic' ? mode : 'split'}/`);
      await page.waitForFunction(() => window.probe, null, {timeout: 30000});
      assert.equal(childRequests.length, 0);
      if (mode !== 'monolithic') {
        assert.equal(await page.evaluate(() => probe.raw.probe_stats(0)), 0);
        assert.equal(await page.evaluate(() => probe.raw.probe_register(1, 99, 0)), 0);
        assert.equal(await page.evaluate(() => probe.raw.probe_register(1, 2, 1)), 0);
        await page.evaluate(mode => probe.prefetch({bridge: mode}), mode);
        assert.equal(await page.evaluate(() => probe.raw.probe_stats(0)), 0);
        await page.evaluate(() => probe.activate());
      }
      const child = label => page.getByRole('button', {name: label, exact: true});
      await child('Child adds 1').waitFor();
      await child('Child adds 1').click();
      await page.waitForFunction(() => probe.raw.probe_read(1) === 1);
      await page.getByText('Child sees parent: 1', {exact: true}).waitFor();
      await child('Change step').click();
      await child('Child adds 7').click();
      await page.waitForFunction(() => probe.raw.probe_read(1) === 8);
      await child('Reset parent').click();
      await page.getByText('Child sees parent: 0', {exact: true}).waitFor();
      await page.getByText('This slot belongs to the parent.', {exact: true}).waitFor();
      await child('Change color').click();
      const oldToken = await page.evaluate(() => probe.raw.probe_stats(2));
      await child('Toggle child').click();
      await page.waitForFunction(() => probe.raw.probe_stats(1) === 1);
      assert.equal(await page.evaluate(t => probe.raw.probe_stale_click(t), oldToken), 0);
      assert.equal(await page.evaluate(() => probe.raw.probe_read(1)), 0);
      await child('Toggle child').click();
      await child('Child adds 7').waitFor();
      await page.waitForFunction(() => probe.raw.probe_stats(0) === 2);
      assert.notEqual(await page.evaluate(() => probe.raw.probe_stats(2)), oldToken);
      assert.equal(await page.evaluate(() => probe.raw.probe_stats(3)), 0);
      if (mode !== 'monolithic') {
        await page.evaluate(() => { probe.activate(); return probe.prefetch(); });
        assert.equal(childRequests.length, 1);
        assert.equal(await page.evaluate(() => probe.raw.probe_stats(0)), 2);
      }
      await page.screenshot({path: output.replace('.json', `-${mode}.png`)});
      const timings = await page.evaluate(() => {
        const r = probe.raw, arithmetic = [], events = [];
        r.probe_bench(100000, 0, 17); r.probe_bench(100000, 1, 17);
        r.probe_event_bench(1000);
        for (let i = 0; i < 9; i++) {
          const order = i % 2 ? [1, 0] : [0, 1], row = {};
          for (const mode of order) {
            const start = performance.now();
            row[`checksum${mode}`] = r.probe_bench(2000000, mode, 17 + i);
            row[`mode${mode}Ms`] = performance.now() - start;
          }
          arithmetic.push(row);
          r.probe_write(1, 0);
          const start = performance.now();
          const value = r.probe_event_bench(10000);
          events.push({ms: performance.now() - start, value});
        }
        return {arithmetic, events, loading: probe.timings, bridgeCalls: probe.bridgeCalls,
          hostMemory: r.memory.buffer.byteLength, childMemory: probe.module?.exports.memory.buffer.byteLength};
      });
      for (const x of timings.arithmetic) assert.equal(x.checksum0, x.checksum1);
      for (const x of timings.events) assert.equal(x.value, 70000);
      if (mode === 'wasm') assert.equal(timings.bridgeCalls, 0);
      if (mode === 'js') assert.ok(timings.bridgeCalls >= 273000);
      assert.deepEqual(errors, []);
      results.push({mode, browser: browser.version(), checks: 'passed', childRequests: childRequests.length, ...timings});
      fs.writeFileSync(output, JSON.stringify(results, null, 2));
      console.log(mode, 'passed', JSON.stringify(timings.loading));
      await context.close();
    }
    for (const failure of ['missing', 'corrupt', 'abi', 'abort']) {
      const context = await browser.newContext();
      const page = await context.newPage();
      await page.goto(`${base}/split/`);
      await page.waitForFunction(() => window.probe);
      if (failure === 'corrupt') await page.route('**/bad.wasm', r => r.fulfill({body: 'not wasm'}));
      const message = await page.evaluate(async failure => {
        try {
          const controller = new AbortController();
          if (failure === 'abort') controller.abort();
          await probe.prefetch({url: failure === 'missing' ? './missing.wasm' : failure === 'corrupt' ? './bad.wasm' : './child.wasm',
            expectedAbi: failure === 'abi' ? 99 : 2, signal: controller.signal});
          return 'unexpected success';
        } catch (e) { return String(e); }
      }, failure);
      assert.notEqual(message, 'unexpected success');
      assert.equal(await page.evaluate(() => probe.raw.probe_stats(0)), 0);
      await page.evaluate(async () => { await probe.prefetch(); probe.activate(); });
      await page.getByRole('button', {name: 'Child adds 1', exact: true}).waitFor();
      results.push({failure, message, retry: 'passed'});
      console.log(failure, 'rejected; retry passed');
      await context.close();
    }
    fs.writeFileSync(output, JSON.stringify(results, null, 2));
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
