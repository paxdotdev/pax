// Run against a built GPU website with PAX_SITE_URL and Playwright available.
const assert = require('node:assert/strict');
const {test} = require('node:test');
const {chromium} = require('playwright');

// Count contiguous subtractive hues along a single edge, ignoring blended
// transition pixels. Seeing all three somewhere isn't enough: CMYCMY is two cycles.
async function readGlint(encoded) {
    const bytes = Uint8Array.from(atob(encoded), c => c.charCodeAt(0));
    const image = await createImageBitmap(new Blob([bytes], {type: 'image/png'}));
    const ctx = new OffscreenCanvas(image.width, image.height).getContext('2d');
    ctx.drawImage(image, 0, 0);
    const {data} = ctx.getImageData(0, 0, image.width, image.height);
    let bestGap = 0, centerHue = '', oneCycle = false;
    for (let row = 0; row < image.height; row++) {
        const bands = [];
        for (let col = 0; col < image.width; col++) {
            const k = (row * image.width + col) * 4;
            for (let c = 0; c < 3; c++) {
                const others = Math.min(data[k + (c + 1) % 3], data[k + (c + 2) % 3]);
                if (others - data[k + c] <= 18) continue;
                if (col === Math.floor(image.width / 2) && others - data[k + c] > bestGap) {
                    bestGap = others - data[k + c];
                    centerHue = 'CMY'[c];
                }
                const last = bands.at(-1);
                if (last?.channel === c) last.pixels++;
                else bands.push({channel: c, pixels: 1});
                break;
            }
        }
        const sequence = bands.filter(b => b.pixels >= 2).map(b => 'CMY'[b.channel]).join('');
        // A rolling phase can split one pigment between the two ends.
        if (sequence.length >= 3 && sequence.length <= 4 &&
            ('CMYCMY'.includes(sequence) || 'YMCYMC'.includes(sequence))) oneCycle = true;
    }
    return {centerHue, oneCycle};
}

for (const width of [1280, 390]) {
    test(`one CMY cycle rolls through the moving pointer glint at ${width}px`, async () => {
        const browser = await chromium.launch({channel: 'chrome'});
        try {
            // Hold the water under the rim still so its moving highlights cannot
            // cross the detector's color threshold while sampling a parked light.
            const page = await browser.newPage({
                viewport: {width, height: 950},
                deviceScaleFactor: 1,
                reducedMotion: 'reduce',
            });
            await page.goto(new URL('/?pax_renderer=gpu', process.env.PAX_SITE_URL).href);
            await page.waitForSelector('[data-pax-loading="active"]', {timeout: 120000});
            const label = page.locator('.native-leaf').filter({hasText: /^Get Started$/}).filter({visible: true}).first();
            const box = await label.boundingBox();
            assert(box);
            const x = Math.round(box.x + box.width / 2), y = Math.round(box.y + box.height / 2);
            // Let the separate introductory strip sweep pass, then sample in
            // glint-relative coordinates: simply translating fixed stripes fails.
            await page.waitForTimeout(2200);
            const readAt = async dx => {
                await page.mouse.move(x + dx, y);
                await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
                const png = await page.screenshot({clip: {x: x + dx - 38, y: y - 37, width: 76, height: 20}});
                return page.evaluate(readGlint, png.toString('base64'));
            };
            const hues = new Set();
            let fullCycle = false, middleHue;
            for (const dx of [-50, -25, 0, 25, 50]) {
                const {centerHue, oneCycle} = await readAt(dx);
                if (centerHue) hues.add(centerHue);
                if (dx === 0) middleHue = centerHue;
                fullCycle ||= oneCycle;
            }
            assert.deepEqual([...hues].sort(), ['C', 'M', 'Y'], 'the traveling center should change hue');
            assert(fullCycle, 'one visible segment should still span one CMY triplet');
            const returned = await readAt(0);
            assert.equal(returned.centerHue, middleHue, 'returning should restore the same center color');
            assert.deepEqual(await readAt(0), returned, 'a held pointer should not cycle');
        } finally {
            await browser.close();
        }
    });
}
