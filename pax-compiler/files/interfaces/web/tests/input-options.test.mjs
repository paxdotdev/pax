import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const module = {exports: {}};
const source = readFileSync(new URL('../src/utils/input-options.ts', import.meta.url), 'utf8');
vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code,
    {module, exports: module.exports, require: () => ({})});
const {resolveWheelEvents: resolve} = module.exports;

test('wheel input defaults to passive, independent of diagnostic URL switches', () => {
    for (const config of [undefined, null, {}, {yield_mount: true}]) {
        assert.equal(resolve(config), 'passive');
    }
    assert.doesNotMatch(source, /location|URLSearchParams/);
});

test('host override wins over generated wheel policy and is resolved per mount', () => {
    assert.equal(resolve({wheel_events: 'cancellable'}), 'cancellable');
    assert.equal(resolve({wheel_events: 'cancellable'}, 'passive'), 'passive');
    assert.equal(resolve({wheel_events: 'passive'}, 'cancellable'), 'cancellable');
    assert.equal(resolve({}), 'passive');
});

test('invalid wheel policies fail explicitly instead of silently changing input semantics', () => {
    for (const value of [true, false, 0, '', 'cancelable', 'blocking', []]) {
        assert.throws(() => resolve({wheel_events: value}), /must be/);
        assert.throws(() => resolve({}, value), /must be/);
    }
});
