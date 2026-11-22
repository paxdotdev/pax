// Run against a built local site with PAX_SITE_URL and Playwright available.
const {test} = require('node:test');
const {chromium} = require('playwright');

test('only a full-screen hero source sheet suppresses the scroll navigation', async () => {
    const browser = await chromium.launch({channel: 'chrome'});
    try {
        const page = await browser.newPage({viewport: {width: 1280, height: 850}});
        await page.goto(new URL('/?pax_renderer=cpu', process.env.PAX_SITE_URL).href);
        await page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
        const heroTitle = page.locator('.native-leaf').filter({hasText: /^Build user interfaces$/});
        const waitForNav = visible => page.waitForFunction(expected => {
            const docked = [...document.querySelectorAll('.native-leaf')].some(el => {
                if (el.textContent !== 'Blog') return false;
                const r = el.getBoundingClientRect();
                return r.width > 0 && r.top >= 0 && r.bottom <= 80;
            });
            return docked === expected;
        }, visible, {timeout: 10000});

        await page.getByRole('button', {name: 'VIEW SOURCE', exact: true}).first().click();
        await heroTitle.waitFor();
        await waitForNav(false);
        // Scroll the preview, not the independently scrollable source code.
        await page.mouse.move(50, 750);
        await page.mouse.wheel(0, 950);
        await waitForNav(true);

        // The same open drawer becomes a viewport-sized sheet at compact widths.
        await page.setViewportSize({width: 390, height: 850});
        await page.waitForFunction(() => [...document.querySelectorAll('.native-leaf')].some(el =>
            el.textContent === 'Build user interfaces' && Math.abs(el.getBoundingClientRect().y - 24) < 2));
        await waitForNav(false);
        await page.getByRole('button', {name: '×', exact: true}).filter({visible: true}).first().click();
        await heroTitle.waitFor({state: 'detached'});
        await waitForNav(true);
    } finally {
        await browser.close();
    }
});
