import test from 'node:test';
import assert from 'node:assert/strict';
import {buildSync} from 'esbuild';
import vm from 'node:vm';

const bundle = name => buildSync({entryPoints: [new URL(`../src/classes/${name}.ts`, import.meta.url).pathname],
    bundle: true, write: false, format: 'cjs'}).outputFiles[0].text;
const hostSource = bundle('native-mask-rasterizer');
const workerSource = bundle('native-mask-worker');
const request = (key = 'A') => ({key, width: 20, height: 20, scale: 2,
    entries: [{path: 'M0,0L20,0L20,20L0,20Z', clips: [], opacity: 1}]});
function fixture(factoryError = false) {
    const module = {exports: {}};
    vm.runInNewContext(hostSource, {module, exports: module.exports});
    const worker = {sent: [], terminated: false,
        postMessage(message) {this.sent.push(message);},
        terminate() {this.terminated = true;},
        reply(id, result = {}) {this.onmessage({data: {id, result}});}};
    const fallbacks = [];
    let starts = 0;
    const rasterizer = new module.exports.NativeMaskRasterizer(() => {
        starts++;
        if (factoryError) throw new Error('CSP blocked');
        return worker;
    }, request => {
        const result = {request, disposed: false, ready: Promise.resolve({fallback: request.key}),
            dispose() {this.disposed = true;}};
        fallbacks.push(result);
        return result;
    });
    return {rasterizer, worker, fallbacks, starts: () => starts};
}

test('one worker serializes owners without allocating raster buffers for queued jobs', async () => {
    const {rasterizer, worker, starts} = fixture();
    const a = rasterizer.encode(request()), b = rasterizer.encode(request('B'));
    assert.equal(worker.sent.length, 1);
    worker.reply(1, {blob: 'A'});
    assert.equal(worker.sent.length, 2);
    worker.reply(2, {blob: 'B'});
    assert.deepEqual(await a.ready, {blob: 'A'});
    assert.deepEqual(await b.ready, {blob: 'B'});
    assert.equal(starts(), 1);
    rasterizer.dispose();
    assert.equal(worker.terminated, true);
});

test('cancelling queued and active owners discards late replies and dispatches only live work', async () => {
    const {rasterizer, worker, fallbacks} = fixture();
    const a = rasterizer.encode(request()), b = rasterizer.encode(request('B')), c = rasterizer.encode(request('C'));
    const rejectedA = assert.rejects(a.ready, /cancelled/), rejectedB = assert.rejects(b.ready, /cancelled/);
    a.dispose(); b.dispose();
    await rejectedA; await rejectedB;
    assert.equal(worker.sent.length, 1);
    worker.reply(1);
    assert.equal(worker.sent.length, 2);
    assert.equal(worker.sent[1].request.key, 'C');
    worker.reply(1); // An obsolete reply cannot complete or clear another job.
    worker.reply(3, {blob: 'C'});
    assert.deepEqual(await c.ready, {blob: 'C'});
    assert.equal(fallbacks.length, 0);
    rasterizer.dispose();
});

for (const failure of ['factory', 'error', 'messageerror', 'reply', 'send']) {
    test(`${failure} failure falls back once, releases the worker and preserves cancellation`, async () => {
        const {rasterizer, worker, fallbacks, starts} = fixture(failure === 'factory');
        if (failure === 'send') worker.postMessage = () => {throw new Error('clone failed');};
        const a = rasterizer.encode(request()), b = rasterizer.encode(request('B'));
        if (failure === 'error') worker.onerror({preventDefault() {}});
        if (failure === 'messageerror') worker.onmessageerror();
        if (failure === 'reply') worker.onmessage({data: {id: 1, unavailable: true, error: 'OffscreenCanvas unavailable'}});
        assert.equal((await a.ready).fallback, 'A');
        assert.equal((await b.ready).fallback, 'B');
        a.dispose(); b.dispose();
        assert.ok(fallbacks.every(job => job.disposed));
        const c = rasterizer.encode(request('C'));
        assert.equal((await c.ready).fallback, 'C');
        assert.equal(starts(), 1);
        c.dispose(); rasterizer.dispose();
    });
}

test('host disposal rejects all pending work without starting main-thread fallback', async () => {
    const {rasterizer, worker, fallbacks} = fixture();
    const a = rasterizer.encode(request()), b = rasterizer.encode(request('B'));
    const rejected = [assert.rejects(a.ready, /disposed/), assert.rejects(b.ready, /disposed/)];
    rasterizer.dispose(); rasterizer.dispose();
    await Promise.all(rejected);
    worker.reply(1);
    await assert.rejects(rasterizer.encode(request()).ready, /unavailable/);
    assert.equal(fallbacks.length, 0);
    assert.equal(worker.terminated, true);
});

test('a per-mask encoding failure does not disable the worker or fall back every owner', async () => {
    const {rasterizer, worker, fallbacks} = fixture();
    const a = rasterizer.encode(request()), b = rasterizer.encode(request('B'));
    const failed = assert.rejects(a.ready, /invalid path/);
    worker.onmessage({data: {id: 1, error: 'invalid path'}});
    await failed;
    assert.equal(worker.terminated, false);
    assert.equal(worker.sent.length, 2);
    worker.reply(2, {blob: 'B'});
    assert.deepEqual(await b.ready, {blob: 'B'});
    assert.equal(fallbacks.length, 0);
    rasterizer.dispose();
});

test('worker paints the same cropped geometry, exports PNG and frees its backing', async () => {
    const sent = [], canvases = [], calls = [];
    const context = {setTransform: (...args) => calls.push(args), save() {}, restore() {},
        clip(path) {calls.push(['clip', path.path]);}, fill(path) {calls.push(['fill', path.path]);}};
    const self = {postMessage: message => sent.push(message)};
    vm.runInNewContext(workerSource, {module: {exports: {}}, self,
        Path2D: class {constructor(path) {this.path = path;}},
        OffscreenCanvas: class {
            constructor(width, height) {this.width = width; this.height = height; canvases.push(this);}
            getContext() {return context;}
            async convertToBlob(options) {assert.equal(options.type, 'image/png'); return {pixels: true};}
        }});
    await self.onmessage({data: {id: 9, request: request()}});
    assert.deepEqual(calls[0], [2, 0, 0, 2, -0, -0]);
    assert.equal(sent[0].id, 9);
    assert.equal(sent[0].result.blob.pixels, true);
    assert.equal(canvases[0].width, 0);
    assert.equal(canvases[0].height, 0);
    const empty = request(); empty.entries = [];
    await self.onmessage({data: {id: 10, request: empty}});
    assert.equal(canvases.length, 1);
    assert.equal(sent[1].id, 10);
});

test('worker failures are reported to the owner and release temporary pixels', async () => {
    const sent = [], canvases = [];
    const self = {postMessage: message => sent.push(message)};
    vm.runInNewContext(workerSource, {module: {exports: {}}, self,
        Path2D: class {},
        OffscreenCanvas: class {
            constructor(width, height) {this.width = width; this.height = height; canvases.push(this);}
            getContext() {return null;}
        }});
    await self.onmessage({data: {id: 1, request: request()}});
    assert.match(sent[0].error, /context unavailable/);
    assert.equal(canvases[0].width, 0);
});
