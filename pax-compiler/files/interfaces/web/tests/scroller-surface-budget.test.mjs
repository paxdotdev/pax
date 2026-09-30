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
    return {pool, candidates, cost: () => 1 + [...candidates.values()]
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
            }
            h.pool.syncWarmScrollerHosts();
            assert.ok(h.cost() <= 64);
            assert.equal(h.candidates.get(atBottom ? 2 : 22).hosts.warmed, true);
            assert.equal(h.candidates.get(atBottom ? 22 : 2).hosts.warmed, false);
        }
    });
}
