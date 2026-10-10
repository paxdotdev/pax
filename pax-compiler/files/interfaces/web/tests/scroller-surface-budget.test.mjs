import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8');

function harness(ios) {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports,
        require: () => ({isIOSWebKitBrowser: () => ios}),
    });
    const pool = Object.create(module.exports.NativeElementPool.prototype);
    const candidates = new Map();
    function add(id, surfaceCost, mandatory, distance) {
        const hosts = {
            id, warmed: true, parked: false, vectorIslandEnabled: true,
            canvasHost: {dataset: {warmState: 'warm', renderState: 'active'}, style: {}},
            contentHost: {dataset: {renderState: 'active'}, style: {}},
        };
        candidates.set(id, {
            id, hosts, state: {}, surfaceCost, mandatory, distance,
            visibleBounds: {left: 0, top: 0, right: distance === 0 ? 100 : 0, bottom: 100},
            parentId: mandatory ? undefined : 1, intent: 'active',
        });
    }
    add(1, 4, true, 0);
    // A long page can warm more surfaces than the pool holds. Numeric layer
    // order is deliberately opposite visual order, as in Paxflix's shelves.
    for (let row = 0; row < 21; row++) add(22 - row, 3, false, row * 345);
    Object.assign(pool, {
        mount: {}, scrollerHosts: new Map([...candidates].map(([id, c]) => [id, c.hosts])),
        scrollerMeasurementStates: new Map(),
        buildWarmScrollerCandidates: () => candidates,
        estimateWarmLayerCanvasCount: () => 1,
        surfaceRefreshPending: false,
    });
    return {pool, candidates, prototype: module.exports.NativeElementPool.prototype, cost: () => 1 + [...candidates.values()]
        .filter(c => c.hosts.warmed).reduce((sum, c) => sum + c.surfaceCost, 0)};
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
    h.pool.syncWarmScrollerHosts();
    assert.equal(visible.hosts.warmed, true);
    assert.ok(h.cost() <= 64);
});

test('nested warm runways use presentation-space dimensions and visibility intersects ancestors', () => {
    const h = harness(false);
    const rect = (left, top, right, bottom) => ({left, top, right, bottom});
    const states = new Map();
    const hosts = new Map();
    const records = new Map();
    function add(id, parentId, bounds, localWidth, localHeight) {
        hosts.set(id, {id, parentId, vectorIslandEnabled: true, leaf: {clientWidth: localWidth, clientHeight: localHeight}});
        states.set(id, {isRootScroller: id === 1, lastWarmAt: -10000});
        records.set(id, {presentedBounds: bounds, presentedClipBounds: rect(0, 0, 1000, 900)});
    }
    add(1, undefined, rect(0, 0, 1000, 900), 1000, 900);
    add(2, 1, rect(100, 100, 600, 450), 1000, 700); // 0.5x embedding
    add(3, 2, rect(100, 2200, 600, 2300), 1000, 200);
    add(4, 2, rect(100, 200, 600, 300), 1000, 200);
    add(5, 3, rect(100, 2200, 500, 2250), 800, 100);
    Object.assign(h.pool, {
        scrollerHosts: hosts, scrollerMeasurementStates: states,
        getWarmthPresentationRecord: id => records.get(id),
        getParentScrollerId: host => host.parentId,
        estimateScrollerWarmSurfaceCost: () => 1,
    });
    const candidates = h.prototype.buildWarmScrollerCandidates.call(h.pool, 10000);
    assert.equal(candidates.get(3).intent, 'prewarm', 'do not expand 700 local px as 700 screen px');
    assert.equal(candidates.get(4).visibleBounds.top, 200);
    assert.ok(candidates.get(5).visibleBounds.bottom <= candidates.get(5).visibleBounds.top);
    for (const [width, height] of [[200, 150], [2000, 1400], [700, 1000]]) {
        candidates.get(2).record.presentedBounds = rect(0, 0, width, height);
        const base = h.pool.getWarmClipBaseForCandidate(candidates.get(4), candidates);
        assert.equal(base.viewportWidth, width);
        assert.equal(base.viewportHeight, height);
    }
});
