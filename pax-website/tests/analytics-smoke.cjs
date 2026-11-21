// Offline browser check against built artifacts; all requests (including Google)
// are intercepted. Run with Playwright available; see the website README.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const path = require('node:path');
const fs = require('node:fs/promises');
const { test } = require('node:test');
const id = 'G-MG3GLRDHYR';

test('consent, Pax actions and one pageview per host document', async () => {
    const site = path.resolve(process.env.PAX_TEST_SITE);
    const blog = path.resolve(__dirname, '../../pax-blog/public');
    const browser = await chromium.launch({channel: process.env.PAX_TEST_BROWSER || 'chrome'});
    try {
        const ctx = await browser.newContext({serviceWorkers: 'block'});
        const tagLoads = [];
        await ctx.route('**/*', async route => {
            const url = new URL(route.request().url());
            if (url.hostname === 'www.googletagmanager.com') {
                assert.equal(url.pathname + url.search, '/gtag/js?id=' + id);
                tagLoads.push(url.href);
                return route.fulfill({contentType: 'text/javascript', body: '/* inert test tag */'});
            }
            if (!['www.pax.dev', 'staging.pax.dev', 'localhost'].includes(url.hostname)) return route.abort();
            const inBlog = url.pathname.startsWith('/blog/');
            const root = inBlog ? blog : site;
            const file = path.join(root, decodeURIComponent(url.pathname.slice(inBlog ? 6 : 1)),
                url.pathname.endsWith('/') ? 'index.html' : '');
            if (!file.startsWith(root + path.sep)) return route.abort();
            try {
                await fs.access(file);
                await route.fulfill({path: file});
            } catch { await route.fulfill({status: 404, body: ''}); }
        });
        await ctx.addInitScript(() => {
            // Exercise the real callback without touching the workstation clipboard.
            Object.defineProperty(navigator, 'clipboard', {value: {writeText: async () => {
                if (window.failCopy) throw Error('Clipboard denied');
            }}});
            const open = window.open;
            window.open = function(url, ...args) {
                if (String(url).startsWith('codex://')) return null;
                return open.call(this, url, ...args);
            };
        });
        const page = await ctx.newPage();
        const origin = 'https://www.pax.dev';
        const active = () => page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
        const panel = page.locator('#pax-analytics-settings');
        const configs = () => page.evaluate(() => (window.dataLayer || []).filter(x => x[0] === 'config').length);
        await page.goto('https://localhost/ai/');
        assert.equal(await panel.count(), 0);
        await page.goto('https://staging.pax.dev/ai/');
        assert.equal(await panel.evaluate(el => el.open), true);
        assert.equal(tagLoads.length, 0);
        assert.equal(await configs(), 0);
        await panel.getByText('Allow analytics', {exact:true}).click();
        await page.waitForFunction(() => !!window.paxSiteAction);
        assert.deepEqual(await page.evaluate(() => {
            const config = window.dataLayer.find(x => x[0] === 'config');
            return [config[1], config[2].debug_mode, config[2].page_location];
        }), [id, true, 'https://staging.pax.dev/ai/']);
        await page.goto('https://staging.pax.dev/');
        await active();
        await page.getByRole('button', {name:'VIEW SOURCE', exact:true}).first().evaluate(el => el.click());
        await page.waitForFunction(() => window.dataLayer.some(x => x[0] === 'event' && x[1] === 'view_source'));
        assert.equal(tagLoads.length, 2);
        const stagingLoads = tagLoads.length;
        await page.goto(origin + '/?prompt=do-not-collect#ignored');
        await active();
        // Staging consent must never opt a visitor into production.
        assert.equal(await panel.evaluate(el => el.open), true);
        assert.equal(tagLoads.length, stagingLoads);
        assert.equal(await configs(), 0);
        await panel.getByText('Decline', {exact:true}).click();
        await page.reload();
        await active();
        assert.equal(await panel.evaluate(el => el.open), false);
        assert.equal(tagLoads.length, stagingLoads);
        await panel.locator('summary').click();
        await panel.getByText('Allow analytics', {exact:true}).click();
        await page.waitForFunction(() => !!window.paxSiteAction);
        assert.equal(await configs(), 1);
        const config = await page.evaluate(() => window.dataLayer.find(x=>x[0]==='config')[2]);
        assert.equal(config.page_location, origin + '/');
        assert.ok(!Object.hasOwn(config, 'debug_mode'));
        const button = name => page.getByRole('button', {name, exact:true}).first();
        await button('Copy to Clipboard').evaluate(el => el.click());
        const action = name => page.waitForFunction(n => window.dataLayer.some(x =>
            x[0] === 'event' && x[1] === n && x[2].placement === 'hero'), name);
        await action('copy_install');
        await button('With AI').evaluate(el => el.click());
        await button('Open in Codex').waitFor({state:'attached'});
        await page.evaluate(() => { window.failCopy = true; });
        await button('Copy to Clipboard').evaluate(el => el.click());
        await page.getByText('Copy was blocked. Select the text to copy it manually.', {exact:true}).first().waitFor({state:'attached'});
        await page.evaluate(() => { window.failCopy = false; });
        await button('Copy to Clipboard').evaluate(el => el.click());
        await action('copy_ai_prompt');
        await button('Open in Codex').evaluate(el => el.click());
        await action('open_in_codex');
        await button('VIEW SOURCE').evaluate(el => el.click());
        await action('view_source');
        await page.evaluate(() => {
            window.paxSiteAction('not_an_event', 'anything');
            window.paxSiteAction('copy_ai_prompt', 'codex://threads/new?prompt=private');
        });
        assert.deepEqual(await page.evaluate(() => window.dataLayer.filter(x => x[0] === 'event').map(x=>x[1])),
            ['copy_install', 'copy_ai_prompt', 'open_in_codex', 'view_source']);
        for (const url of ['/paxflix/', '/blog/']) {
            await page.goto(origin + url);
            await page.waitForFunction(() => !!window.paxSiteAction);
            assert.equal(await configs(), 1);
        }
        assert.equal(tagLoads.length - stagingLoads, 3);
        await page.evaluate(() => { document.cookie = '_ga=example; Path=/; Secure'; });
        await panel.locator('summary').click();
        await panel.getByText('Decline', {exact:true}).click();
        assert.ok(await page.evaluate(() => !window.paxSiteAction && !document.cookie.includes('_ga=')));
        await page.goto(origin + '/ai/');
        assert.equal(await configs(), 0);
        assert.equal(tagLoads.length - stagingLoads, 3);
    } finally { await browser.close(); }
});
