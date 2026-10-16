import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const module = {exports: {}};
vm.runInNewContext(transformSync(readFileSync(new URL('../src/utils/startup-options.ts', import.meta.url), 'utf8'),
    {loader: 'ts', format: 'cjs'}).code, {module, exports: module.exports, URLSearchParams});
const {resolveStartupOptions: resolve} = module.exports;

test('plain URLs use metadata; embedding can override it and diagnostics take precedence', () => {
    assert.equal(resolve({}, '').yieldMount, false);
    assert.equal(resolve({yield_mount: true}, '').yieldMount, true);
    assert.equal(resolve({yield_mount: true}, '', false).yieldMount, false);
    assert.equal(resolve({}, '', true).yieldMount, true);
    assert.equal(resolve({yield_mount: true}, '?pax_mount_yield=0').yieldMount, false);
    assert.equal(resolve({}, '?pax_mount_yield=1', false).yieldMount, true);
    assert.equal(resolve({yield_mount: true}, '?pax_mount_yield').yieldMount, true);
    assert.equal(resolve({yield_mount: 'true'}, '').yieldMount, false);
    assert.throws(() => resolve({}, '', 'true'), /boolean/);
});

test('preview is opt-in and cannot bypass synchronous startup', () => {
    assert.equal(resolve({yield_mount: true}, '').preview, false);
    assert.equal(resolve({yield_mount: true}, '?pax_startup_preview=1').preview, true);
    assert.equal(resolve({}, '?pax_startup_preview=1').preview, false);
    assert.equal(resolve({yield_mount: true}, '?pax_startup_preview=1&pax_mount_yield=0').preview, false);
});
