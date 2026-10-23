import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function load(path, require = () => ({})) {
    const module = {exports: {}};
    const source = readFileSync(new URL(path, import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code,
        {module, exports: module.exports, require, window:{innerWidth:1000,innerHeight:900}});
    return module.exports;
}
const helpers = load('../src/utils/helpers.ts');
const {NativeElementPool} = load('../src/classes/native-element-pool.ts', path =>
    path === '../utils/helpers' ? helpers : {isIOSWebKitBrowser: () => false});
const {NativeElementPool: IOSNativeElementPool} = load('../src/classes/native-element-pool.ts', path =>
    path === '../utils/helpers' ? helpers : {isIOSWebKitBrowser: () => true});

test('compositing diagnostics report work deltas without attributing old rebuild causes to a new pass', () => {
    const pool = Object.create(NativeElementPool.prototype);
    const events = [];
    pool.surfaceTrace = {record: (kind, data) => events.push({kind, data})};
    const first = {passes:4, rebuilds:2, structural_reconciliations:1, records_retained:2000, orders_updated:1000, records_updated:3000, masks_evaluated:500,
        mask_candidates:900, rebuild_reason:'structure', rebuild_node:null,
        rebuild_origins:[{file:'scene.rs',line:12,node:10,count:2}]};
    pool.recordSurfaceCompositing(first);
    pool.recordSurfaceCompositing({...first,passes:5,records_updated:3004,masks_evaluated:502,mask_candidates:906});
    assert.equal(events.length, 2);
    const second = events[1].data;
    assert.deepEqual(JSON.parse(JSON.stringify(second.delta)), {
        passes:1,rebuilds:0,structural_reconciliations:0,records_retained:0,
        topology_nodes_visited:0,topology_nodes_reused:0,
        orders_updated:0,records_updated:4,masks_evaluated:2,mask_candidates:6,
    });
    assert.equal(second.rebuild_reason, null);
    assert.equal(second.rebuild_origins.length, 0);
    pool.recordSurfaceCompositing({...first,passes:6,structural_reconciliations:2,
        records_retained:4000,orders_updated:1900,rebuild_reason:'structure-reconciled'},
        [['property_effects', 12], ['occlusion.topology', 2], ['occlusion.coverage', 0.5], ['occlusion.masks', 1], ['occlusion', 0]]);
    assert.equal(events[2].data.delta.structural_reconciliations, 1);
    assert.equal(events[2].data.delta.records_retained, 2000);
    assert.equal(events[2].data.delta.orders_updated, 900);
    assert.equal(events[2].data.rebuild_reason, 'structure-reconciled');
    assert.equal(events[2].data.rebuild_origins.length, 1);
    assert.deepEqual(JSON.parse(JSON.stringify(events[2].data.phases)),
        [['occlusion.topology', 2], ['occlusion.coverage', 0.5], ['occlusion.masks', 1], ['occlusion', 0]]);
});

test('planning measures a shared clip once per pass, never reuses it across passes', () => {
    const pool = Object.create(NativeElementPool.prototype);
    const mount = {};
    let reads = 0, left = 0;
    const ancestor = {parentElement:mount, style:{overflow:'hidden'}, dataset:{},
        getBoundingClientRect: () => { reads++; return {left,top:0,right:500,bottom:500}; }};
    const hosts = new Map([1,2].map(id => [id, {id, canvasHost:{dataset:{}},
        leaf:{parentElement:ancestor, clientWidth:100, clientHeight:100,
            getBoundingClientRect: () => ({left:0,top:0,right:100,bottom:100})}}]));
    const observations = [];
    Object.assign(pool, {mount, scrollerHosts:hosts, canvases:new Map(),
        scrollerMeasurementStates:new Map([1,2].map(id=>[id,{}])),
        getParentScrollerId:()=>undefined,
        chassis:{prepare_surface_hosts: snapshot => { observations.push(snapshot); return []; }}});
    pool.buildWarmScrollerCandidates(0);
    assert.equal(reads, 1, 'both visible and preparation clips share the same observation');
    left = 20;
    pool.buildWarmScrollerCandidates(16);
    assert.equal(reads, 2, 'browser-only geometry changes are read next time');
    assert.equal(observations[1].hosts[0].intermediateClip[0], 20);
});

function harness(delegated = false, Pool = NativeElementPool) {
    const element = () => ({style: {}, dataset: {}, scrollLeft: 0, scrollTop: 0});
    const leaf = element(), pane = element(), canvas = element(), content = element();
    const state = {};
    const pool = Object.create(Pool.prototype);
    Object.assign(pool, {
        nodesLookup: new Map([[1, leaf]]), scrollerHosts: new Map(),
        scrollerMeasurementStates: new Map([[1, state]]),
        getScrollerInnerPane: () => pane, getScrollerSnapHost: element,
        getScrollerCanvasHost: () => canvas, getScrollerContentHost: () => content,
        updatePresentationRecord() {}, applyLeafPlacement() {},
        shouldDelegateToPageScroll: () => delegated,
        setPageScrollDelegation() {}, syncScrollerSnap() {}, activateScrollerMeasurement() {},
        readPageScrollPosition: () => ({scrollX: leaf.scrollLeft, scrollY: leaf.scrollTop}),
        setPageScrollPosition: (x, y) => {
            if (x != null) leaf.scrollLeft = x;
            if (y != null) leaf.scrollTop = y;
        },
    });
    return {pool, leaf, pane, canvas, content, state, update: patch => pool.scrollerUpdate({id: 1, ...patch})};
}

test('pane recenter preserves content-space scroll and canvas/native alignment', () => {
    const h = harness();
    const transform = [0.5, 0, 0, 0.75, 20, 90];
    h.update({sizeX:1800, sizeY:534, sizeInnerPaneX:50046, sizeInnerPaneY:534,
        transform, scrollX:42000.25, contentOriginX:0});
    const inverse = h.content.style.transform;
    const measure = () => h.pool.measureScrollerPosition(h.leaf, h.state);
    const before = measure();
    for (const origin of [16082, 0, -16082, 32164]) {
        h.update({scrollX:42000.25 - origin, contentOriginX:origin});
        assert.equal(measure().presentationScrollX, before.presentationScrollX);
        assert.equal(h.canvas.style.translate, origin === 0 ? '' : `${-origin}px 0px`);
        assert.equal(h.content.style.translate, h.canvas.style.translate);
        assert.equal(h.content.style.transform, inverse, 'origin does not replace inverse world transform');
        assert.equal(h.canvas.dataset.approvedScrollX, '42000.25');
        assert.equal(h.pane.style.width, '50046px', 'finite native scroll extent stays fixed');
    }
    h.leaf.scrollLeft += 137.5;
    assert.equal(measure().presentationScrollX, 42137.75, 'subsequent input uses the new origin');
    h.update({presentationScrollX:42100});
    assert.equal(measure().presentationScrollX, 42137.75, 'observation does not rewind native input');
});

test('queued origin and scroll deltas are replayed as one patch', () => {
    const h = harness();
    h.pool.pendingScrollerUpdates = new Map();
    h.pool.queueScrollerUpdate({id:1, contentOriginX:16082, scrollX:25918.25});
    h.pool.queueScrollerUpdate({id:1, opacity:0.8});
    const queued = h.pool.pendingScrollerUpdates.get(1);
    assert.equal(queued.contentOriginX, 16082);
    assert.equal(queued.scrollX, 25918.25);
});

for (const [platform, Pool] of [['desktop', NativeElementPool], ['iOS', IOSNativeElementPool]]) {
  for (const delegated of [false, true]) {
    test(`${platform}: native observation cannot rewind moving ${delegated ? 'page' : 'nested'} scroller`, () => {
        const h = harness(delegated, Pool);
        h.state.active = true;
        h.leaf.scrollLeft = 55;
        h.leaf.scrollTop = 120;
        h.update({presentationScrollX: 40, presentationScrollY: 100});
        assert.equal(h.leaf.scrollLeft, 55);
        assert.equal(h.leaf.scrollTop, 120);
        assert.equal(h.canvas.dataset.approvedScrollY, '100');
        // Intentional commands work during motion, on either axis.
        h.update({scrollX: 200, presentationScrollX: 200});
        assert.equal(h.leaf.scrollLeft, 200);
        assert.equal(h.leaf.scrollTop, 120);
        h.update({scrollY: 600, presentationScrollY: 600});
        assert.equal(h.leaf.scrollTop, 600);
    });
  }
}

for (const transform of [[0.5, 0, 0, 0.5, 65, 1200], [2, 0, 0, 2, 40, 80],
    [0.5, 0, 0, 0.75, 20, 90], [0, 0.5, -0.5, 0, 400, 80]]) {
    test(`transformed native content cannot enlarge the declared pane: ${transform}`, () => {
        const h = harness();
        h.update({sizeX: 1024, sizeY: 255, sizeInnerPaneX: 3160, sizeInnerPaneY: 255, transform});
        assert.equal(h.pane.style.overflow, 'clip');
        assert.equal(h.pane.style.width, '3160px');
        assert.equal(h.leaf.style.width, '1024px');
        assert.equal(h.canvas.dataset.viewportWidth, '1024');
        assert.equal(h.content.style.transform,
            helpers.packAffineCoeffsIntoMatrix3DString(helpers.invertAffineCoeffs(transform)));
        h.update({sizeInnerPaneY: 7772});
        assert.equal(h.pane.style.height, '7772px');
        assert.equal(h.pane.style.overflow, 'clip');
        // Do not add another scroll container or Blink-sensitive paint containment.
        assert.equal(h.pane.style.contain, undefined);
    });
}

test('unclipped hosts stay unclipped across delta updates and can resume clipping', () => {
    const h = harness();
    h.update({clipContent: false, sizeX: 200, sizeY: 100, sizeInnerPaneX: 500, sizeInnerPaneY: 800});
    h.update({sizeInnerPaneY: 900});
    assert.equal(h.pane.style.overflow, 'visible');
    assert.equal(h.leaf.style.overflowY, 'visible');
    h.update({clipContent: true});
    assert.equal(h.pane.style.overflow, 'clip');
    assert.equal(h.leaf.style.overflowY, 'auto');
});

test('page-scroll delegation keeps the whole declared content pane, not the viewport', () => {
    const h = harness(true);
    h.update({sizeX: 390, sizeY: 844, sizeInnerPaneX: 390, sizeInnerPaneY: 7772});
    assert.equal(h.pane.style.height, '7772px');
    assert.equal(h.pane.style.overflow, 'clip');
    assert.equal(h.leaf.style.overflowY, undefined, 'delegation owns the outer overflow mode');
});
