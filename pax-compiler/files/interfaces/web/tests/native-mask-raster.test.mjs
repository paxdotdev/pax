import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = transformSync(readFileSync(new URL('../src/classes/native-mask-raster.ts', import.meta.url), 'utf8'),
    {loader: 'ts', format: 'cjs'}).code;
function load(globals = {}) {
    const module = {exports: {}};
    vm.runInNewContext(source, {module, exports: module.exports, ...globals});
    return module.exports;
}
const {nativeMaskPathBounds, nativeMaskRasterBounds, paintNativeMaskCoverage} = load({Path2D: class {
    constructor(path) {this.path = path;}
}});
const plain = value => value === undefined ? value : JSON.parse(JSON.stringify(value));
const rect = (x, y, w, h) => `M${x},${y}L${x+w},${y}L${x+w},${y+h}L${x},${y+h}Z`;
const entry = (path, opacity = 1, clips = []) => ({path, opacity, clips});
const request = (entries, scale = 2, width = 1800, height = 1078) => ({entries, scale, width, height, key: ''});

test('bounds contain absolute line, quadratic and cubic control hulls and repeated coordinates', () => {
    assert.deepEqual(plain(nativeMaskPathBounds('M10,20 30,40L-5,60Q1e2,-20 50,60C80,90 130,10 40,50Z')),
        {left: -5, top: -20, right: 130, bottom: 90});
    for (const path of ['M0 0h100v50z', 'M0 0A20 20 0 0 1 40 40', 'M0 0L1', 'M0 0 L', '', 'M1e999,0']) {
        assert.equal(nativeMaskPathBounds(path), undefined, path);
    }
});

test('top navigation mask encodes only its strip at screen density', () => {
    assert.deepEqual(plain(nativeMaskRasterBounds(request([entry(rect(0, 0, 1800, 80))]))),
        {left: 0, top: 0, right: 1800, bottom: 80.5});
});

test('crop unions coverage, intersects clips and includes a physical-pixel antialias guard', () => {
    const entries = [entry(rect(10.2, 20.2, 200, 200), .5, [rect(20, 30, 30, 20)]),
        entry(rect(80, 90, 5, 5)), entry(rect(1000, 1000, 10, 10), 0)];
    assert.deepEqual(plain(nativeMaskRasterBounds(request(entries))),
        {left: 19.5, top: 29.5, right: 85.5, bottom: 95.5});
    assert.equal(nativeMaskRasterBounds(request([entry(rect(-50, -50, 20, 20))])), undefined);
    assert.equal(nativeMaskRasterBounds(request([entry(rect(10, 10, 20, 20), 1, [rect(100, 100, 5, 5)])])), undefined);
});

test('unknown syntax conservatively falls back to the surface, but known clips still bound it', () => {
    assert.deepEqual(plain(nativeMaskRasterBounds(request([entry('M0 0h100v50z')]))),
        {left: 0, top: 0, right: 1800, bottom: 1078});
    assert.deepEqual(plain(nativeMaskRasterBounds(request([entry('M0 0h100v50z', 1, [rect(10, 20, 30, 40)])]))),
        {left: 9.5, top: 19.5, right: 40.5, bottom: 60.5});
});

test('fractional dimensions and DPR retain pixel-aligned conservative bounds', () => {
    assert.deepEqual(plain(nativeMaskRasterBounds(request([entry(rect(0, 0, 101, 101))], 1.5, 100.1, 100.2))),
        {left: 0, top: 0, right: 151/1.5, bottom: 151/1.5});
});

test('raster paints ordered alpha with all entry-local clips and crop-space transform', () => {
    const calls = [];
    const ctx = {setTransform: (...args) => calls.push(['transform', ...args]),
        save: () => calls.push('save'), restore: () => calls.push('restore'),
        clip: path => calls.push(['clip', path.path]),
        fill: path => calls.push(['fill', path.path, ctx.globalAlpha])};
    paintNativeMaskCoverage(ctx, request([entry('a', .5, ['x', 'y']), entry('b', 2), entry('c', NaN)]),
        {left: 10, top: 20, right: 100, bottom: 100});
    assert.deepEqual(calls, [['transform', 2, 0, 0, 2, -20, -40],
        'save', ['clip', 'x'], ['clip', 'y'], ['fill', 'a', .5], 'restore',
        'save', ['fill', 'b', 1], 'restore', 'save', ['fill', 'c', 1], 'restore']);
});

function rasterFixture() {
    const canvases = [], images = [], revoked = [], urls = [];
    const ctx = {setTransform() {}, save() {}, restore() {}, clip() {}, fill() {}};
    const api = load({document: {createElement() {
        const canvas = {width: 0, height: 0, getContext: () => ctx,
            toBlob(callback, type) {this.finish = callback; assert.equal(type, 'image/png');}};
        canvases.push(canvas); return canvas;
    }}, Path2D: class {}, Image: class {
        constructor() {images.push(this);}
        decode() {return new Promise((resolve, reject) => {this.resolve = resolve; this.reject = reject;});}
        removeAttribute() {this.src = '';}
    }, URL: {createObjectURL(blob) {urls.push(blob); return `blob:${urls.length}`;},
        revokeObjectURL(url) {revoked.push(url);}}});
    return {api, canvases, images, revoked, urls};
}
const drain = async () => {for (let i = 0; i < 6; i++) await Promise.resolve();};

test('encoding waits for real bitmap decode and releases temporary canvas and retained resources', async () => {
    const {api, canvases, images, revoked} = rasterFixture();
    const job = api.prepareNativeMaskRaster(request([entry(rect(0, 0, 1800, 80))]));
    assert.equal(canvases[0].width, 3600); assert.equal(canvases[0].height, 161);
    assert.equal(images.length, 0);
    canvases[0].finish({}); await drain();
    assert.equal(canvases[0].width, 0); assert.equal(canvases[0].height, 0);
    let completed = false; job.ready.then(() => completed = true);
    await drain(); assert.equal(completed, false);
    images[0].resolve();
    const ready = await job.ready;
    assert.equal(ready.url, 'blob:1'); assert.equal(revoked.length, 0);
    job.dispose(); job.dispose();
    assert.deepEqual(revoked, ['blob:1']); assert.equal(images[0].src, '');
});

for (const stage of ['encode', 'decode']) {
    test(`cancellation during ${stage} cannot leak or complete a stale mask`, async () => {
        const {api, canvases, images, revoked, urls} = rasterFixture();
        const job = api.prepareNativeMaskRaster(request([entry(rect(0, 0, 50, 50))]));
        const rejected = assert.rejects(job.ready, /cancelled/);
        if (stage === 'decode') {canvases[0].finish({}); await drain();}
        job.dispose();
        if (stage === 'encode') canvases[0].finish({});
        else images[0].resolve();
        await rejected;
        assert.equal(canvases[0].width, 0);
        assert.equal(urls.length, stage === 'decode' ? 1 : 0);
        assert.equal(revoked.length, urls.length);
    });
}

test('encoding/decode failures free their resources', async () => {
    for (const stage of ['encode', 'decode']) {
        const {api, canvases, images, revoked} = rasterFixture();
        const job = api.prepareNativeMaskRaster(request([entry(rect(0, 0, 50, 50))]));
        const rejected = assert.rejects(job.ready, /failed/);
        canvases[0].finish(stage === 'encode' ? null : {}); await drain();
        if (stage === 'decode') images[0].reject(new Error('decode failed'));
        await rejected;
        assert.equal(canvases[0].width, 0);
        assert.equal(revoked.length, stage === 'decode' ? 1 : 0);
    }
});

test('empty clipped coverage needs no canvas, bitmap or URL', async () => {
    const {api, canvases, images, urls} = rasterFixture();
    const job = api.prepareNativeMaskRaster(request([entry(rect(0, 0, 50, 50), 0)]));
    assert.deepEqual(plain(await job.ready), {});
    assert.equal(canvases.length + images.length + urls.length, 0);
    job.dispose();
});
