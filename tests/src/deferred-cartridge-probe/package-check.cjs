const {chromium} = require('playwright');
const fs = require('fs');
const assert = require('assert/strict');
const base = process.argv[2] || 'http://127.0.0.1:18751';
const output = process.argv[3] || '/tmp/pax-package-check.json';
(async () => {
  const browser = await chromium.launch({headless: true,
    executablePath: process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});
  const results = [];
  try {
    const page = await browser.newPage({viewport: {width: 1100, height: 800}});
    const errors = [], requests = [];
    page.on('pageerror', e => errors.push(String(e)));
    page.on('request', r => requests.push(r.url()));
    await page.goto(`${base}/alternate/`);
    await page.waitForFunction(() => window.probe);
    assert.equal(requests.filter(url => url.endsWith('/child.wasm')).length, 0);
    await page.evaluate(() => probe.prefetch());
    assert.equal(await page.getByRole('button', {name: 'Bundle v2 adds 1', exact: true}).count(), 0);
    await page.evaluate(() => probe.activate());
    await page.getByText('Bundle revision: 2026 · Clicks: 0', {exact: true}).waitFor();
    await page.getByRole('button', {name: 'Bundle v2 adds 1', exact: true}).click();
    await page.getByText('Bundle revision: 2026 · Clicks: 1', {exact: true}).waitFor();
    assert.equal(await page.evaluate(() => probe.raw.probe_read(1)), 1);
    await page.getByText('This slot belongs to the parent.', {exact: true}).waitFor();
    // Scalar handles are revoked along with module instance tokens.
    const handles = await page.evaluate(() => [probe.raw.probe_bind(1, 0), probe.raw.probe_bind(1, 3)]);
    assert.ok(handles.every(Boolean));
    await page.getByRole('button', {name: 'Toggle child', exact: true}).click();
    await page.waitForFunction(() => probe.raw.probe_stats(1) === 1);
    assert.equal(await page.evaluate(() => probe.raw.probe_bind(1, 0)), 0);
    for (const handle of handles) assert.equal(await page.evaluate(h => probe.raw.probe_write(h, 999), handle), 0);
    await page.getByRole('button', {name: 'Toggle child', exact: true}).click();
    await page.getByText('Bundle revision: 2026 · Clicks: 0', {exact: true}).waitFor();
    await page.getByText('Child sees parent: 1', {exact: true}).waitFor();
    await page.screenshot({path: output.replace('.json', '.png')});
    assert.deepEqual(errors, []);
    assert.equal(requests.filter(url => url.endsWith('.paxm') || url.endsWith('/descriptor.json')).length, 0);
    results.push({alternate: 'template, new local property, renamed handler, slot, remount passed', separateMetadataRequests: 0});
    await page.close();
    for (const failure of ['schema', 'route', 'definition']) {
      const page = await browser.newPage();
      await page.goto(`${base}/split/`);
      await page.waitForFunction(() => window.probe);
      const message = await page.evaluate(async failure => {
        try { await probe.prefetch({url: `./bad-${failure}.wasm`}); return 'unexpected success'; }
        catch (error) { return String(error); }
      }, failure);
      assert.notEqual(message, 'unexpected success');
      assert.equal(await page.evaluate(() => probe.raw.probe_stats(0)), 0);
      await page.evaluate(async () => { await probe.prefetch(); probe.activate(); });
      await page.getByRole('button', {name: 'Child adds 1', exact: true}).waitFor();
      results.push({failure, message, retry: 'passed'});
      await page.close();
    }
    fs.writeFileSync(output, JSON.stringify(results, null, 2));
    console.log(JSON.stringify(results));
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
