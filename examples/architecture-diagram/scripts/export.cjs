#!/usr/bin/env node
// Capture the real Pax app, including native text and GPU canvases.
// Dependency: Playwright and a Chromium installation (or PAX_EXPORT_BROWSER).
const { chromium } = require('playwright');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');

async function main() {
  const [url, directory = '/tmp/pax-architecture-exports'] = process.argv.slice(2);
  if (!url) throw new Error('Usage: node scripts/export.cjs <running-app-url> [output-directory]');
  await fs.mkdir(directory, { recursive: true });
  const browser = await chromium.launch({
    executablePath: process.env.PAX_EXPORT_BROWSER || undefined,
    args: ['--enable-unsafe-webgpu', '--use-angle=metal'],
  });
  try {
    for (const density of [1, 2]) {
      const page = await browser.newPage({ viewport: { width: 1560, height: 1144 }, deviceScaleFactor: density });
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.goto(url, { waitUntil: 'networkidle', timeout: 120000 });
      await page.getByRole('button', { name: 'Sheet view', exact: true }).click();
      await page.getByRole('button', { name: 'Sheet view', exact: true }).waitFor({ state: 'hidden' });
      await page.evaluate(() => document.fonts.ready);
      // A still sheet must settle to repeated identical composed frames. This
      // checks readiness without assuming an arbitrary asset-loading delay.
      let previous = '', identical = 0, bytes;
      for (let attempt = 0; attempt < 20 && identical < 2; attempt++) {
        await page.waitForTimeout(500);
        bytes = await page.screenshot({ animations: 'disabled' });
        const hash = crypto.createHash('sha256').update(bytes).digest('hex');
        identical = hash === previous ? identical + 1 : 0;
        previous = hash;
      }
      if (identical < 2) throw new Error('Sheet did not settle to a deterministic still');
      if (errors.length) throw new Error(errors.join('\n'));
      const filename = path.join(directory, `pax-architecture-${1560*density}.png`);
      await fs.writeFile(filename, bytes);
      console.log(`${filename}  sha256:${previous}`);
      await page.close();
    }
  } finally { await browser.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
