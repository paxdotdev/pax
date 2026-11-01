import test from 'node:test';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import vm from 'node:vm';
import {buildSync} from 'esbuild';

const source = buildSync({
    entryPoints: [fileURLToPath(new URL('../src/utils/helpers.ts', import.meta.url))],
    bundle: true, format: 'cjs', write: false,
}).outputFiles[0].text;
function load(globals) {
    const module = {exports: {}};
    vm.runInNewContext(source,
        {module, exports: module.exports, ...globals});
    return module.exports.readImageToByteBuffer;
}

test('disposing during retry backoff cancels the timer and prevents another fetch', async () => {
    const controller = new AbortController();
    let retries = 0, reads = 0, cleared;
    let scheduled;
    const retryScheduled = new Promise(resolve => {scheduled = resolve;});
    const decode = load({
        fetch: async () => {reads++; throw new Error('offline');},
        setTimeout: () => {retries++; scheduled(); return 17;},
        clearTimeout: id => {cleared = id;},
    });
    const pending = decode('film.png', controller.signal);
    await retryScheduled;
    controller.abort();
    await assert.rejects(pending, {name: 'AbortError'});
    assert.equal(reads, 1);
    assert.equal(retries, 1);
    assert.equal(cleared, 17);
});

test('aborted bitmap decoding closes its bitmap without reading pixels or retrying', async () => {
    const controller = new AbortController();
    let closed = 0;
    const decode = load({
        fetch: async () => ({ok: true, blob: async () => ({})}),
        createImageBitmap: async () => {
            controller.abort();
            return {close: () => {closed++;}};
        },
    });
    await assert.rejects(decode('film.png', controller.signal), {name: 'AbortError'});
    assert.equal(closed, 1);
});
