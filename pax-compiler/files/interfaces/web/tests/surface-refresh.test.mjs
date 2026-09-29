import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Exercise the real reconciliation method without a browser or GPU. Geometry and
// warm-host planning are inputs here; the interrupt stream is the renderer contract.
const source = readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8');
const module = {exports: {}};
vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
    module, exports: module.exports, require: () => ({}),
    window: {devicePixelRatio: 1},
});
const {NativeElementPool} = module.exports;

function canvas(layer, surface = 'size:100', transform = '0,0') {
    return {parentElement: {}, dataset: {
        layerId: String(layer), hostSignature: `scroller:${layer}`,
        surfaceSignature: surface, transformSignature: transform,
    }};
}

function harness(entries) {
    const refreshes = [];
    const pool = Object.create(NativeElementPool.prototype);
    Object.assign(pool, {
        canvases: new Map(entries),
        lastCanvasSurfaceSignatures: new Map(), lastCanvasTransformSignatures: new Map(),
        lastCanvasLayerCounts: new Map(), layerCanvasPlanCache: new Map(),
        surfaceRefreshPending: false,
        chassis: {layer_canvas_plan_generation: () => 1,
            interrupt: event => refreshes.push(event.RenderSurfaceUpdate.layer_id)},
        layers: {syncLayerCanvasLayouts() {}},
        syncWarmScrollerHosts() {}, collectCanvasBackingMismatchLayers: () => new Set(),
    });
    pool.syncRenderSurfaceLayouts();
    refreshes.length = 0;
    return {pool, refreshes, sync() {
        pool.lastLayerCanvasPlanGeneration = 0;
        pool.syncRenderSurfaceLayouts();
        return refreshes;
    }};
}

test('a new surface and another layer retarget are both refreshed in the same frame', () => {
    const root = canvas(1), row = canvas(2);
    const h = harness([['root', root], ['row', row]]);
    root.dataset.transformSignature = '0,1024';
    row.dataset.surfaceSignature = 'size:200';
    assert.deepEqual(h.sync(), [1, 2]);
});

test('removing a row does not swallow another layer retarget', () => {
    const root = canvas(1);
    const h = harness([['root', root], ['row', canvas(2)]]);
    root.dataset.transformSignature = '0,2048';
    h.pool.canvases.delete('row');
    assert.deepEqual(h.sync(), [1, 2]);
});

test('unchanged surfaces do not request replay', () => {
    const h = harness([['root', canvas(1)]]);
    assert.deepEqual(h.sync(), []);
});

test('a recycled canvas at the same tile address receives a fresh renderer', () => {
    const tile = canvas(2);
    tile.dataset.surfaceGeneration = '7';
    const h = harness([['row-tile-0', tile]]);
    // Release shrinks/clears the backing store; the same canvas and geometry
    // can be checked out again before the next reconciliation snapshot.
    tile.dataset.surfaceGeneration = '8';
    assert.deepEqual(h.sync(), [2]);
});

test('surface generation survives steady layout and changes on each canvas assignment', () => {
    class Element {
        dataset = {}; style = {}; parentElement; children = [];
        appendChild(child) { child.parentElement = this; this.children.push(child); }
        removeChild(child) {
            this.children = this.children.filter(c => c !== child);
            child.parentElement = undefined;
        }
        getAttribute(name) {
            return this.dataset[name.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase())] ?? null;
        }
        removeAttribute(name) {
            if (name.startsWith('data-')) {
                delete this.dataset[name.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase())];
            }
        }
    }
    const layerModule = {exports: {}};
    const layerSource = readFileSync(new URL('../src/classes/layer.ts', import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(layerSource, {loader: 'ts', format: 'cjs'}).code, {
        module: layerModule, exports: layerModule.exports,
        require: () => ({releaseCanvasBacking(canvas) {
            if (!Number(canvas.getAttribute('data-pax-surface-owners'))) {
                canvas.width = 1;
                canvas.height = 1;
            }
        }}), HTMLElement: Element,
    });
    const parent = new Element();
    parent.dataset.role = 'scroller-canvas-host';
    const backing = new Element();
    const layer = new layerModule.exports.Layer({getFromPool: () => new Element()});
    layer.build(parent, 1, new Map(), {
        checkout: () => backing,
        release: canvas => canvas.parentElement?.removeChild(canvas),
    });
    layer.setCanvasPlan({layerId: 1, active: true, surfaces: [{
        id: 'tile-0', key: '0:0', left: 0, top: 0, width: 100, height: 100,
        replayPriority: 0, surfaceSignature: '100x100', transformSignature: '0,0',
        hostSignature: 'scroller:42',
    }]});
    layer.syncCanvasLayout();
    const firstGeneration = backing.dataset.surfaceGeneration;
    assert.ok(firstGeneration);
    layer.syncCanvasLayout();
    assert.equal(backing.dataset.surfaceGeneration, firstGeneration);
    layer.detachCanvases(true);
    assert.equal(backing.width, 1, 'release cleared the old backing store');
    layer.syncCanvasLayout();
    assert.notEqual(backing.dataset.surfaceGeneration, firstGeneration);
    assert.equal(backing.id, 'tile-0');

    // Rebinding the numeric layer to another row must not carry its old pixels
    // into that host, or let a late GPU initializer configure the new row.
    backing.dataset.paxSurfaceOwners = '1';
    backing.width = 300;
    const replacement = new Element();
    layer.canvasPool.checkout = () => replacement;
    const nextHost = new Element();
    nextHost.dataset.role = 'scroller-canvas-host';
    layer.attachToParents(nextHost, nextHost);
    assert.equal(backing.parentElement, undefined);
    assert.equal(backing.width, 300, 'the old GPU owner still holds this backing');
    assert.equal(replacement.parentElement, nextHost);
    assert.equal(replacement.id, 'tile-0');
});
