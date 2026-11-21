// Run against a built local site with PAX_SITE_URL and Playwright available.
const assert = require('node:assert/strict');
const {test} = require('node:test');
const {chromium} = require('playwright');

test('logo, source viewer, document routes and Paxflix share one preference', async () => {
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
        // The second, docked logo is offscreen at the top.
        const logos = button('Switch to light mode');
        const hero = logos.filter({visible: true});
        for (const logo of await hero.all()) {
            const box = await logo.boundingBox();
            if (box && box.x >= 0 && box.y >= 0 && box.y + box.height <= 850) {
                await logo.click();
                break;
            }
        }
        await mode('light');
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
        // The docked logo switches back in place, with the profile still open.
        await button('Switch to light mode').first().click();
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
