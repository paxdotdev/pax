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
const viewport = {devicePixelRatio: 1, innerWidth: 900, innerHeight: 700, scrollX: 0, scrollY: 0};
vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
    module, exports: module.exports, require: surfaceRequire,
    window: viewport, performance,
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
        surfacePlanPending: false,
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

test('a plan wake does not refresh unchanged survivors or issue a global refresh', () => {
    const h = harness([['root', canvas(1)], ['row', canvas(2)]]);
    h.pool.surfacePlanPending = true;
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, []);
    assert.equal(h.pool.surfacePlanPending, false);
});

test('priority-only plan changes refresh their layer, not every installed layer', () => {
    const row = canvas(2);
    Object.assign(row, {width: 200, height: 100, parentElement: {dataset: {}}});
    Object.assign(row.dataset, {logicalWidth: '100', logicalHeight: '50',
        expectedBackingWidth: '200', expectedBackingHeight: '100',
        surfaceGeneration: '5', surfaceConfiguredGeneration: '5',
        surfaceContentRevision: '7', surfaceSubmittedRevision: '7', replayPriority: '3'});
    const h = harness([['root', canvas(1)], ['row', row], ['other', canvas(3)]]);
    const before = {...row.dataset};
    const snapshots = [], trace = [];
    h.pool.chassis.install_surface_plan = plan => snapshots.push(plan);
    h.pool.surfaceCoordinator.trace = {record: (type, data) => trace.push({type, ...data}),
        firstVisibleAt: () => 1};
    for (const priority of ['0', '-1', '4']) {
        h.refreshes.length = 0;
        row.dataset.replayPriority = priority;
        h.pool.surfacePlanPending = true;
        h.pool.syncRenderSurfaceLayouts();
        assert.deepEqual(h.refreshes, [2], 'promotion, readiness and demotion are targeted');
        assert.equal(snapshots.at(-1).surfaces[0].replayPriority, Number(priority));
        const refresh = trace.filter(e => e.type === 'refresh').at(-1);
        assert.deepEqual(Array.from(refresh.surfaceChanged), []);
        assert.deepEqual(Array.from(refresh.transformChanged), []);
        assert.deepEqual(Array.from(refresh.priorityChanged), [2]);
        assert.equal(row.dataset.surfaceGeneration, before.surfaceGeneration);
        assert.equal(row.dataset.surfaceContentRevision, before.surfaceContentRevision);
        assert.equal(row.dataset.surfaceSubmittedRevision, before.surfaceSubmittedRevision);
        assert.deepEqual([row.width, row.height], [200, 100]);
    }
});

test('simultaneous priority and geometry changes refresh both affected layers', () => {
    const root = canvas(1), row = canvas(2);
    const h = harness([['root', root], ['row', row], ['other', canvas(3)]]);
    root.dataset.transformSignature = '0,1024';
    row.dataset.replayPriority = '-1';
    h.pool.surfacePlanPending = true;
    assert.deepEqual(h.sync(), [1, 2]);
});

test('parking and unparking reconcile only the affected host layers', () => {
    const row = canvas(2), host = {dataset: {}, style: {}};
    row.parentElement = host;
    const h = harness([['root', canvas(1)], ['row', row], ['other', canvas(3)]]);
    const hosts = {canvasHost: host, contentHost: {dataset: {}, style: {}}, parked: false};
    h.pool.parkScrollerHosts(hosts);
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [2]);
    h.refreshes.length = 0;
    h.pool.unparkScrollerHosts(hosts);
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [2]);
    h.refreshes.length = 0;
    h.pool.surfacePlanPending = true;
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [], 'unchanged activation is not an invalidation');
});

test('ownership changes with unchanged layer counts still reconcile both owners', () => {
    const a = canvas(1), b = canvas(2);
    const h = harness([['a', a], ['b', b], ['other', canvas(3)]]);
    a.dataset.layerId = '2'; b.dataset.layerId = '1';
    h.pool.surfacePlanPending = true;
    assert.deepEqual(h.sync(), [1, 2]);
});

test('pending configuration is not repeatedly repaired; configured backings still are', () => {
    const tile = canvas(7);
    Object.assign(tile, {width: 1, height: 1});
    Object.assign(tile.dataset, {surfaceGeneration: '1', expectedBackingWidth: '200', expectedBackingHeight: '100'});
    const h = harness([['tile', tile]]);
    for (let i = 0; i < 4; i++) h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [], 'queued/failed initialization waits for renderer progress or changed layout');
    tile.dataset.surfaceSignature = 'replacement-after-failure';
    h.pool.lastLayerCanvasPlanGeneration = 0;
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [7], 'a changed layout still retries a failed/unconfigured assignment');
    h.refreshes.length = 0;
    tile.width = 200; tile.height = 100;
    tile.dataset.surfaceConfiguredGeneration = '1';
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [], 'configuration completion alone does not invalidate content');
    tile.width = 1;
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [7], 'a configured backing reset is a real mismatch');
    tile.width = 200;
    h.refreshes.length = 0;
    tile.dataset.expectedBackingWidth = '300';
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [7], 'resize/DPR/backend-size changes still need reconciliation');
});

test('configuration acknowledgement belongs to one assignment, not a recycled canvas', () => {
    const tile = canvas(7);
    Object.assign(tile, {width: 200, height: 100});
    Object.assign(tile.dataset, {surfaceGeneration: '1', surfaceConfiguredGeneration: '1',
        expectedBackingWidth: '200', expectedBackingHeight: '100'});
    const h = harness([['tile', tile]]);
    tile.dataset.surfaceGeneration = '2'; tile.width = tile.height = 1;
    h.sync();
    assert.deepEqual(h.refreshes, [7], 'new assignment is published once');
    h.refreshes.length = 0;
    for (let i = 0; i < 4; i++) h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, [], 'old acknowledgement cannot repair the pending new assignment');
    tile.dataset.surfaceConfiguredGeneration = '2'; tile.width = 200; tile.height = 100;
    h.pool.syncRenderSurfaceLayouts();
    assert.deepEqual(h.refreshes, []);
});

test('capacity availability wakes surface work, without lifecycle ticks or forced survivor refresh', () => {
    const h = harness([['tile', canvas(7)]]);
    let wakes = 0, layouts = 0;
    h.pool.setPostAsyncInterruptFlush(() => assert.fail('capacity is not an application interrupt'));
    h.pool.setSurfaceWorkWake(() => wakes++);
    h.pool.layers.syncLayerCanvasLayouts = () => layouts++;
    h.pool.surfaceCapacityAvailable();
    assert.equal(wakes, 1);
    h.pool.syncRenderSurfaceLayouts();
    assert.equal(layouts, 1, 'available capacity retries allocation');
    assert.deepEqual(h.refreshes, [], 'survivors do not need replay just because capacity changed');
    h.pool.syncRenderSurfaceLayouts();
    assert.equal(layouts, 1, 'notification was consumed');
    h.pool.chassis = undefined;
    h.pool.surfaceCapacityAvailable();
    assert.equal(wakes, 1, 'late old-chassis notification is ignored');
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

// Keep the actual candidate/admission/coordinator path. Only Rust's plan and
// scheduler outputs and the DOM installer are inputs; time is deterministic.
function settlingPlanHarness(change) {
    let now = 0, installs = 0;
    const isolated = {exports: {}};
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module: isolated, exports: isolated.exports, require: surfaceRequire,
        window: viewport, performance: {now: () => now},
    });
    const tile = {parentElement: {dataset: {}}, dataset: {}};
    const snapshots = [], refreshes = [];
    const pool = Object.create(isolated.exports.NativeElementPool.prototype);
    Object.assign(pool, {
        canvases: new Map([['tile', tile]]), layerCanvasPlanCache: new Map(),
        surfacePlanPending: false,
        buildWarmScrollerCandidates: () => new Map(),
        layers: {layerIds: () => [0], syncLayerCanvasLayouts(provider) {
            const s = provider(0).surfaces[0];
            installs++;
            tile.width = s.backingWidth; tile.height = s.backingHeight;
            Object.assign(tile.dataset, {
                layerId: '0', tileKey: s.key, tileOriginX: String(s.left), tileOriginY: String(s.top),
                logicalWidth: String(s.width), logicalHeight: String(s.height),
                surfaceGeneration: '1', surfaceConfiguredGeneration: '1',
                expectedBackingWidth: String(s.backingWidth), expectedBackingHeight: String(s.backingHeight),
                densityClamped: String(!!s.densityClamped), hostSignature: s.hostSignature,
                surfaceSignature: s.surfaceSignature, transformSignature: s.transformSignature,
                replayPriority: String(s.replayPriority),
            });
        }},
        chassis: {
            // Neither generation, policy nor DPR changes when the timer settles.
            layer_canvas_plan_generation: () => 1, surface_policy_revision: () => 1,
            install_surface_plan: plan => snapshots.push(plan),
            interrupt: event => refreshes.push(event.RenderSurfaceUpdate.layer_id),
            get_layer_canvas_plan_for_transform() {
                return {layerId: 0, active: true, densityPending: now < 500, surfaces: [{
                    id: 'tile', key: 'single', left: 0, top: 0, width: 100, height: 100,
                    backingWidth: 200, backingHeight: 200, hostSignature: 'root',
                    surfaceSignature: '100x100:backing=200x200', transformSignature: '0,0',
                    replayPriority: -1, ...(now < 500 ? {} : change),
                }]};
            },
            schedule_surfaces(input) {
                return {admitted: [0], denied: [], overloaded: false,
                    backingPixels: input.candidates[0].pixels,
                    ranks: [{coverage: 0, distance: 0, retained: input.candidates[0].retained, ready: false}]};
            },
        },
    });
    return {tile, snapshots, refreshes, get installs() { return installs; },
        sync(time) { now = time; pool.syncRenderSurfaceLayouts(); }};
}

test('settled density is installed without a new interrupt, generation or priority', () => {
    const h = settlingPlanHarness({backingWidth: 100, backingHeight: 100,
        surfaceSignature: '100x100:backing=100x100'});
    h.sync(0);
    h.sync(200);
    assert.equal(h.tile.width, 200, 'retain the valid backing during hysteresis');
    assert.equal(h.installs, 1);
    h.sync(500);
    assert.equal(h.tile.width, 100, 'settlement reaches the installer before any further input');
    assert.equal(h.snapshots.at(-1).surfaces[0].backingWidth, 100);
    assert.equal(h.installs, 2);
    h.refreshes.length = 0;
    h.sync(600);
    h.sync(1000);
    assert.equal(h.installs, 2, 'stable frames do not reinstall');
    assert.deepEqual(h.refreshes, [], 'stable frames do not replay');
});

test('same-ID admitted descriptor changes reach installation', async t => {
    for (const [field, value] of Object.entries({key: 'other', left: 10, top: 20,
        width: 120, height: 130, backingWidth: 240, backingHeight: 260,
        surfaceSignature: 'changed', transformSignature: '10,20',
        hostSignature: 'replacement', densityClamped: true})) {
        await t.test(field, () => {
            const h = settlingPlanHarness({[field]: value});
            h.sync(0);
            h.refreshes.length = 0;
            h.sync(500);
            assert.equal(h.installs, 2);
            assert.deepEqual(h.refreshes, [0], 'changed installed geometry reaches the renderer');
            if (!field.endsWith('Signature') || field === 'hostSignature') {
                assert.equal(h.snapshots.at(-1).surfaces[0][field], value);
            }
            h.sync(600);
            assert.equal(h.installs, 2);
        });
    }
});

test('densityPending alone is not an installation change', () => {
    const h = settlingPlanHarness({});
    h.sync(0);
    h.refreshes.length = 0;
    h.sync(500);
    h.sync(600);
    assert.equal(h.installs, 1);
    assert.deepEqual(h.refreshes, []);
});

test('a warm tile becoming visible refreshes replay priority without changing its backing', () => {
    const tile = canvas(2);
    tile.dataset.replayPriority = '3';
    const h = harness([['row', tile]]);
    tile.dataset.replayPriority = '0';
    assert.deepEqual(h.sync(), [2]);
});

function geometryHarness() {
    const h = harness([]);
    let reads = 0, now = 10;
    const preparations = [];
    const leaf = {clientWidth: 100, clientHeight: 100};
    const state = {active: true, stableFrames: 0,
        lastMeasuredScrollX: 0, lastMeasuredScrollY: 0,
        lastMeasuredPresentationScrollX: 0, lastMeasuredPresentationScrollY: 0};
    Object.assign(h.pool, {
        scrollerHosts: new Map([[1, {leaf, canvasHost: {dataset: {}}}]]),
        scrollerMeasurementStates: new Map([[1, state]]),
        nodesLookup: new Map([[1, leaf]]),
        getWarmthPresentationRecord() {
            reads++;
            return {presentedBounds: {left: 0, top: 0, right: 100, bottom: 100},
                presentedClipBounds: {left: 0, top: 0, right: 900, bottom: 700}};
        },
        getIntermediateScrollerClip: () => undefined,
        getParentScrollerId: () => undefined,
        syncDelegatedPageScrollViewport() {},
        emitScrollerPosition() {},
        syncWarmScrollerHosts(continuation) {
            this.buildWarmScrollerCandidates(now++, continuation);
            return true;
        },
    });
    h.pool.chassis.prepare_surface_hosts = input => {
        preparations.push(input);
        return [{id: 1, visible: [0,0,100,100], prepared: [0,0,100,200]}];
    };
    return {...h, state, preparations, reads: () => reads};
}

test('continuations reuse geometry but still observe time and new submissions', () => {
    const h = geometryHarness();
    h.pool.syncRenderSurfaceLayouts();
    h.pool.syncRenderSurfaceLayouts(true);
    const tile = canvas(1);
    Object.assign(tile.dataset, {surfaceContentRevision: '2', surfaceSubmittedRevision: '2',
        surfacePreparedAt: '1', surfaceGeneration: '3'});
    h.pool.canvases.set('ready', tile);
    h.pool.syncRenderSurfaceLayouts(true);
    assert.equal(h.reads(), 1);
    assert.deepEqual(h.preparations.map(p => p.now), [10,11,12]);
    assert.equal(h.preparations[2].readiness[0][0], 'ready:3:2');
    h.pool.surfaceCapacityAvailable();
    h.pool.syncRenderSurfaceLayouts(true);
    assert.equal(h.reads(), 1, 'capacity release does not invalidate geometric observations');
    h.pool.syncRenderSurfaceLayouts();
    assert.equal(h.reads(), 2, 'ordinary application frames refresh observations');
});

test('native reversal sampling invalidates geometry even before a new plan generation', () => {
    const h = geometryHarness();
    h.pool.syncRenderSurfaceLayouts();
    for (const y of [80, 20, 20.05]) {
        h.state.active = true;
        h.pool.measureScrollerPosition = () => ({
            scrollX: 0, scrollY: y, presentationScrollX: 0, presentationScrollY: y});
        h.pool.sampleFrameInputs();
        h.pool.syncRenderSurfaceLayouts(true);
        assert.equal(h.preparations.at(-1).hosts[0].offset[1], y);
    }
    assert.equal(h.reads(), 4, 'even movement below event tolerance invalidates observations');
});

test('geometry observations expire on resize, plan/policy changes, and native patches', () => {
    const h = geometryHarness();
    h.pool.syncRenderSurfaceLayouts();
    const invalidations = [
        () => { viewport.innerWidth++; },
        () => { viewport.visualViewport = {width: 600, height: 500, offsetTop: 20, offsetLeft: 0, scale: 1.5}; },
        () => { h.pool.chassis.layer_canvas_plan_generation = () => 2; },
        () => { h.pool.chassis.surface_policy_revision = () => 1; },
        () => { viewport.devicePixelRatio = 2; },
        () => h.pool.invalidateSurfaceHostObservation(),
    ];
    for (const invalidate of invalidations) {
        const before = h.reads();
        invalidate();
        h.pool.syncRenderSurfaceLayouts(true);
        assert.equal(h.reads(), before + 1);
        h.pool.syncRenderSurfaceLayouts(true);
        assert.equal(h.reads(), before + 1);
    }
    viewport.innerWidth = 900; viewport.devicePixelRatio = 1;
    delete viewport.visualViewport;
    const index = readFileSync(new URL('../src/index.ts', import.meta.url), 'utf8');
    assert.match(index, /function processMessages[\s\S]*?nativePool\.invalidateSurfaceHostObservation\(\)/);
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
        hasAttribute(name) { return this.getAttribute(name) !== null; }
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
        require: name => name === './surface-host-policy' ? surfaceRequire(name) : ({releaseCanvasBacking(canvas) {
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
    backing.dataset.surfaceConfiguredGeneration = firstGeneration;
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
    assert.equal(backing.dataset.surfaceConfiguredGeneration, undefined, 'retirement clears configuration acknowledgement');
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
