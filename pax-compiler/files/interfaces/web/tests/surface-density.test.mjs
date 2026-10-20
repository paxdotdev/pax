import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync, mkdtempSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
import {surfaceRequire} from './surface-test-loader.mjs';

const directory = mkdtempSync(join(tmpdir(), 'pax-surface-density-'));
let fixtures;
try {
    const binary = join(directory, 'metrics');
    execFileSync('rustc', ['--edition=2021', '-Awarnings',
        fileURLToPath(new URL('../../../../../pax-chassis-web/tests/surface_metrics_fixture.rs', import.meta.url)),
        '-o', binary]);
    fixtures = JSON.parse(execFileSync(binary, {encoding: 'utf8'}));
} finally {
    rmSync(directory, {recursive: true, force: true});
}

const source = readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8');
const module = {exports: {}};
const window = {devicePixelRatio: 1};
vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
    module, exports: module.exports, require: surfaceRequire, window,
});

for (const f of fixtures) {
    test(`Rust backing contract ${f.width}×${f.height} @ ${f.dpr}, clamped=${f.densityClamped}`, () => {
        window.devicePixelRatio = f.dpr;
        const refreshes = [];
        const canvas = {width: f.backingWidth, height: f.backingHeight,
            parentElement: {dataset: {role: 'scroller-canvas-host'}}, dataset: {
                layerId: '7', logicalWidth: String(f.width), logicalHeight: String(f.height),
                expectedBackingWidth: String(f.backingWidth), expectedBackingHeight: String(f.backingHeight),
                tileKey: '0:0', surfaceSignature: 'size', transformSignature: '0,0',
                hostSignature: 'scroller:42', surfaceGeneration: '3', surfaceConfiguredGeneration: '3',
            }};
        const pool = Object.create(module.exports.NativeElementPool.prototype);
        Object.assign(pool, {
            canvases: new Map([['tile', canvas]]),
            layerCanvasPlanCache: new Map(),
            surfaceRefreshPending: false,
            chassis: {layer_canvas_plan_generation: () => 1, surface_policy_revision: () => 0,
                interrupt: e => refreshes.push(e.RenderSurfaceUpdate.layer_id)},
            layers: {syncLayerCanvasLayouts() {}}, syncWarmScrollerHosts() { return true; },
        });
        pool.syncRenderSurfaceLayouts();
        refreshes.length = 0;
        for (let frame = 0; frame < 5; frame++) pool.syncRenderSurfaceLayouts();
        assert.deepEqual(refreshes, [], 'valid unchanged backings must not cause replay/refresh churn');
        assert.equal(canvas.dataset.surfaceGeneration, '3');
        canvas.width += 1;
        pool.syncRenderSurfaceLayouts();
        assert.deepEqual(refreshes, [7], 'even a one-pixel allocation mismatch is detected');
    });
}

test('fractional and hard-limit expectations actually came from Rust', () => {
    assert.equal(fixtures.find(f => f.width === 73.8181 && f.dpr === 2).backingHeight, 1069);
    assert.equal(fixtures.find(f => f.width === 3000 && f.dpr === 3).backingWidth, 2048);
    assert.equal(fixtures.find(f => f.width === 500 && f.dpr === 2).backingWidth, 1000);
});
