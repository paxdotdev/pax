import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
import {surfaceRequire} from './surface-test-loader.mjs';

// Exercise the real reconciliation method without a browser or GPU. Geometry and
// warm-host planning are inputs here; the interrupt stream is the renderer contract.
const source = readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8');
const module = {exports: {}};
vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
    module, exports: module.exports, require: surfaceRequire,
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
        layerCanvasPlanCache: new Map(),
        surfaceRefreshPending: false,
        chassis: {layer_canvas_plan_generation: () => 1,
            interrupt: event => refreshes.push(event.RenderSurfaceUpdate.layer_id)},
        layers: {syncLayerCanvasLayouts() {}},
        syncWarmScrollerHosts() { return true; },
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

test('installed snapshot follows retirement and placement and precedes replay refresh', () => {
    const {SurfaceCoordinator} = surfaceRequire('./surface-coordinator');
    const events = [], canvases = new Map();
    const layer = {syncLayerCanvasLayouts(provider, retired) {
        events.push('layout');
        provider(1);
        retired([1]);
        canvases.set('ready', {parentElement: {dataset: {}}, width: 100, height: 100,
            dataset: {layerId: '1', logicalWidth: '100', logicalHeight: '100',
                expectedBackingWidth: '200', expectedBackingHeight: '200',
                surfaceGeneration: '4', hostSignature: 'row:1', replayPriority: '-1'}});
        events.push('placed');
    }};
    const coordinator = new SurfaceCoordinator(canvases, layer, () => events.push('refresh'),
        snapshot => {
            events.push('published');
            assert.deepEqual(Array.from(snapshot.pending), ['pending']);
            assert.equal(snapshot.surfaces[0].generation, '4');
            assert.equal(snapshot.surfaces[0].replayPriority, -1);
            assert.equal(snapshot.version, 2);
            assert.equal(snapshot.surfaces[0].backingWidth, 200);
            assert.equal(snapshot.surfaces[0].backingHeight, 200);
        }, () => events.push('begin'));
    coordinator.sync(() => ({surfaces: [{id: 'ready'}, {id: 'pending'}]}));
    assert.deepEqual(events, ['begin', 'layout', 'refresh', 'placed', 'published', 'refresh']);
});

test('backend policy revision invalidates plans once even without a geometry change', () => {
    const h = harness([['root', canvas(1)]]);
    let revision = 1;
    let syncCount = 0;
    h.pool.chassis.surface_policy_revision = () => revision;
    h.pool.layers.syncLayerCanvasLayouts = () => syncCount++;
    h.pool.layerCanvasPlanCache.set(1, {surfaces: []});
    h.pool.syncRenderSurfaceLayouts();
    assert.equal(h.pool.layerCanvasPlanCache.size, 0);
    h.pool.syncRenderSurfaceLayouts();
    assert.equal(syncCount, 1);
    revision++;
    h.pool.syncRenderSurfaceLayouts();
    assert.equal(syncCount, 2);
});

test('missing Rust plans reserve installed assignments without a JS geometry fallback', () => {
    const h = harness([['one', canvas(7)], ['two', canvas(7)], ['other', canvas(8)]]);
    h.pool.chassis.get_layer_canvas_plan_for_transform = () => null;
    assert.equal(h.pool.candidatePlan(7), null);
    assert.equal(h.pool.canvases.size, 3);
});

test('a pending density shrink is reobserved until Rust reports a stable backing', () => {
    const h = harness([]);
    let calls = 0;
    h.pool.chassis.get_layer_canvas_plan_for_transform = (_layer, a, b, c, d, now) => {
        calls++;
        assert.deepEqual([a,b,c,d], [0.5,0,0,0.5]);
        return {densityPending: now < 510, surfaces: []};
    };
    h.pool.candidatePlan(3, [0.5,0,0,0.5], 10);
    h.pool.candidatePlan(3, [0.5,0,0,0.5], 200);
    h.pool.candidatePlan(3, [0.5,0,0,0.5], 510);
    h.pool.candidatePlan(3, [0.5,0,0,0.5], 520);
    assert.equal(calls, 3, 'stable geometry does not keep crossing the bridge');
    h.pool.candidatePlan(3);
    assert.equal(calls, 3, 'coordinator consumes the cached transformed plan');
});

test('a warm tile becoming visible refreshes replay priority without changing its backing', () => {
    const tile = canvas(2);
    tile.dataset.replayPriority = '3';
    const h = harness([['row', tile]]);
    tile.dataset.replayPriority = '0';
    assert.deepEqual(h.sync(), [2]);
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
        module: layerModule, exports: layerModule.exports, performance,
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
        reserveBacking: () => true,
        release: canvas => canvas.parentElement?.removeChild(canvas),
    });
    layer.setCanvasPlan({layerId: 1, active: true, surfaces: [{
        id: 'tile-0', key: '0:0', left: 0, top: 0, width: 100, height: 100,
        replayPriority: 0, surfaceSignature: '100x100', transformSignature: '0,0',
        hostSignature: 'scroller:42',
        backingWidth: 150, backingHeight: 150,
    }]});
    layer.syncCanvasLayout();
    const firstGeneration = backing.dataset.surfaceGeneration;
    const preparedAt = backing.dataset.surfacePreparedAt;
    assert.ok(preparedAt);
    assert.ok(firstGeneration);
    assert.equal(backing.dataset.expectedBackingWidth, '150');
    assert.equal(backing.dataset.expectedBackingHeight, '150');
    layer.syncCanvasLayout();
    assert.equal(backing.dataset.surfaceGeneration, firstGeneration);
    assert.equal(backing.dataset.surfacePreparedAt, preparedAt, 'steady frames preserve the latency origin');
    layer.setCanvasPlan(undefined);
    layer.syncCanvasLayout();
    assert.equal(backing.dataset.surfaceGeneration, firstGeneration, 'pending plans preserve installed assignments');
    layer.setCanvasPlan({layerId: 1, active: true, surfaces: [{
        id: 'tile-0', key: '0:0', left: 0, top: 0, width: 100, height: 100,
        replayPriority: 0, surfaceSignature: '100x100', transformSignature: '0,0', hostSignature: 'scroller:42',
    }]});
    layer.detachCanvases(true);
    assert.equal(backing.width, 1, 'release cleared the old backing store');
    assert.equal(backing.dataset.expectedBackingWidth, undefined);
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
