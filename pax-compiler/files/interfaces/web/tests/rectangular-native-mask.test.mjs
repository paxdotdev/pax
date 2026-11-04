import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {buildSync} from 'esbuild';

const module = {exports: {}};
vm.runInNewContext(buildSync({entryPoints: [new URL('../src/classes/rectangular-native-mask.ts', import.meta.url).pathname],
    bundle: true, write: false, format: 'cjs', platform: 'node'}).outputFiles[0].text, {module, exports: module.exports});
const {nativeMaskRectangle: rectangle, rectangularNativeMaskCoverage: coverage} = module.exports;
const rect = (l, t, r, b) => `M${l},${t}L${r},${t}L${r},${b}L${l},${b}Z`;
const entry = (path, opacity = 1, clips = []) => ({path, opacity, clips});
const request = entries => ({entries, width: 200, height: 100, scale: 2, key: ''});
const plain = value => value && {...value};

test('recognizes exact rectangular outlines including zero-radius Pax corners and reversed winding', () => {
    for (const path of [rect(0, -12.5, 200, 80),
        'M0,-12.5C0,-12.5 0,-12.5 0,-12.5L200,-12.5C200,-12.5 200,-12.5 200,-12.5L200,80C200,80 200,80 200,80L0,80C0,80 0,80 0,80Z',
        'M200,80L200,-12.5L0,-12.5L0,80L200,80Z',
        'M0,-1.25e1L2e2,-1.25e1L200,80L0,80Z']) {
        assert.deepEqual(plain(rectangle(path)), {left: 0, top: -12.5, right: 200, bottom: 80});
    }
});

test('does not mistake bounding boxes, rounded corners, holes, or malformed input for rectangles', () => {
    for (const path of ['M0,0L200,10L200,80L0,70Z', 'M0,0L200,80L200,0L0,80Z',
        'M0,0L200,0L200,80L0,80', 'M0,0L200,0L200,80Z',
        'M0,0L200,0L200,80L0,80Z M10,10L20,10L20,20L10,20Z',
        'M0,0L200,0L200,80L0,80Zjunk', 'm0,0h200v80h-200z',
        'M0,0L0,0L0,0L0,0Z', 'M0,0C0,0 1,0 0,0L200,0L200,80L0,80Z',
        'M0,0Q0,1 0,0L200,0L200,80L0,80Z', 'MNaN,0L200,0L200,80L0,80Z',
        'M0,0L1e999,0L200,80L0,80Z']) assert.equal(rectangle(path), undefined, path);
});

test('opaque panel subsumes interior paths and conservatively bounded clips, not protruding shapes', () => {
    const panel = entry(rect(0, 0, 200, 80));
    assert.deepEqual(plain(coverage(request([panel, entry('M10,10Q50,20 30,30Z', .3)]))),
        {left: 0, top: 0, right: 200, bottom: 80});
    assert.ok(coverage(request([entry(rect(-20, -20, 220, 120), .7, [rect(10, 10, 190, 70)]), panel])));
    for (const other of [entry(rect(10, 10, 30, 90)), entry(rect(10, 0, 30, 20)),
        entry('unknown'), entry(rect(10, 10, 30, 30), 1, ['unknown'])]) {
        assert.equal(coverage(request([panel, other])), undefined);
    }
    assert.ok(coverage(request([panel, entry('unknown', 0)])));
    assert.ok(coverage(request([panel, entry(rect(220, 110, 250, 150))])));
});

test('only opaque, unclipped candidates qualify; bounds are cropped without rounding authored placement', () => {
    assert.equal(coverage(request([entry(rect(0, 0, 200, 80), .5)])), undefined);
    assert.equal(coverage(request([entry(rect(0, 0, 200, 80), 1, [rect(0, 0, 100, 80)])])), undefined);
    assert.deepEqual(plain(coverage(request([entry(rect(-20, -3.125, 250, 12.875))]))),
        {left: 0, top: 0, right: 200, bottom: 12.875});
    assert.equal(coverage(request([entry(rect(0, -80, 200, -1))])), undefined);
    for (const key of ['width', 'height', 'scale']) {
        for (const value of [0, -1, NaN, Infinity]) {
            assert.equal(coverage({...request([entry(rect(0, 0, 200, 80))]), [key]: value}), undefined);
        }
    }
});
