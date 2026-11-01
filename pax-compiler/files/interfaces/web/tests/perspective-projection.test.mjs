import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const module = {exports: {}};
vm.runInNewContext(transformSync(readFileSync(new URL('../src/utils/projection.ts', import.meta.url), 'utf8'),
    {loader: 'ts', format: 'cjs'}).code, {module, exports: module.exports});
const {planeMapping, planeMatrix3d, projectPoint, inversePlane} = module.exports;
const close = (a, b) => assert.ok(Math.abs(a - b) < 1e-9, `${a} != ${b}`);

test('native perspective and its inverse retain the same local input coordinates', () => {
    const matrix = planeMatrix3d([1.2, 0.3, 0.002, -0.2, 0.9, 0.001, 140, 80, 1]);
    const inverse = inversePlane(matrix);
    for (const p of [[0, 0], [150, 30], [20, 240]]) {
        const projected = projectPoint(matrix, p);
        const local = projectPoint(inverse, projected);
        close(local[0], p[0]); close(local[1], p[1]);
    }
    assert.deepEqual(Array.from(planeMapping(matrix)), [1.2, 0.3, 0.002, -0.2, 0.9, 0.001, 140, 80, 1]);
});

test('affine transport remains exact and degenerate/behind-eye mappings are unavailable', () => {
    assert.deepEqual(Array.from(projectPoint([2, 0, 0, 3, 10, 20], [4, 5])), [18, 35]);
    assert.equal(inversePlane([0, 0, 0, 1, 0, 0]), undefined);
    assert.equal(projectPoint(planeMatrix3d([1, 0, -0.1, 0, 1, 0, 0, 0, 1]), [20, 0]), undefined);
});
