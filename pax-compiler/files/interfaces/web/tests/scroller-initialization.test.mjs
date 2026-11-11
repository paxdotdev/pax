import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function load(path, require = () => ({})) {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(readFileSync(new URL(path, import.meta.url), 'utf8'),
        {loader: 'ts', format: 'cjs'}).code,
        {module, exports: module.exports, require, console, window: {}});
    return module.exports;
}
const helpers = load('../src/utils/helpers.ts');
const placement = load('../src/classes/native-placement.ts', () => helpers);
const {StartupWork} = load('../src/utils/startup-scheduler.ts');

function harness(ios = false, delegated = false) {
    const {NativeElementPool} = load('../src/classes/native-element-pool.ts', name => {
        if (name === '../utils/helpers') return helpers;
        if (name === './native-placement') return placement;
        if (name === './surface-host-policy') return {isIOSWebKitBrowser: () => ios};
        if (name === './render-layer-context') return {setLeafLocalOpacity() {}};
        return {};
    });
    function element() {
        let x = 0, y = 0;
        const range = axis => Math.max(0, (parseFloat(node.children[0]?.style[axis]) || 0)
            - (parseFloat(node.style[axis]) || 0));
        const node = {style: {setProperty(k,v) {this[k]=v;}}, dataset: {}, attrs: {}, children: [],
            setAttribute(k, v) {this.attrs[k] = v;}, getAttribute(k) {return this.attrs[k];},
            addEventListener() {},
            appendChild(child) {this.children.push(child); child.parentElement = this;},
            removeChild(child) {this.children.splice(this.children.indexOf(child), 1); child.parentElement = null;},
            // Model browser clamping: an offset applied before extent configuration is lost.
            get scrollLeft() {return x;}, set scrollLeft(v) {x = Math.min(range('width'), Math.max(0, v));},
            get scrollTop() {return y;}, set scrollTop(v) {y = Math.min(range('height'), Math.max(0, v));},
            getBoundingClientRect() {throw new Error('initialization must not depend on onscreen visibility');},
        };
        return node;
    }
    const root = element(), feedback = [], placements = [];
    const pool = Object.create(NativeElementPool.prototype);
    const hosts = leaf => pool.scrollerHosts.get(Number(leaf.getAttribute('pax_id')));
    Object.assign(pool, {
        nodesLookup: new Map(), scrollerHosts: new Map(), scrollerMeasurementStates: new Map(),
        pendingScrollerUpdates: new Map(), presentationRecords: new Map(),
        objectManager: {getFromPool: element},
        chassis: {interrupt: m => feedback.push(JSON.parse(JSON.stringify(m.ScrollerPosition)))},
        layers: {
            removeElementMask() {},
            addElement: leaf => root.appendChild(leaf), registerParentFrameHost() {},
            registerScrollerIslandHosts: () => false, unregisterScrollerIslandHosts: () => false,
            unregisterParentFrameHost() {}, claimLayerForScrollerIsland: () => false,
        },
        getScrollerInnerPane: leaf => hosts(leaf)?.innerPane,
        getScrollerSnapHost: leaf => hosts(leaf)?.snapHost,
        getScrollerCanvasHost: leaf => hosts(leaf)?.canvasHost,
        getScrollerContentHost: leaf => hosts(leaf)?.contentHost,
        updatePresentationRecord() {}, applyLeafPlacement: (leaf, patch) => placements.push({...patch}),
        shouldDelegateToPageScroll: () => delegated,
        setPageScrollDelegation(id, leaf, state, active, _w, _h, _contentH, x, y) {
            const activating = active && !state.delegatesToPageScroll;
            state.delegatesToPageScroll = active;
            if (activating) {
                leaf.scrollLeft = x; leaf.scrollTop = y;
                // The real delegation path can report synchronously, before both
                // initial position commands finish. It shares the readiness gate.
                this.emitScrollerPosition(id, {scrollX: x, scrollY: y,
                    presentationScrollX: x, presentationScrollY: y}, state);
            }
        },
        readPageScrollPosition() {const leaf = pool.nodesLookup.get(1); return {scrollX: leaf.scrollLeft, scrollY: leaf.scrollTop};},
        setPageScrollPosition(x, y) {const leaf = pool.nodesLookup.get(1);
            if (x != null) leaf.scrollLeft = x; if (y != null) leaf.scrollTop = y;},
        syncDelegatedPageScrollViewport() {}, syncScrollerSnap() {}, invalidateSurfaceHostObservation() {},
    });
    return {pool, feedback, placements,
        create: (id = 1) => pool.scrollerCreate({id, renderLayerId: 0, parentFrame: delegated ? null : 99}),
        update: patch => pool.scrollerUpdate({id: 1, ...patch}),
        sample: () => pool.sampleFrameInputs(), observe: () => pool.observeFrameInputs(),
        leaf: (id = 1) => pool.nodesLookup.get(id),
    };
}
const geometry = {sizeX: 294, sizeY: 140.8, sizeInnerPaneX: 8192, sizeInnerPaneY: 8192};
const initial = {...geometry, scrollX: 3949, scrollY: 4025.6};
const positions = h => h.feedback.map(p => [p.id, p.scroll_x, p.scroll_y]);

for (const ios of [false, true]) {
    const platform = ios ? 'iOS polling' : 'desktop';
    test(`${platform}: observation coalesces reversals and publishes the final idle position`, () => {
        const h = harness(ios); h.create(); h.update(initial); h.sample(); h.feedback.length = 0;
        let invalidations = 0;
        h.pool.invalidateSurfaceHostObservation = () => invalidations++;
        for (const [x, y] of [[4200, 4500], [3800, 3900], [4100, 4300]]) {
            h.leaf().scrollLeft = x; h.leaf().scrollTop = y;
            h.pool.activateScrollerMeasurement(1); h.observe();
            const state = h.pool.scrollerMeasurementStates.get(1);
            assert.equal(state.lastMeasuredPresentationScrollX, x);
            assert.equal(state.lastMeasuredPresentationScrollY, y);
            assert.deepEqual(h.feedback, [], 'presentation observation must not run scroll handlers');
        }
        assert.equal(invalidations, 3, 'each movement invalidates planning geometry, including reversal');
        h.observe(); h.observe(); h.observe();
        assert.equal(h.pool.scrollerMeasurementStates.get(1).active, false);
        h.sample(); h.sample();
        assert.deepEqual(positions(h), [[1, 4100, 4300]], 'idle does not discard an unpublished position');
    });

    test(`${platform}: continuation observations preserve initialization and remount boundaries`, () => {
        const h = harness(ios); h.create();
        h.observe(); h.observe(); h.sample();
        assert.deepEqual(h.feedback, []);
        h.update(initial);
        h.observe(); h.observe(); h.observe();
        assert.deepEqual(h.feedback, []);
        h.sample();
        assert.deepEqual(positions(h), [[1, 3949, 4025.6]]);
        h.leaf().scrollTop = 4500; h.pool.activateScrollerMeasurement(1); h.observe();
        h.pool.scrollerDelete(1); h.feedback.length = 0;
        h.create(); h.update({...initial, scrollY: 100}); h.sample();
        assert.deepEqual(positions(h), [[1, 3949, 100]]);
    });

    test(`${platform}: authored commands and content rebases supersede unpublished observations`, () => {
        const h = harness(ios); h.create(); h.update(initial); h.sample(); h.feedback.length = 0;
        h.leaf().scrollLeft = 4200; h.pool.activateScrollerMeasurement(1); h.observe();
        h.update({scrollX: 100, contentOriginX: 4000}); h.observe();
        assert.deepEqual(h.feedback, []);
        const state = h.pool.scrollerMeasurementStates.get(1);
        assert.equal(state.lastMeasuredPresentationScrollX, 4100);
        h.sample(); h.sample();
        assert.deepEqual(positions(h), [[1, 100, 4025.6]]);
        assert.equal(h.feedback[0].presentation_scroll_x, 4100);
    });

    test(`${platform}: delegated viewport observation changes demand without engine feedback`, () => {
        const h = harness(ios, true); h.create(); h.update(initial); h.sample(); h.feedback.length = 0;
        delete h.pool.syncDelegatedPageScrollViewport; // Exercise the real viewport method.
        h.pool.activePageScrollScrollerId = 1;
        h.pool.mount = {dataset: {}};
        let height = 140.8;
        h.pool.readVisiblePageViewport = () => ({width: 294, height, offsetX: 0, offsetY: 12});
        const interrupts = [];
        h.pool.chassis.interrupt = m => interrupts.push(m);
        h.leaf().scrollTop = 4200; h.pool.activateScrollerMeasurement(1); h.observe();
        height = 170; h.observe(); h.observe();
        assert.equal(h.pool.mount.dataset.viewportHeight, '170');
        assert.equal(h.pool.scrollerHosts.get(1).canvasHost.dataset.approvedScrollY, '4200');
        assert.deepEqual(interrupts, [], 'even visual viewport feedback waits for an application frame');
        h.sample();
        assert.equal(interrupts[0].VisualViewportUpdate.height, 170);
        assert.equal(interrupts[0].VisualViewportUpdate.page_scroll_y, 4200);
        assert.equal(interrupts[1].ScrollerPosition.scroll_y, 4200);
    });

    for (const delegated of [false, true]) {
        test(`${platform}, ${delegated ? 'page' : 'nested'}: create/configure yield preserves both initial offsets`, () => {
            const h = harness(ios, delegated);
            h.create();
            for (let i = 0; i < 4; i++) h.sample();
            assert.deepEqual(h.feedback, []);
            h.update(initial);
            assert.deepEqual(h.feedback, [], 'configuration-time callbacks are not observations yet');
            h.sample();
            assert.deepEqual(positions(h), [[1, 3949, 4025.6]]);
            h.leaf().scrollTop = 4100;
            h.pool.activateScrollerMeasurement(1); h.sample();
            assert.deepEqual(positions(h).at(-1), [1, 3949, 4100]);
            h.update({presentationScrollY: 4050});
            assert.equal(h.leaf().scrollTop, 4100, 'observations still cannot rewind live scrolling');
            h.update({scrollY: 4200}); h.sample();
            assert.deepEqual(positions(h).at(-1), [1, 3949, 4200]);
        });
    }
    test(`${platform}: fragmented configuration retains commands until geometry and both axes arrive`, () => {
        const h = harness(ios); h.create();
        const first = {scrollX: 3000, transform: [1, 0, 0, 1, 10, 20], parentFrame: 99, zIndex: 2};
        h.update(first);
        first.scrollX = 0; first.transform[4] = 999; // Returned pooled patch is reused.
        for (const patch of [{scrollX: 3949}, {sizeX: 294}, {sizeY: 140.8},
            {sizeInnerPaneX: 8192}, {sizeInnerPaneY: 8192}, {parentFrame: null, zIndex: 0}]) {
            h.update(patch); h.sample(); assert.deepEqual(h.feedback, []);
        }
        h.update({scrollY: 4025.6}); h.sample();
        assert.deepEqual(positions(h), [[1, 3949, 4025.6]]);
        assert.equal(h.placements[0].parentFrame, null);
        assert.equal(h.placements[0].zIndex, 0);
        assert.equal(h.placements[0].transform[4], 10);
        assert.equal(h.pool.pendingScrollerUpdates.size, 0);
    });
    test(`${platform}: queued update before create and deletion/remount have independent readiness`, () => {
        const h = harness(ios);
        h.update(initial); h.create(); h.sample();
        assert.deepEqual(positions(h), [[1, 3949, 4025.6]]);
        h.pool.scrollerDelete(1); h.feedback.length = 0;
        h.create(); h.sample(); assert.deepEqual(h.feedback, []);
        h.update({scrollX: 600});
        h.pool.scrollerDelete(1);
        assert.equal(h.pool.pendingScrollerUpdates.size, 0, 'delete discards an unfinished initialization');
        h.create(); h.sample(); assert.deepEqual(h.feedback, []);
        h.update({...initial, scrollX: 0, scrollY: 0}); h.sample();
        assert.deepEqual(positions(h), [[1, 0, 0]], 'no command or readiness leaks across remount');
    });
}

test('an unconfigured host does not block an initialized sibling', () => {
    const h = harness(); h.create(); h.update(initial); h.sample(); h.feedback.length = 0;
    h.create(2); h.leaf().scrollLeft = 4000; h.pool.activateScrollerMeasurement(1); h.sample();
    assert.deepEqual(positions(h), [[1, 4000, 4025.6]]);
});

test('explicit zero geometry and zero offsets complete initialization', () => {
    const h = harness(); h.create();
    h.update({sizeX: 0, sizeY: 0, sizeInnerPaneX: 0, sizeInnerPaneY: 0, scrollX: 0, scrollY: 0});
    h.sample(); assert.deepEqual(positions(h), [[1, 0, 0]]);
    h.update(initial); h.sample();
    assert.deepEqual(positions(h).at(-1), [1, 3949, 4025.6]);
});

test('the startup FIFO may yield between native create and update without resetting application state', () => {
    const h = harness(); let now = 0, issued = false;
    const work = new StartupWork(() => issued && !work.patchesPending,
        () => {issued = true; return ['create', 'update'];},
        op => {if (op === 'create') h.create(); else h.update(initial); now += 2;},
        () => now);
    assert.equal(work.service(1), true);
    h.sample(); assert.deepEqual(h.feedback, []);
    assert.equal(work.service(1), false);
    h.sample(); assert.deepEqual(positions(h), [[1, 3949, 4025.6]]);
});
