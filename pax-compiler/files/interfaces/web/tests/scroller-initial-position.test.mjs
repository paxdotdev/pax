import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function load(path, require = () => ({})) {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(readFileSync(new URL(path, import.meta.url), 'utf8'),
        {loader: 'ts', format: 'cjs'}).code, {module, exports: module.exports, require, console});
    return module.exports;
}
const helpers = load('../src/utils/helpers.ts');

function harness(ios = false) {
    const {NativeElementPool} = load('../src/classes/native-element-pool.ts', path =>
        path === '../utils/helpers' ? helpers : {isIOSWebKitBrowser: () => ios});
    const element = () => ({
        style: {}, dataset: {}, children: [], scrollLeft: 0, scrollTop: 0,
        setAttribute() {}, addEventListener() {},
        appendChild(child) { this.children.push(child); },
    });
    const messages = [];
    const pool = Object.create(NativeElementPool.prototype);
    Object.assign(pool, {
        objectManager: {getFromPool: element},
        layers: {addElement() {}, registerParentFrameHost() {}, registerScrollerIslandHosts() {}},
        nodesLookup: new Map(), scrollerHosts: new Map(),
        scrollerMeasurementStates: new Map(), pendingScrollerUpdates: new Map(),
        chassis: {interrupt(message) { messages.push(message.ScrollerPosition); }},
        getScrollerInnerPane: leaf => leaf.children[0],
        getScrollerSnapHost: leaf => leaf.children[0].children[0],
        getScrollerCanvasHost: leaf => leaf.children[0].children[1],
        getScrollerContentHost: leaf => leaf.children[0].children[2],
        updatePresentationRecord() {}, applyLeafPlacement() {},
        shouldDelegateToPageScroll: () => false,
        setPageScrollDelegation() {}, syncScrollerSnap() {}, syncDelegatedPageScrollViewport() {},
    });
    return {pool, messages, create: () => pool.scrollerCreate({id: 1, renderLayerId: 0}),
        update: patch => pool.scrollerUpdate({id: 1, ...patch})};
}

const initial = {sizeX: 360, sizeY: 120, sizeInnerPaneX: 8192, sizeInnerPaneY: 8192,
    scrollX: 3916, scrollY: 4036};

for (const ios of [false, true]) {
    test(`yielded creation cannot overwrite the authored center (iOS=${ios})`, () => {
        const h = harness(ios);
        h.create();
        // A startup frame can run between ScrollerCreate and its initial ScrollerUpdate.
        for (let frame = 0; frame < 3; frame++) h.pool.sampleFrameInputs();
        assert.equal(h.messages.length, 0, 'uninitialized DOM zero is not browser input');
        h.update({sizeX: 360, sizeY: 120});
        h.pool.sampleFrameInputs();
        assert.equal(h.messages.length, 0, 'placement alone is not the initial scroll state');
        h.update(initial);
        h.pool.sampleFrameInputs();
        assert.equal(h.messages.length, 1);
        assert.equal(h.messages[0].scroll_x, 3916);
        assert.equal(h.messages[0].scroll_y, 4036);

        const leaf = h.pool.nodesLookup.get(1);
        leaf.scrollLeft += 80;
        h.pool.activateScrollerMeasurement(1);
        h.pool.sampleFrameInputs();
        assert.equal(h.messages.at(-1).scroll_x, 3996, 'normal browser input remains authoritative');
        if (ios) {
            h.update({presentationScrollX: 3916});
            assert.equal(leaf.scrollLeft, 3996, 'presentation feedback does not overwrite native momentum');
            h.update({scrollX: 3916});
            assert.equal(leaf.scrollLeft, 3916, 'explicit authored commands still apply during active iOS scrolling');
        }
        h.pool.sampleFrameInputs();
        h.pool.sampleFrameInputs();
        h.update({scrollX: 4000, scrollY: 4100});
        h.pool.sampleFrameInputs();
        assert.equal(h.messages.at(-1).scroll_x, 4000, 'later programmatic changes still apply');
    });
}

test('an update queued before creation initializes the host before its first observation', () => {
    const h = harness();
    h.update(initial);
    h.create();
    h.pool.sampleFrameInputs();
    assert.equal(h.messages.length, 1);
    assert.equal(h.messages[0].scroll_y, 4036);
});

test('zero is a valid authored starting position, not a missing initialization sentinel', () => {
    const h = harness();
    h.create();
    h.update({...initial, scrollX: 0, scrollY: 0});
    h.pool.sampleFrameInputs();
    assert.equal(h.messages.length, 1);
    assert.equal(h.messages[0].scroll_x, 0);
    assert.equal(h.messages[0].scroll_y, 0);
});
