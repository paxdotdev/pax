import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
import {surfaceRequire} from './surface-test-loader.mjs';
import {sharedSurfaceDecision} from './shared-surface-policy.mjs';

const source = readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8');

function harness(ios) {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports,
        require: name => ({...surfaceRequire(name), isIOSWebKitBrowser: () => ios}),
        performance: {now: () => 10000},
        window: {innerWidth: 1000, innerHeight: 900},
    });
    const pool = Object.create(module.exports.NativeElementPool.prototype);
    const candidates = new Map();
    function add(id, surfaceCost, mandatory, distance) {
        const hosts = {
            id, warmed: true, parked: false, vectorIslandEnabled: true, leaf: {clientWidth: 100, clientHeight: 100},
            canvasHost: {dataset: {warmState: 'warm', renderState: 'active'}, style: {}},
            contentHost: {dataset: {renderState: 'active'}, style: {}},
        };
        candidates.set(id, {
            id, hosts, state: {contentLayerId: id, viewportWidth: 100, viewportHeight: 100,
                lastMeasuredScrollX: 0, lastMeasuredScrollY: 0,
                lastMeasuredPresentationScrollX: 0, lastMeasuredPresentationScrollY: 0},
            surfaceCost, mandatory, distance,
            record: {presentedBounds: {left: 0, top: distance, right: 100, bottom: distance + 100}},
            visibleBounds: {left: 0, top: 0, right: distance === 0 ? 100 : 0, bottom: 100},
            parentId: mandatory ? undefined : 1, intent: distance === 0 ? 'active' : 'prewarm',
        });
    }
    add(1, 4, true, 0);
    // A long page can warm more surfaces than the pool holds. Numeric layer
    // order is deliberately opposite visual order, as in Paxflix's shelves.
    for (let row = 0; row < 21; row++) add(22 - row, 3, false, row * 345);
    Object.assign(pool, {
        mount: {dataset: {}}, scrollerHosts: new Map([...candidates].map(([id, c]) => [id, c.hosts])),
        scrollerMeasurementStates: new Map(),
        buildWarmScrollerCandidates: () => candidates,
        estimateWarmLayerCanvasCount: () => 1,
        surfaceRefreshPending: false,
        canvases: new Map(), layerCanvasPlanCache: new Map(),
        layers: {layerIds: () => [0, ...candidates.keys()], estimateScrollerIslandOwnedLayers: id => [id]},
        // This admission-only harness uses the legacy observation bridge; the
        // production v2 bridge consumes the separate preparation transaction.
        chassis: {schedule_surfaces: snapshot => sharedSurfaceDecision({...snapshot,version:1,
            hosts: [...candidates.values()].map(c => ({id:c.id,parent:c.parentId ?? null,
                bounds:Object.values(c.record.presentedBounds),visible:Object.values(c.visibleBounds),
                viewport:[100,100],offset:[0,0],observed:[0,0],basis:[1,0,0,1],
                active:c.intent === 'active',geometry:'same'}))}), get_layer_canvas_plan_for_transform: layer => ({layerId: layer, active: true,
            surfaces: Array.from({length: candidates.get(layer)?.surfaceCost ?? 1}, (_, i) => ({
                id: `${layer}:${i}`, key: String(i), left: i * 100, top: 0, width: 100, height: 100,
                hostSignature: `scroller:${layer}`, surfaceSignature: '100', transformSignature: `${i * 100},0`,
                replayPriority: i,
            }))})},
    });
    return {pool, candidates, prototype: module.exports.NativeElementPool.prototype,
        cost: () => [...pool.admittedPlans.values()].reduce((sum, p) => sum + p.surfaces.length, 0)};
}

for (const ios of [false, true]) {
    test(`${ios ? 'iOS' : 'desktop'} warm rows share the pool budget and prioritize nearby content`, () => {
        const h = harness(ios);
        h.pool.syncWarmScrollerHosts();
        assert.ok(h.cost() <= 64, `selected ${h.cost()} surfaces for a 64-canvas pool`);
        assert.equal(h.candidates.get(1).hosts.warmed, true, 'retain the ancestor scroller');
        assert.equal(h.candidates.get(22).hosts.warmed, true, 'the visible first shelf must fit');
        assert.equal(h.candidates.get(2).hosts.warmed, false, 'evict the farthest shelf first');

        // Scroll to the bottom, then back. Previously warmed rows must yield to
        // the new visible region instead of reserving capacity indefinitely.
        for (const atBottom of [true, false]) {
            for (const c of h.candidates.values()) {
                if (c.mandatory) continue;
                c.distance = (atBottom ? c.id - 2 : 22 - c.id) * 345;
                c.intent = c.distance < 1500 ? 'active' : 'cold';
                c.visibleBounds.right = c.distance === 0 ? 100 : 0;
                c.record.presentedBounds.top = c.distance;
                c.record.presentedBounds.bottom = c.distance + 100;
            }
            h.pool.syncWarmScrollerHosts();
            assert.ok(h.cost() <= 64);
            assert.equal(h.candidates.get(atBottom ? 2 : 22).hosts.warmed, true);
            assert.equal(h.candidates.get(atBottom ? 22 : 2).hosts.warmed, false);
        }
    });
}

test('visible nested shelves outrank closer warm-only candidates', () => {
    const h = harness(false);
    const visible = h.candidates.get(2);
    visible.visibleBounds = {left: 0, top: 0, right: 100, bottom: 100};
    visible.distance = 10000;
    visible.record.presentedBounds = visible.visibleBounds;
    h.pool.syncWarmScrollerHosts();
    assert.equal(visible.hosts.warmed, true);
    assert.ok(h.cost() <= 64);
});

test('visible native-only scrollers need no canvas allocation to remain visible', () => {
    const h = harness(false);
    const host = h.candidates.get(22);
    host.surfaceCost = 0;
    host.hosts.warmed = false;
    host.hosts.parked = true;
    host.hosts.contentHost.style.visibility = 'hidden';
    h.pool.syncWarmScrollerHosts();
    assert.equal(host.hosts.parked, false);
    assert.equal(host.hosts.contentHost.style.visibility, '');
    assert.equal(h.pool.admittedPlans.has(22), false);
});

test('admitted warm rows stay presentable without waiting for a visibility tick', () => {
    const h = harness(false);
    const host = h.candidates.get(21);
    host.hosts.parked = true;
    host.hosts.canvasHost.style.visibility = 'hidden';
    host.hosts.contentHost.style.visibility = 'hidden';
    h.pool.syncWarmScrollerHosts();
    assert.equal(host.intent, 'prewarm');
    assert.equal(host.hosts.parked, false);
    assert.equal(host.hosts.canvasHost.style.visibility, '');
    assert.equal(host.hosts.contentHost.style.visibility, '');
    assert.equal(host.hosts.canvasHost.dataset.renderState, 'active');
    assert.equal(host.hosts.contentHost.dataset.renderState, 'active');
    const cold = h.candidates.get(2);
    assert.equal(cold.hosts.parked, true, 'denied content remains parked');
    assert.equal(cold.hosts.canvasHost.style.visibility, 'hidden');
});

test('zero-sized SVG clip wrappers use their path bounds, not an empty DOM rect', () => {
    const h = harness(false);
    const wrapper = {style: {clipPath: 'url(#frame)', overflow: '', overflowX: '', overflowY: ''},
        dataset: {clipBounds: JSON.stringify({left: 100, top: 200, right: 600, bottom: 500})},
        getBoundingClientRect: () => ({left: 0, top: -50, right: 0, bottom: -50}), parentElement: h.pool.mount};
    const leaf = {getBoundingClientRect: () => ({left: 100, top: 150, right: 600, bottom: 450}),
        parentElement: wrapper};
    const result = h.prototype.getPresentationRecordForLeaf.call(h.pool, 42, leaf);
    assert.deepEqual(JSON.parse(JSON.stringify(result.presentedClipBounds)),
        {left: 100, top: 150, right: 600, bottom: 450});
});

test('shared preparation measures presentation clips without independently padding nested viewports', () => {
    const h = harness(false);
    const rect = (left, top, right, bottom) => ({left, top, right, bottom});
    const states = new Map();
    const hosts = new Map();
    const records = new Map();
    function add(id, parentId, bounds, localWidth, localHeight) {
        hosts.set(id, {id, parentId, vectorIslandEnabled: true, canvasHost: {dataset: {}}, leaf: {clientWidth: localWidth, clientHeight: localHeight}});
        states.set(id, {isRootScroller: id === 1, lastWarmAt: -10000,
            viewportWidth: localWidth, viewportHeight: localHeight, contentWidth: localWidth, contentHeight: 10000,
            lastMeasuredPresentationScrollX:0,lastMeasuredPresentationScrollY:0,lastMeasuredScrollX:0,lastMeasuredScrollY:0});
        records.set(id, {presentedBounds: bounds, presentedClipBounds: rect(0, 0, 1000, 900)});
    }
    add(1, undefined, rect(0, 0, 1000, 900), 1000, 900);
    add(2, 1, rect(100, 100, 600, 450), 1000, 700); // 0.5x embedding
    add(3, 2, rect(100, 900, 600, 1000), 1000, 200);
    add(4, 2, rect(100, 200, 600, 300), 1000, 200);
    add(5, 3, rect(100, 900, 500, 950), 800, 100);
    Object.assign(h.pool, {
        scrollerHosts: hosts, scrollerMeasurementStates: states,
        getWarmthPresentationRecord: id => records.get(id),
        getParentScrollerId: host => host.parentId,
        getIntermediateScrollerClip: () => undefined,
        estimateScrollerWarmSurfaceCost: () => 1,
        chassis: {prepare_surface_hosts: sharedSurfaceDecision},
    });
    const candidates = h.prototype.buildWarmScrollerCandidates.call(h.pool, 10000);
    assert.equal(candidates.get(3).intent, 'cold', 'stationary ancestors do not grant an independent child runway');
    assert.equal(candidates.get(4).visibleBounds.top, 200);
    assert.ok(candidates.get(5).visibleBounds.bottom <= candidates.get(5).visibleBounds.top);
    assert.equal(candidates.get(4).coverageKey,'|0,0,1000,400',
        'the actually visible child may enumerate its own bounded retention margin');
    const canvas={dataset:{surfaceContentRevision:'a',surfaceSubmittedRevision:'a',surfaceGeneration:'1',surfacePreparedAt:'9900'}};
    h.pool.canvases.set('test',canvas);
    h.prototype.buildWarmScrollerCandidates.call(h.pool, 10000);
    assert.equal(canvas.dataset.surfacePreparationLatency,'100');
    h.prototype.buildWarmScrollerCandidates.call(h.pool, 12000);
    assert.equal(canvas.dataset.surfacePreparationLatency,'100','idle time is not another latency sample');
    const events = [];
    h.pool.surfaceTrace = {record: (event, fields) => events.push({event, ...fields})};
    states.get(2).viewportWidth = -28;
    const rejected = h.prototype.buildWarmScrollerCandidates.call(h.pool, 12010);
    assert.equal(rejected.get(1).intent, 'active', 'invalid child does not stop its parent');
    assert.equal(rejected.get(2).intent, 'cold');
    assert.equal(rejected.get(4).intent, 'cold', 'invalid ancestry cannot grant child coverage');
    assert.ok(events.some(e => e.id === 2 && e.reason === 'negative surface extent'));
    h.prototype.buildWarmScrollerCandidates.call(h.pool, 12020);
    assert.equal(events.length, 4, 'unchanged branch rejection is not traced every frame');
    states.get(2).viewportWidth = 1000;
    const recovered = h.prototype.buildWarmScrollerCandidates.call(h.pool, 12030);
    assert.equal(recovered.get(4).intent, 'active');
    assert.equal(hosts.get(2).canvasHost.dataset.preparationRejection, undefined);
    assert.ok(events.some(e => e.id === 2 && e.event === 'preparation-branch-recovered'));
});
