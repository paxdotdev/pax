// Run against a built local site with PAX_SITE_URL and Playwright available.
const assert = require('node:assert/strict');
const {test} = require('node:test');
const {chromium} = require('playwright');

test('a mounted Markdown link changes theme in place in both directions', async () => {
    const browser = await chromium.launch({channel: 'chrome'});
    try {
        const page = await browser.newPage({viewport: {width: 1280, height: 850}});
        await page.goto(new URL('/?pax_renderer=cpu', process.env.PAX_SITE_URL).href);
        await page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
        const link = page.locator('.native-leaf [data-pax-markdown="true"] a[href$="/paxflix/"]').first();
        await link.scrollIntoViewIfNeeded();
        const anchor = await link.elementHandle();
        const expectColor = color => page.waitForFunction(([a, value]) =>
            a.isConnected && getComputedStyle(a).color === value, [anchor, color]);
        await expectColor('rgb(252, 249, 242)');
        for (const [mode, color] of [['light', 'rgb(23, 23, 21)'], ['dark', 'rgb(252, 249, 242)']]) {
            await page.getByRole('button', {name: 'Switch to ' + mode + ' mode', exact: true}).click();
            await expectColor(color);
        }
    } finally {
        await browser.close();
    }
});

test('hero replay preserves appearance; navigation, drawers and Paxflix share the preference', async () => {
    const browser = await chromium.launch({channel: 'chrome'});
    try {
        const page = await browser.newPage({viewport: {width: 1280, height: 850}});
        const url = new URL(process.env.PAX_SITE_URL);
        const go = path => page.goto(new URL(path, url).href);
        const mode = value => page.waitForFunction(v => document.documentElement.dataset.paxTheme === v, value);
        const wings = filenames => page.waitForFunction(names =>
            names.every(name => [...document.images].some(img => img.complete && img.naturalWidth > 0 &&
                img.src.endsWith('/assets/paxflix/' + name))), filenames);
        const button = name => page.getByRole('button', {name, exact: true});
        await go('/?pax_renderer=cpu');
        await mode('dark');
        await page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
        const word = text => page.locator('.native-leaf').filter({hasText: new RegExp('^' + text + '$')}).first();
        // Native normally advances to portable, not back to creative.
        await word('native').waitFor({state: 'visible', timeout: 30000});
        await button('Replay logo animation').click();
        assert.equal(await button('Switch to light mode').count(), 0);
        await page.mouse.move(8, 600);
        await page.mouse.wheel(0, 700);
        const nav = button('Switch to light mode');
        await nav.waitFor();
        assert.equal(await page.evaluate(() => localStorage.getItem('pax-site-theme')), 'dark');
        await nav.focus();
        await page.keyboard.press('Enter');
        await mode('light');
        await page.mouse.wheel(0, -1200);
        await word('creative').waitFor({state: 'visible', timeout: 5000});
        await button('VIEW SOURCE').first().click();
        await button('×').first().waitFor();
        assert.equal(await page.getByRole('button', {name: /Solarized/}).count(), 0);
        await button('×').first().click();
        // Move the page scroller itself; the live tablet has its own nested scrollers.
        const profile = page.getByText('P', {exact:true}).last();
        for (let i = 0; i < 12; ++i) {
            const box = await profile.boundingBox();
            if (box && box.y > 95 && box.y < 580) break;
            await page.mouse.move(160, 650);
            await page.mouse.wheel(0, Math.max(-600, Math.min(600, (box?.y ?? 1200) - 340)));
            await page.waitForTimeout(350);
        }
        await wings(['laptop-light.jpg', 'phone-light.jpg']);
        await profile.click({force:true});
        await page.getByText('Light mode', {exact:true}).last().click({force:true});
        await mode('dark');
        await wings(['macos.jpg', 'ios.jpg']);
        // The nav switch changes the shared preference without closing the profile.
        await button('Switch to light mode').click();
        await mode('light');
        await page.getByText('Light mode', {exact:true}).last().waitFor();
        await go('/ai/');
        await mode('light');
        await go('/paxflix/?pax_renderer=cpu');
        await mode('light');
        await page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
        // The visible text marks a canvas hit region, not a DOM button.
        await page.getByText('P', {exact:true}).last().click({force:true});
        await page.getByText('Light mode', {exact:true}).last().click({force:true});
        await mode('dark');
        await go('/?pax_renderer=cpu');
        await mode('dark');
    } finally { await browser.close(); }
});

test('the compact nav switch fits between the logo and Blog and responds to touch', async () => {
    const browser = await chromium.launch({channel: 'chrome'});
    try {
        const page = await browser.newPage({viewport: {width: 320, height: 720}, hasTouch: true});
        await page.goto(new URL('/?pax_renderer=cpu', process.env.PAX_SITE_URL).href);
        await page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
        await page.mouse.move(8, 550);
        await page.mouse.wheel(0, 900);
        const toggle = page.getByRole('button', {name: 'Switch to light mode', exact: true});
        await page.waitForFunction(() => [...document.querySelectorAll('.native-leaf')].some(el =>
            el.textContent === 'Blog' && Math.abs(el.getBoundingClientRect().y - 10) < 1));
        const box = await toggle.boundingBox();
        const blog = await page.locator('.native-leaf').filter({hasText: /^Blog$/}).first().boundingBox();
        const island = await toggle.evaluate(el => {
            const r = el.closest('.scroller-container').getBoundingClientRect();
            return {x: r.x, width: r.width};
        });
        assert.equal(island.x, 10);
        assert.equal(island.width, 300);
        assert(box && blog && box.x >= island.x + 14 + 52 && box.x + box.width <= blog.x);
        assert(box.height >= 44);
        await toggle.tap();
        await page.waitForFunction(() => document.documentElement.dataset.paxTheme === 'light');
    } finally { await browser.close(); }
});
