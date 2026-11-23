// Run against a built website with PAX_SITE_URL and Playwright available.
const assert = require('node:assert/strict');
const {test} = require('node:test');
const {chromium, devices} = require('playwright');

for (const [renderer, mobile] of [['gpu', false], ['gpu', true], ['cpu', true]]) {
    test(`Increment Cube counts each ${mobile ? 'tap' : 'click'} once (${renderer})`, async () => {
        const browser = await chromium.launch({channel: 'chrome'});
        try {
            const page = await browser.newPage(mobile ? devices['iPhone 13'] : {viewport: {width: 1440, height: 900}});
            await page.goto(new URL('/?pax_renderer=' + renderer, process.env.PAX_SITE_URL).href);
            await page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
            const prompt = page.locator('.native-leaf').filter({hasText: mobile ? /^TAP$/ : /^CLICK$/});
            await prompt.scrollIntoViewIfNeeded();
            const box = await prompt.boundingBox();
            assert(box && box.width > 0 && box.height > 0);
            const x = box.x + box.width / 2, y = box.y + box.height / 2;
            for (const expected of [1, 2]) {
                if (mobile) await page.touchscreen.tap(x, y);
                else await page.mouse.click(x, y);
                await page.waitForTimeout(1200); // Let the one-second roll land.
                const centered = await page.locator('.native-leaf').filter({hasText: new RegExp('^' + expected + '$')})
                    .evaluateAll((nodes, [x, y]) => nodes.some(node => {
                        for (let el = node; el; el = el.parentElement) {
                            if (Number(getComputedStyle(el).opacity) === 0) return false;
                        }
                        const r = node.getBoundingClientRect();
                        return r.width > 0 && r.height > 0
                            && Math.abs(r.left + r.width / 2 - x) < 3 && Math.abs(r.top + r.height / 2 - y) < 3;
                    }), [x, y]);
                assert(centered, 'the cube should settle on ' + expected + ' at its original center');
                assert.equal(await prompt.count(), 0);
            }
        } finally {
            await browser.close();
        }
    });
}
