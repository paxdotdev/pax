import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const code = transformSync(readFileSync(new URL('../src/classes/text.ts', import.meta.url), 'utf8'), {loader: 'ts', format: 'cjs'}).code;
function harness({fontFailure = false} = {}) {
    const module = {exports: {}}, requests = [], styles = [], descriptors = [], faces = [], warnings = [];
    const document = {
        head: {querySelectorAll: () => styles, appendChild: n => styles.push(n)},
        createElement: () => ({setAttribute(key, value) {this[key] = value;}, getAttribute(key) {return this[key];}}),
        fonts: {ready: Promise.resolve(), load: async s => {descriptors.push(s);}, add: face => faces.push(face), forEach: f => faces.forEach(f)},
    };
    vm.runInNewContext(code, {
        module, exports: module.exports, require: () => ({}), document, URL, console: {warn: (...a) => warnings.push(a)},
        btoa: s => Buffer.from(s, 'binary').toString('base64'),
        FontFace: class {constructor(family, url, descriptors) {Object.assign(this, {family, url, descriptors});} async load() {if (fontFailure) throw new Error('font unavailable'); return this;}},
        fetch: async input => {
            const url = String(input); requests.push(url);
            if (url.endsWith('pax-fonts.json')) return {ok: true, json: async () => [{family: 'Example', url: 'https://fonts.googleapis.com/css2?family=Example'}]};
            if (url.includes('fonts.googleapis.com')) return {ok: true, text: async () => '@font-face {font-family: Example;src:url(https://font.test/example.woff2)}'};
            return {ok: true, headers: {get: () => 'font/woff2'}, arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer};
        },
    });
    return {...module.exports, requests, styles, descriptors, faces, warnings};
}

test('manifest preload and multiple weights share CSS and embedded asset loading', async () => {
    const h = harness(), registered = new Set();
    const preload = h.preloadManifestFonts('http://localhost/sub/pax-cartridge');
    const a = new h.Font(), b = new h.Font();
    a.fromFontPatch({Web: {family: 'Example', url: 'https://fonts.googleapis.com/css2?family=Example', style: 'Normal', weight: 'Normal'}}, registered);
    b.fromFontPatch({Web: {family: 'Example', url: 'https://fonts.googleapis.com/css2?family=Example', style: 'Normal', weight: 'Bold'}}, registered);
    const ready = [a.ready, b.ready]; a.cleanUp(); b.cleanUp();
    await Promise.all([preload, ...ready]);
    assert.equal(h.requests.filter(s => s.includes('googleapis')).length, 1);
    assert.equal(h.requests.filter(s => s.includes('woff2')).length, 1);
    assert.equal(h.styles.length, 1);
    assert.deepEqual(h.descriptors.sort(), ['normal 400 16px "Example"', 'normal 700 16px "Example"']);
    assert.equal(h.warnings.length, 0, 'pooled objects must not corrupt asynchronous font registration');
    const c = new h.Font();
    c.fromFontPatch({Web: {family: 'Example', url: 'https://fonts.googleapis.com/css2?family=Example', style: 'Normal', weight: 'Bold'}}, new Set());
    assert.equal(c.ready, ready[1], 'different pools reuse completed font readiness');
});

test('different source URLs with the same family/style/weight register independently', async () => {
    const h = harness(), registered = new Set();
    for (const path of ['first.woff2', 'replacement.woff2']) {
        const font = new h.Font();
        font.fromFontPatch({Local: {family: 'Example', path, style: 'Normal', weight: 'Normal'}}, registered);
        await font.ready;
    }
    assert.deepEqual(h.faces.map(f => f.url), ['url(first.woff2)', 'url(replacement.woff2)']);
});

test('failed fonts keep stable fallback readiness across animated style patches', async () => {
    const h = harness({fontFailure: true});
    const patch = {Local: {family: 'Missing', path: 'missing.woff2', style: 'Normal', weight: 'Normal'}};
    const first = new h.Font();
    first.fromFontPatch(patch, new Set());
    await first.ready;
    const next = new h.Font();
    next.fromFontPatch(patch, new Set());
    assert.equal(next.ready, first.ready);
    await next.ready;
    assert.equal(h.warnings.length, 1);
});
