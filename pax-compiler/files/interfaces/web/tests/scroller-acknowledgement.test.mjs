import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function load(path, require = () => ({})) {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(readFileSync(new URL(path, import.meta.url), 'utf8'),
        {loader: 'ts', format: 'cjs'}).code, {module, exports: module.exports, require});
    return module.exports;
}
const helpers = load('../src/utils/helpers.ts');

// Only browser layout/snap resolution is simulated. Updates, snap configuration,
// active-frame sampling, deduplication, and interrupt emission use production code.
function harness({ios = false, delegated = false, x = 0, y = 125, max = 500} = {}) {
    const {NativeElementPool} = load('../src/classes/native-element-pool.ts', path =>
        path === '../utils/helpers' ? helpers : {isIOSWebKitBrowser: () => ios});
    const element = () => ({style: {}, dataset: {}});
    const leaf = element(), pane = element(), snap = element(), canvas = element(), content = element();
    const native = {x, y}, reads = {x: 0, y: 0}, writes = [];
    const messages = [];
    const state = {
        active: false, stableFrames: 2, delegatesToPageScroll: delegated,
        snapHostDelegatesToPageScroll: delegated,
        lastMeasuredScrollX: x, lastMeasuredScrollY: y,
        lastMeasuredPresentationScrollX: x, lastMeasuredPresentationScrollY: y,
        lastSentScrollX: x, lastSentScrollY: y,
        lastSentPresentationScrollX: x, lastSentPresentationScrollY: y,
    };
    const resolve = (value, points) => {
        value = Math.max(0, Math.min(max, value));
        return points?.length ? points.reduce((best, point) =>
            Math.abs(point - value) < Math.abs(best - value) ? point : best) : value;
    };
    for (const [property, axis, points] of [['scrollLeft', 'x', 'snapPointsX'], ['scrollTop', 'y', 'snapPointsY']]) {
        Object.defineProperty(leaf, property, {
            get() { reads[axis]++; return native[axis]; },
            set(value) { writes.push([axis, value]); native[axis] = resolve(value, state[points]); },
        });
    }
    leaf.getAttribute = () => '1';
    const pool = Object.create(NativeElementPool.prototype);
    Object.assign(pool, {
        nodesLookup: new Map([[1, leaf]]), scrollerHosts: new Map(),
        scrollerMeasurementStates: new Map([[1, state]]),
        getScrollerInnerPane: () => pane, getScrollerSnapHost: () => snap,
        getScrollerCanvasHost: () => canvas, getScrollerContentHost: () => content,
        updatePresentationRecord() {}, applyLeafPlacement() {},
        shouldDelegateToPageScroll: () => delegated, setPageScrollDelegation() {},
        syncDelegatedPageScrollViewport() {}, ensurePageSnapHost: () => null,
        readPageScrollPosition: () => ({scrollX: leaf.scrollLeft, scrollY: leaf.scrollTop}),
        setPageScrollPosition(sx, sy) {
            if (sx !== undefined) leaf.scrollLeft = sx;
            if (sy !== undefined) leaf.scrollTop = sy;
        },
        renderSnapMarkers() {
            const sx = resolve(native.x, state.snapPointsX), sy = resolve(native.y, state.snapPointsY);
            if (sx !== native.x || sy !== native.y) {
                native.x = sx; native.y = sy;
                pool.activateScrollerMeasurement(1); // browser scroll event after re-snapping
            }
        },
        chassis: {interrupt(message) { messages.push(JSON.parse(JSON.stringify(message.ScrollerPosition))); }},
    });
    const update = patch => pool.scrollerUpdate({id: 1, ...patch});
    const sample = () => pool.sampleFrameInputs();
    const settle = () => { sample(); sample(); sample(); };
    const userMove = (sx, sy) => {
        native.x = sx; native.y = sy;
        pool.activateScrollerMeasurement(1);
    };
    return {pool, leaf, native, reads, writes, messages, state, update, sample, settle, userMove};
}

test('repeated off-anchor requests acknowledge actual position even without native motion', () => {
    const h = harness();
    h.update({snapPointsY: []});
    h.update({scrollY: 173});
    h.settle();
    assert.equal(h.messages.at(-1).scroll_y, 173);
    h.update({snapPointsY: [0, 125, 250, 375, 500]});
    h.settle();
    assert.equal(h.messages.at(-1).scroll_y, 125);
    for (let request = 0; request < 3; request++) {
        const before = h.messages.length;
        h.update({scrollY: 173});
        h.settle();
        assert.equal(h.native.y, 125);
        assert.equal(h.messages.length, before + 1, 'each request needs one actual-position acknowledgement');
        assert.equal(h.messages.at(-1).scroll_y, 125);
        assert.equal(h.state.active, false);
    }
});

test('range clamping and the existing subpixel write threshold still acknowledge actual position', () => {
    for (const requested of [800, 500.2]) {
        const h = harness({y: 500});
        h.update({scrollY: requested});
        h.settle();
        assert.equal(h.messages.length, 1);
        assert.equal(h.messages[0].scroll_y, 500);
        assert.equal(h.writes.length, requested === 800 ? 1 : 0);
    }
});

test('matching echoes, presentation-only and placement patches do not add acknowledgements', () => {
    const h = harness();
    h.update({scrollY: 125});
    h.update({presentationScrollY: 173});
    h.update({sizeX: 200, sizeY: 100, sizeInnerPaneY: 900});
    h.settle();
    assert.equal(h.messages.length, 0);
    assert.equal(h.writes.length, 0);
    const reads = {...h.reads};
    for (let i = 0; i < 120; i++) h.sample();
    assert.deepEqual(h.reads, reads, 'settled desktop scrollers must not be polled');
});

test('one pending acknowledgement uses one native snapshot and retires before interrupt dispatch', () => {
    const h = harness({x: 500, y: 500});
    h.update({scrollX: 700, scrollY: 800});
    h.reads.x = h.reads.y = 0;
    h.pool.chassis.interrupt = message => {
        h.messages.push(JSON.parse(JSON.stringify(message.ScrollerPosition)));
        h.sample(); // a synchronous observer must not receive the same acknowledgement again
    };
    h.sample();
    assert.equal(h.messages.length, 1);
    assert.deepEqual(h.messages[0], {id: 1, scroll_x: 500, scroll_y: 500,
        presentation_scroll_x: 500, presentation_scroll_y: 500});
    assert.deepEqual(h.reads, {x: 2, y: 2}, 'one read per axis in each of the two samples');
});

test('active iOS and delegated page scrolling acknowledge ignored requests without cancelling inertia', () => {
    for (const options of [{ios: true}, {delegated: true}]) {
        const h = harness(options);
        h.userMove(0, 125);
        h.update({scrollY: 173});
        h.sample();
        assert.equal(h.messages.length, 1);
        assert.equal(h.messages[0].scroll_y, 125);
        assert.equal(h.writes.length, 0, 'existing browser-owned active-scroll guards remain in effect');
        h.userMove(0, 141);
        h.sample();
        h.update({scrollY: 141});
        h.userMove(0, 156);
        h.sample();
        assert.deepEqual(h.messages.map(message => message.scroll_y), [125, 141, 156]);
        assert.equal(h.writes.length, 0);
    }
});

test('continuous programmatic movement reports actual positions without extra writes or duplicate echoes', () => {
    const h = harness();
    for (const requested of [140, 155, 170, 185]) {
        h.update({scrollY: requested});
        h.sample();
        h.update({scrollY: requested});
        h.sample();
    }
    assert.deepEqual(h.messages.map(message => message.scroll_y), [140, 155, 170, 185]);
    assert.deepEqual(h.writes, [['y', 140], ['y', 155], ['y', 170], ['y', 185]]);
});

test('a logical request on either axis coalesces into one acknowledgement without waking other scrollers', () => {
    const h = harness({x: 500, y: 500});
    const otherState = {...h.state};
    h.pool.scrollerMeasurementStates.set(2, otherState);
    h.pool.nodesLookup.set(2, {get scrollLeft() { throw new Error('idle sibling read'); },
        get scrollTop() { throw new Error('idle sibling read'); }});
    h.update({scrollX: 700});
    h.update({scrollY: 800});
    h.settle();
    assert.equal(h.messages.length, 1);
    assert.equal(h.messages[0].id, 1);
    assert.equal(otherState.active, false);
});
