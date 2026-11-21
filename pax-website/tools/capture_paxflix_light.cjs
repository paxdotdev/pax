// Run against a local website build. The frames and the web route's back link
// are not part of the captures; movie artwork keeps its original colors.
const { chromium } = require('playwright');
const path = require('node:path');

(async () => {
    const browser = await chromium.launch({channel: 'chrome'});
    try {
        for (const [device, width, height, scale, scroll] of [
            ['laptop', 1280, 800, 1, 820],
            ['tablet', 1024, 768, 1, 0],
            ['phone', 374, 786, 2, 0],
        ]) {
            const page = await browser.newPage({viewport: {width, height: height + 52}, deviceScaleFactor: scale});
            await page.addInitScript(() => localStorage.setItem('pax-site-theme', 'light'));
            await page.goto(new URL('/paxflix/', process.env.PAX_SITE_URL).href);
            await page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
            await page.waitForFunction(() => document.documentElement.dataset.paxTheme === 'light');
            await page.evaluate(() => document.fonts.ready);
            if (scroll) {
                await page.mouse.move(width / 2, height / 2);
                await page.mouse.wheel(0, scroll);
            }
            // Let the visible rows enter and the decoded posters reach the compositor.
            await page.waitForTimeout(2000);
            await page.screenshot({path: path.join(__dirname, '../assets/paxflix', device + '-light.jpg'),
                clip: {x: 0, y: 52, width, height}, quality: 87});
            await page.close();
        }
    } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
