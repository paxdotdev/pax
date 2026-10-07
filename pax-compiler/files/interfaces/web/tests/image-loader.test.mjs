import test from 'node:test';
import assert from 'node:assert/strict';
import {surfaceRequire} from './surface-test-loader.mjs';
const {ImageLoader} = surfaceRequire('./image-loader');

function harness() {
    const reads = [], completes = [], wakes = [], loaded = new Set();
    const loader = new ImageLoader({
        loaded: path => loaded.has(path),
        decode: (path, signal) => new Promise((resolve, reject) => reads.push({path, signal, resolve, reject})),
        complete: (path, image, ids) => { loaded.add(path); completes.push({path, image, ids: [...ids]}); },
        ready: ids => wakes.push([...ids]),
    });
    return {loader, reads, completes, wakes, loaded};
}
const image = {pixels: new Uint8ClampedArray([255, 0, 0, 255]), width: 1, height: 1};

test('420 requests share one decode and one pixel publication per resource', async () => {
    const h = harness();
    const requests = Array.from({length: 420}, (_, id) => h.loader.request('same.png', id));
    await Promise.resolve();
    assert.equal(h.reads.length, 1);
    h.reads[0].resolve(image);
    await Promise.all(requests);
    assert.equal(h.completes.length, 1);
    assert.equal(h.completes[0].ids.length, 420);
    assert.equal(h.completes[0].image, image);
    await h.loader.request('same.png', 500);
    assert.equal(h.reads.length, 1);
    assert.deepEqual(h.wakes, [[500]], 'cache hits wake newly mounted/cull-clean consumers');
});

test('source changes detach only the changed consumer, not a shared request', async () => {
    const h = harness();
    const a = h.loader.request('old', 1), b = h.loader.request('old', 2), c = h.loader.request('new', 1);
    await Promise.resolve();
    h.reads[1].resolve(image);
    await c;
    h.reads[0].resolve(image);
    await Promise.all([a, b]);
    assert.deepEqual(h.completes.map(c => [c.path, c.ids]), [['new', [1]], ['old', [2]]]);
});

test('failure releases in-flight state so a later request can retry', async () => {
    const h = harness();
    const first = h.loader.request('retry', 1);
    await Promise.resolve();
    h.reads[0].reject(new Error('decode failed'));
    await assert.rejects(first, /decode failed/);
    const second = h.loader.request('retry', 2);
    await Promise.resolve();
    h.reads[1].resolve(image);
    await second;
    assert.deepEqual(h.completes[0].ids, [2]);
});

test('chassis teardown aborts requests and rejects late publication into a replacement', async () => {
    const h = harness();
    const pending = h.loader.request('late', 1);
    await Promise.resolve();
    h.loader.dispose();
    assert.equal(h.reads[0].signal.aborted, true);
    h.reads[0].resolve(image);
    await pending;
    await h.loader.request('late', 2);
    assert.equal(h.reads.length, 1);
    assert.equal(h.completes.length, 0);
});

test('teardown before queued work starts does not invoke the decoder', async () => {
    const h = harness();
    const pending = h.loader.request('queued', 1);
    h.loader.dispose();
    await pending;
    assert.equal(h.reads.length, 0);
});
