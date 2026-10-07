import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

class Element {
    dataset = {}; style = {}; children = []; parentElement;
    appendChild(child) {
        child.parentElement?.removeChild(child);
        child.parentElement = this;
        this.children.push(child);
    }
    removeChild(child) { this.children = this.children.filter(c => c !== child); child.parentElement = undefined; }
    removeAttribute(name) { delete this.dataset[name.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase())]; }
}
function load(file) {
    const module = {exports: {}};
    const source = readFileSync(new URL(`../src/classes/${file}.ts`, import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, HTMLElement: Element, performance,
        require: () => ({releaseCanvasBacking() {}}),
    });
    return module.exports;
}
const {Layer} = load('layer');
const {RenderLayerManager} = load('render-layer-context');
const surface = (id, replayPriority = 0) => ({id, key: id, left: 0, top: 0, width: 100, height: 100, replayPriority});
const plan = (...surfaces) => ({layerId: 0, active: true, surfaces});

function harness(capacity) {
    const map = new Map(), free = [], leased = new Set();
    let allocated = 0, misses = 0;
    const pool = {
        reserveBacking: () => true,
        checkout() {
            const available = free.findIndex(canvas => !leased.has(canvas));
            if (available >= 0) return free.splice(available, 1)[0];
            if (allocated === capacity) { misses++; return null; }
            allocated++;
            return new Element();
        },
        release(canvas) { canvas.parentElement?.removeChild(canvas); free.push(canvas); },
    };
    const layers = Array.from({length: 3}, (_, id) => {
        const host = new Element();
        host.dataset.role = 'scroller-canvas-host';
        host.dataset.viewportVisible = id === 1 ? 'true' : 'false';
        const layer = new Layer({getFromPool: () => new Element()});
        layer.build(host, id, map, pool);
        return layer;
    });
    const manager = Object.create(RenderLayerManager.prototype);
    manager.layers = layers;
    return {map, free, leased, layers, manager, misses: () => misses};
}

test('retire across all layers and release GPU ownership before any replacement checkout', () => {
    const h = harness(2);
    h.layers[2].setCanvasPlan(plan(surface('survivor'), surface('old')));
    h.layers[2].syncCanvasLayout();
    const survivor = h.map.get('survivor');
    h.leased.add(h.map.get('old'));
    const plans = [plan(), plan(surface('new')), plan(surface('survivor'))];
    h.manager.syncLayerCanvasLayouts(id => plans[id], retired => {
        assert.deepEqual(Array.from(retired), [2]);
        assert.equal(h.map.has('new'), false);
        h.leased.clear(); // synchronous renderer refresh drops obsolete surfaces
    });
    assert.equal(h.misses(), 0);
    assert.ok(h.map.has('new'));
    assert.equal(h.map.get('survivor'), survivor);
});

test('visible layers allocate before warm layers regardless of numeric layer order', () => {
    const h = harness(1);
    h.manager.syncLayerCanvasLayouts(id => plan(surface(`layer-${id}`)));
    assert.ok(h.map.has('layer-1'));
    assert.equal(h.map.has('layer-0'), false);
});

test('global materialization rank beats layer-local optional ordering', () => {
    const h = harness(2);
    const plans = [plan(surface('first-visible', -2), surface('optional', 1)),
        plan(surface('second-visible', -1)), plan()];
    h.manager.syncLayerCanvasLayouts(id => plans[id]);
    assert.ok(h.map.has('first-visible'));
    assert.ok(h.map.has('second-visible'));
    assert.equal(h.map.has('optional'), false);
});

test('single-layer turnover retires first and prioritizes visible tiles', () => {
    const h = harness(1);
    const layer = h.layers[1];
    layer.setCanvasPlan(plan(surface('old')));
    layer.syncCanvasLayout();
    layer.setCanvasPlan(plan(surface('warm', 2), surface('visible', 0)));
    layer.syncCanvasLayout();
    assert.ok(h.map.has('visible'));
    assert.equal(h.map.has('old'), false);
});

test('an in-flight owner blocks reuse until it releases; unchanged plans retry', () => {
    const h = harness(1);
    const layer = h.layers[1];
    layer.setCanvasPlan(plan(surface('old')));
    layer.syncCanvasLayout();
    h.leased.add(h.map.get('old'));
    layer.setCanvasPlan(plan(surface('new')));
    layer.syncCanvasLayout();
    assert.equal(h.map.has('new'), false);
    h.leased.clear();
    layer.syncCanvasLayout();
    assert.ok(h.map.has('new'));
});

test('priority-only plan changes update the canvas without replacing its lease', () => {
    const h = harness(1);
    const layer = h.layers[1];
    layer.setCanvasPlan(plan(surface('tile', 2)));
    layer.syncCanvasLayout();
    const canvas = h.map.get('tile');
    const generation = canvas.dataset.surfaceGeneration;
    layer.setCanvasPlan(plan(surface('tile', 0)));
    layer.syncCanvasLayout();
    assert.equal(canvas.dataset.replayPriority, '0');
    assert.equal(canvas.dataset.surfaceGeneration, generation);
});
