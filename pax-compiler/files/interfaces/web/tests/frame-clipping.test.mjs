import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function load(path, globals = {}) {
    const module = {exports: {}};
    const source = readFileSync(new URL(path, import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(source, {loader:'ts', format:'cjs'}).code,
        {module, exports:module.exports, require: () => ({}), ...globals});
    return module.exports;
}
const projection = load('../src/utils/projection.ts');
const helpers = load('../src/utils/helpers.ts', {
    require: path => path === './projection' ? projection : {},
});
const placement = load('../src/classes/native-placement.ts', {
    require: () => helpers,
});
const {NATIVE_SURFACE, NATIVE_SURFACE_INVERSE} = placement;
class Element {
    style = {}; dataset = {}; children = []; parentElement;
    classList = {contains: () => false};
    get firstElementChild() { return this.children[0]; }
    setAttribute() {}
    appendChild(child) {
        child.parentElement?.removeChild(child);
        this.children.push(child); child.parentElement = this;
    }
    removeChild(child) { this.children = this.children.filter(c => c !== child); child.parentElement = undefined; }
    querySelector(selector) { return this.querySelectorAll(selector)[0]; }
    querySelectorAll(selector) {
        const id = selector.match(/data-container-id="(\d+)"/)?.[1];
        return this.children.flatMap(c => [...(id != null && c.dataset.containerId === id ? [c] : []), ...c.querySelectorAll(selector)]);
    }
}
function fixture(supportsClip = true) {
    const root = new Element();
    const {RenderLayerManager, ContainerStyle, nativeFrameBoxInverse} = load('../src/classes/render-layer-context.ts', {
        require: path => path === '../utils/helpers' ? helpers : path === './native-placement' ? placement : {},
        document: {createElement: () => new Element(), querySelectorAll: selector => root.querySelectorAll(selector)},
        CSS: {supports: () => supportsClip},
    });
    const manager = new RenderLayerManager({getFromPool: () => new Element(), returnToPool() {}});
    manager.layers = [{native:root}];
    manager.effects = {updateClipPath() {}};
    return {root, manager, ContainerStyle, nativeFrameBoxInverse};
}
const geometry = {clipIsRect:true, clipContent:true, width:200, height:100,
    borderRadius:20, transform:[0.5,0,0,0.5,120,40]};

test('scroller-local Frame clips retain their chart through box, SVG and world handoffs', () => {
    const {root, manager} = fixture();
    manager.addContainer(1);
    manager.updateContainer(1, {...geometry, transformScroller:42});
    const content = manager.getOrCreateContainer(1, 0);
    const box = root.children[0];
    assert.equal(box.style.transform, 'matrix(0.5,0,0,0.5,120,40)');
    assert.equal(content.style.transform, 'matrix(2,0,0,2,-240,-80)');
    // Host pose changes only its inherited chart; no child update is necessary.
    root.style.transform = 'matrix(1,0,0,1,60,70)';
    assert.equal(box.style.transform, 'matrix(0.5,0,0,0.5,120,40)');
    manager.updateContainer(1, {clipIsRect:false, clipPath:'M0,0L50,0L0,50Z'});
    assert.equal(box.style.transform, '');
    assert.equal(content.style.transform, '');
    manager.updateContainer(1, {transformScroller:null});
    assert.equal(box.style.transform, NATIVE_SURFACE_INVERSE);
    assert.equal(content.style.transform, NATIVE_SURFACE);
    manager.updateContainer(1, {...geometry, transformScroller:42});
    assert.equal(box.style.transform, 'matrix(0.5,0,0,0.5,120,40)');
    manager.updateContainer(1, {transformScroller:null});
    assert.equal(box.style.transform, `${NATIVE_SURFACE_INVERSE} matrix(0.5,0,0,0.5,120,40)`);
});

test('native frame box preserves world-space leaves, nesting and radius with no extra scroller', () => {
    const {root, manager} = fixture();
    manager.addContainer(1);
    manager.updateContainer(1, geometry);
    const content = manager.getOrCreateContainer(1, 0);
    const box = root.children[0];
    assert.equal(box.style.overflow, 'clip');
    assert.equal(box.style.borderRadius, '20px');
    assert.equal(box.style.transform, `${NATIVE_SURFACE_INVERSE} matrix(0.5,0,0,0.5,120,40)`);
    assert.equal(content.style.transform, `matrix(2,0,0,2,-240,-80) ${NATIVE_SURFACE}`);
    assert.equal(box.dataset.clipBounds, undefined, 'sized box must not add old world-space bounds twice');
    manager.addContainer(2, 1);
    manager.updateContainer(2, {...geometry, transform:[1,0,0,1,140,50]});
    const nested = manager.getOrCreateContainer(2, 0);
    assert.equal(nested.parentElement.parentElement, content);
    assert.equal(manager.getOrCreateContainer(1, 0), content);
    manager.removeContainer(2);
    manager.removeContainer(1);
    assert.equal(root.children.length, 0);
});

test('box transitions to SVG and unclipped preserve children and clear compensation', () => {
    const {root, manager} = fixture();
    manager.addContainer(1);
    manager.updateContainer(1, geometry);
    const content = manager.getOrCreateContainer(1, 0);
    const leaf = new Element(); content.appendChild(leaf);
    const box = root.children[0];
    manager.updateContainer(1, {clipIsRect:false, clipPath:'M0,0L50,0L0,50Z'});
    assert.equal(box.style.overflow, '');
    assert.equal(content.style.transform, NATIVE_SURFACE);
    assert.equal(box.style.transform, NATIVE_SURFACE_INVERSE);
    assert.equal(box.style.clipPath, 'url(#pax-container-clip-1)');
    assert.equal(content.children[0], leaf);
    manager.updateContainer(1, {clipContent:false});
    assert.equal(box.style.clipPath, 'none');
    assert.equal(box.dataset.clipBounds, undefined);
    assert.equal(box.style.transform, '', 'unclipped frames must not create a stacking context');
    assert.equal(content.style.transform, '');
});

test('perspective frames retain their projected aperture through affine and near-plane transitions', () => {
    const {root, manager, nativeFrameBoxInverse, ContainerStyle} = fixture();
    manager.addContainer(1);
    manager.updateContainer(1, geometry);
    const content = manager.getOrCreateContainer(1, 0);
    const leaf = new Element(); content.appendChild(leaf);
    const box = root.children[0];
    // The engine clips the projected aperture at the camera depth range. Even
    // an invertible homography cannot replace this with the whole local box.
    const transform = [1,0,0,0.003, 0,1,0,0, 0,0,1,0, 120,40,0,1];
    const clipPath = 'M120,40L200,25L200,87.5L120,140Z';
    assert.equal(nativeFrameBoxInverse({...new ContainerStyle(), ...geometry, transform}), undefined);
    manager.updateContainer(1, {transform, clipPath});
    assert.equal(box.dataset.paxClipMode, 'svg');
    assert.equal(box.style.clipPath, 'url(#pax-container-clip-1)');
    assert.equal(box.style.transform, NATIVE_SURFACE_INVERSE, 'never serialize sixteen coefficients into CSS matrix()');
    assert.equal(content.style.transform, NATIVE_SURFACE, 'clear the previous affine box compensation');
    assert.equal(manager.containers.get(1).clipPathData(), clipPath);
    assert.equal(content.children[0], leaf);

    manager.updateContainer(1, {transform: Array(16).fill(0), clipPath: 'M0,0Z'});
    assert.equal(box.dataset.paxClipMode, 'svg');
    assert.equal(manager.containers.get(1).clipPathData(), 'M0,0Z');
    manager.updateContainer(1, {...geometry, clipPath: undefined});
    assert.equal(box.dataset.paxClipMode, 'box');
    assert.equal(box.style.transform, `${NATIVE_SURFACE_INVERSE} matrix(0.5,0,0,0.5,120,40)`);
    assert.equal(content.style.transform, `matrix(2,0,0,2,-240,-80) ${NATIVE_SURFACE}`);
    assert.equal(content.children[0], leaf, 'camera recovery must preserve mounted native content');
});

test('arbitrary masks, degenerate transforms and unsupported geometry never become box clips', () => {
    const {nativeFrameBoxInverse, ContainerStyle} = fixture();
    const style = {...new ContainerStyle(), ...geometry};
    for (const patch of [{clipIsRect:false}, {clipContent:false}, {width:0}, {width:NaN},
        {transform:[0,0,0,0,0,0]}, {transform:[1,0,0,1,Infinity,0]}]) {
        assert.equal(nativeFrameBoxInverse({...style, ...patch}), undefined);
    }
    for (const matrix of [[0,-2,3,0,40,80], [-1,0,0,2,30,60], [1,0.2,0.5,1,30,60]]) {
        const inverse = nativeFrameBoxInverse({...style, transform:matrix});
        const local = [31,72];
        const restored = helpers.affineMultiply(helpers.affineMultiply(local, inverse), matrix);
        assert.ok(restored.every((v, i) => Math.abs(v-local[i]) < 1e-10));
    }
    const {root, manager} = fixture(false);
    manager.addContainer(1); manager.updateContainer(1, geometry); manager.getOrCreateContainer(1, 0);
    assert.equal(root.children[0].dataset.paxClipMode, 'svg');
    assert.equal(manager.getOrCreateContainer(1, 0), root.children[0].children[0], 'SVG fallback uses the same local-content chart');
    assert.equal(root.children[0].style.transform, NATIVE_SURFACE_INVERSE);
});
