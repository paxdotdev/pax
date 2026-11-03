import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Only model the SVG DOM operations used by the real registry; the registry and
// native deletion handlers themselves run unchanged.
class Element {
    children = []; style = {}; dataset = {}; attributes = {}; parentElement;
    classList = {remove() {}};
    constructor(tag) { this.tag = tag; }
    get id() { return this.attributes.id; }
    setAttribute(name, value) { this.attributes[name] = value; }
    getAttribute(name) { return this.attributes[name]; }
    appendChild(child) { child.remove(); child.parentElement = this; this.children.push(child); }
    removeChild(child) { this.children = this.children.filter(c => c !== child); child.parentElement = undefined; }
    remove() { this.parentElement?.removeChild(this); }
    querySelectorAll(selector) {
        const all = this.children.flatMap(child => [child, ...child.querySelectorAll('*')]);
        if (selector === '*') return all;
        if (selector.startsWith('#')) return all.filter(child => child.id === selector.slice(1));
        if (selector.startsWith('[id^=')) return all.filter(child => child.id?.startsWith(selector.slice(6, -2)));
        if (selector === ':scope > path') return this.children.filter(child => child.tag === 'path');
        if (selector === ':scope > rect[data-role="backdrop"]') return this.children.filter(child => child.tag === 'rect' && child.dataset.role === 'backdrop');
        throw new Error(selector);
    }
    querySelector(selector) { return this.querySelectorAll(selector)[0]; }
}
function load(name, suffix = '') {
    const module = {exports: {}};
    const source = readFileSync(new URL(`../src/classes/${name}.ts`, import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(source + suffix, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, require: () => ({}),
        CSS: {escape: value => value}, document: {createElementNS: (_, tag) => new Element(tag)},
    });
    return module.exports;
}
const {RenderLayerManager, SvgEffectManager} = load('render-layer-context', '\nexport {SvgEffectManager};');
const {NativeElementPool} = load('native-element-pool');

for (const kind of ['checkbox', 'nativeImage', 'youtubeVideo', 'textbox', 'radioList', 'slider', 'dropdown', 'button', 'photoPicker', 'text', 'eventBlocker', 'scroller']) {
    test(`${kind} route turnover releases masks and nested clip definitions`, () => {
        const host = new Element('div');
        const effects = new SvgEffectManager();
        effects.attach(host);
        const manager = new RenderLayerManager({});
        manager.effects = effects;
        manager.unregisterScrollerIslandHosts = () => false;
        manager.unregisterParentFrameHost = () => {};
        const pool = Object.create(NativeElementPool.prototype);
        Object.assign(pool, {layers: manager, nodesLookup: new Map(), textStates: new Map(), pendingTextPresentation: new Set(), resizeObserver: {unobserve() {}},
            scrollerMeasurementStates: new Map(), pendingScrollerUpdates: new Map(), presentationRecords: new Map(), scrollerHosts: new Map(),
            getScrollerCanvasHost() {}, getScrollerContentHost() {}});
        effects.updateClipPath('unrelated-container', 'M0 0H20V20Z');
        for (let id = 1; id <= 100; id++) {
            const leaf = new Element('div');
            host.appendChild(leaf);
            pool.nodesLookup.set(id, leaf);
            let released = false;
            manager.maskImages.set(leaf, {dispose() {released = true;}});
            effects.updateMask(id, [{path: 'M0 0H10V10Z', opacity: 1, clips: ['M0 0H20V20Z', 'M1 1H19V19Z']}], 100, 100);
            assert.equal(effects.defs.children.length, 4);
            pool[`${kind}Delete`](id);
            assert.equal(effects.defs.children.length, 1, `mask leaked after deletion ${id}`);
            assert.equal(effects.defs.children[0].id, 'unrelated-container');
            assert.equal(pool.nodesLookup.size, 0);
            assert.equal(released, true, 'prepared bitmap owner must be retired too');
            assert.equal(manager.maskImages.size, 0);
        }
    });
}
