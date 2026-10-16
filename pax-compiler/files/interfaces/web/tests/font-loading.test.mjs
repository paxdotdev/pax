import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const code = transformSync(readFileSync(new URL('../src/classes/text.ts', import.meta.url), 'utf8'), {loader: 'ts', format: 'cjs'}).code;
function harness({fontFailure = false} = {}) {
    const module = {exports: {}}, requests = [], styles = [], descriptors = [], faces = [], warnings = [];
    const document = {
        baseURI: 'https://app.test/',
        head: {querySelectorAll: () => styles, appendChild: n => styles.push(n)},
        createElement: () => ({setAttribute(key, value) {this[key] = value;}, getAttribute(key) {return this[key];}}),
        fonts: {ready: Promise.resolve(), load: async (s, text) => {descriptors.push([s, text]);}, add: face => faces.push(face), forEach: f => faces.forEach(f)},
    };
    vm.runInNewContext(code, {
        module, exports: module.exports, require: () => ({}), document, URL, console: {warn: (...a) => warnings.push(a)},
        btoa: s => Buffer.from(s, 'binary').toString('base64'),
        FontFace: class {constructor(family, url, descriptors) {Object.assign(this, {family, url, descriptors});} async load() {if (fontFailure) throw new Error('font unavailable'); return this;}},
        fetch: async input => {
            const url = String(input); requests.push(url);
            if (url.endsWith('pax-fonts.json')) return {ok: true, json: async () => [{family: 'Example', url: 'https://fonts.googleapis.com/css2?family=Example'}]};
            if (url.includes('fonts.googleapis.com')) return {ok: true, text: async () => '@font-face {font-family: Example;src:url(https://font.test/example.woff2)}'};
            if (url.includes('.css')) return {ok: true, text: async () => '@font-face {font-family: Example;src:url(./example.woff2)}'};
            return {ok: true, headers: {get: () => 'font/woff2'}, arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer};
        },
    });
    return {...module.exports, requests, styles, descriptors, faces, warnings};
}

test('manifest preload registers CSS; actual text selects faces without embedding assets', async () => {
    const h = harness(), registered = new Set();
    const preload = h.preloadManifestFonts('http://localhost/sub/pax-cartridge');
    const a = new h.Font(), b = new h.Font();
    a.fromFontPatch({Web: {family: 'Example', url: 'https://fonts.googleapis.com/css2?family=Example', style: 'Normal', weight: 'Normal'}}, registered);
    b.fromFontPatch({Web: {family: 'Example', url: 'https://fonts.googleapis.com/css2?family=Example', style: 'Normal', weight: 'Bold'}}, registered);
    const ready = [a.getTextLoader()("Hello"), b.getTextLoader()("Ж")] ; a.cleanUp(); b.cleanUp();
    await Promise.all([preload, ...ready]);
    assert.equal(h.requests.filter(s => s.includes('googleapis')).length, 1);
    assert.equal(h.requests.filter(s => s.includes('woff2')).length, 0, 'display leaves binary selection to the browser');
    assert.equal(h.styles.length, 1);
    assert.deepEqual(h.descriptors.sort(), [['normal 400 16px "Example"', "Hello"], ['normal 700 16px "Example"', "Ж"]]);
    assert.equal(h.warnings.length, 0, 'pooled objects must not corrupt asynchronous font registration');
    const c = new h.Font();
    c.fromFontPatch({Web: {family: 'Example', url: 'https://fonts.googleapis.com/css2?family=Example', style: 'Normal', weight: 'Bold'}}, new Set());
    assert.equal(h.requests.length, 2, 'unused declarations do not load');
    await c.getTextLoader()('Other');
    assert.equal(h.requests.length, 2, 'different pools reuse stylesheet registration');
});

test('different source URLs with the same family/style/weight register independently', async () => {
    const h = harness(), registered = new Set();
    for (const path of ['first.woff2', 'replacement.woff2']) {
        const font = new h.Font();
        font.fromFontPatch({Local: {family: 'Example', path, style: 'Normal', weight: 'Normal'}}, registered);
        await font.getTextLoader()("Example");
    }
    assert.deepEqual(h.faces.map(f => f.url), ['url("https://app.test/first.woff2")', 'url("https://app.test/replacement.woff2")']);
});

test('failed fonts keep stable fallback readiness across animated style patches', async () => {
    const h = harness({fontFailure: true});
    const patch = {Local: {family: 'Missing', path: 'missing.woff2', style: 'Normal', weight: 'Normal'}};
    const first = new h.Font();
    first.fromFontPatch(patch, new Set());
    await first.getTextLoader()("Hello");
    const next = new h.Font();
    next.fromFontPatch(patch, new Set());
    assert.equal(next.getTextLoader(), first.getTextLoader());
    await next.getTextLoader()("Other");
    assert.equal(h.warnings.length, 1);
});

test('declared CSS sources load on demand and resolve relative assets without a global stylesheet', async () => {
    const h = harness();
    assert.equal(h.requests.length, 0);
    const font = new h.Font();
    font.fromFontPatch({Web: {family: 'Example', url: '/fonts/family.css?v=2', style: 'Normal', weight: 'Normal'}}, new Set());
    await font.getTextLoader()("Example");
    assert.deepEqual(h.requests, ['/fonts/family.css?v=2']);
    assert.equal(h.faces.length, 0, 'CSS is not passed to FontFace as a binary font');
    assert.equal(h.styles.length, 1);
    assert.equal(h.warnings.length, 0);
});


test('capture embeds fonts on demand, resolves relative URLs and caches the result', async () => {
    const h = harness();
    const font = new h.Font();
    font.fromFontPatch({Web: {family: 'Example', url: '/fonts/family.css', style: 'Normal', weight: 'Normal'}}, new Set());
    assert.equal(h.requests.length, 0);
    await font.getTextLoader()('Hello');
    assert.deepEqual(h.requests, ['/fonts/family.css']);
    assert.match(h.styles[0].textContent, /https:\/\/app.test\/fonts\/example.woff2/);
    const css = await h.getRegisteredFontCssText();
    assert.match(css, /data:font\/woff2;base64,AQID/);
    assert.equal(await h.getRegisteredFontCssText(), css);
    assert.deepEqual(h.requests, ['/fonts/family.css', 'https://app.test/fonts/example.woff2']);
});

test('unused link declarations and empty text never start a font request', async () => {
    const h = harness();
    const font = new h.Font();
    font.fromFontPatch({Web: {family: 'Unused', url: '/unused.css', style: 'Normal', weight: 'Normal'}}, new Set());
    await font.getTextLoader()('');
    assert.equal(h.requests.length, 0);
});

test('native controls register an applied font without a Text presentation gate', async () => {
    const h = harness();
    const font = new h.Font();
    font.fromFontPatch({Local: {family: 'Example', path: '/font.woff2', style: 'Normal', weight: 'Normal'}}, new Set());
    font.applyFontToDiv({style: {}});
    await h.waitForRegisteredFonts();
    assert.equal(h.faces.length, 1);
    assert.match(await h.getRegisteredFontCssText(), /data:font\/woff2;base64,AQID/);
});
