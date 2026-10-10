import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function load(path, require = () => ({})) {
    const module = {exports: {}};
    const source = readFileSync(new URL(path, import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code,
        {module, exports: module.exports, require});
    return module.exports;
}
const helpers = load('../src/utils/helpers.ts');
const {NativeElementPool} = load('../src/classes/native-element-pool.ts', path =>
    path === '../utils/helpers' ? helpers : {isIOSWebKitBrowser: () => false});

function harness(delegated = false) {
    const element = () => ({style: {}, dataset: {}, scrollLeft: 0, scrollTop: 0});
    const leaf = element(), pane = element(), canvas = element(), content = element();
    const state = {};
    const pool = Object.create(NativeElementPool.prototype);
    Object.assign(pool, {
        nodesLookup: new Map([[1, leaf]]), scrollerHosts: new Map(),
        scrollerMeasurementStates: new Map([[1, state]]),
        getScrollerInnerPane: () => pane, getScrollerSnapHost: element,
        getScrollerCanvasHost: () => canvas, getScrollerContentHost: () => content,
        updatePresentationRecord() {}, applyLeafPlacement() {},
        shouldDelegateToPageScroll: () => delegated,
        setPageScrollDelegation() {}, syncScrollerSnap() {}, activateScrollerMeasurement() {},
    });
    return {leaf, pane, canvas, content, state, update: patch => pool.scrollerUpdate({id: 1, ...patch})};
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
