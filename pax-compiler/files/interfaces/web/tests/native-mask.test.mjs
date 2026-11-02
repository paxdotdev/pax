import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

class Element {
    attributes = new Map(); children = []; dataset = {}; style = {};
    constructor(tag) { this.tag = tag; }
    get id() { return this.getAttribute('id'); }
    setAttribute(name, value) { this.attributes.set(name, value); }
    getAttribute(name) { return this.attributes.get(name) ?? null; }
    appendChild(child) { child.remove(); child.parentElement = this; this.children.push(child); }
    remove() {
        if (this.parentElement) {
            const parent = this.parentElement;
            parent.children.splice(parent.children.indexOf(this), 1);
            this.parentElement = undefined;
        }
    }
    descendants() { return this.children.flatMap(child => [child, ...child.descendants()]); }
    querySelectorAll(selector) {
        const all = this.descendants();
        if (selector.startsWith('#')) return all.filter(e => e.id === selector.slice(1));
        if (selector.startsWith('[id^="')) return all.filter(e => e.id?.startsWith(selector.slice(6, -2)));
        if (selector === ':scope > path') return this.children.filter(e => e.tag === 'path');
        if (selector === ':scope > rect[data-role="backdrop"]') {
            return this.children.filter(e => e.tag === 'rect' && e.dataset.role === 'backdrop');
        }
        throw new Error(`Unexpected selector: ${selector}`);
    }
    querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
}

const source = readFileSync(new URL('../src/classes/render-layer-context.ts', import.meta.url), 'utf8');
function harness() {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, require: () => ({}), CSS: {escape: value => value},
        document: {createElementNS: (_, tag) => new Element(tag)},
    });
    const manager = new module.exports.RenderLayerManager({});
    const parent = new Element('div');
    manager.effects.attach(parent);
    return {effects: manager.effects, parent};
}

test('native mask clips retain empty intersections and recover on subsequent motion frames', () => {
    const {effects, parent} = harness();
    const rect = 'M0,0H100V40H0Z';
    for (const clip of [rect, '', '', rect, '', rect]) {
        effects.updateMask(88, [{path: rect, clips: [rect, clip], opacity: 1}], 100, 40);
        const mask = parent.querySelector('#pax-native-mask-88');
        const groups = mask.descendants().filter(e => e.tag === 'g');
        assert.equal(groups.length, 2);
        for (const group of groups) {
            const reference = group.getAttribute('clip-path').slice(5, -1);
            assert.ok(parent.querySelector(`#${reference}`), 'every mask clip reference must resolve');
        }
        const projectedClip = parent.querySelector('#pax-native-mask-88-clip-0-0');
        assert.equal(projectedClip.querySelector(':scope > path').getAttribute('d'), clip);
        assert.equal(mask.children.filter(e => e.tag === 'rect').length, 1);
    }
    effects.updateMask(88, [], 100, 40);
    assert.equal(parent.querySelector('#pax-native-mask-88'), null);
    assert.equal(parent.querySelectorAll('[id^="pax-native-mask-88-clip-"]').length, 0);
});

test('an absent clip removes its definition; an empty clip remains a valid empty region', () => {
    const {effects, parent} = harness();
    effects.updateClipPath('clip', 'M0,0H100V40H0Z');
    effects.updateClipPath('clip', '');
    assert.equal(parent.querySelector('#clip').querySelector(':scope > path').getAttribute('d'), '');
    effects.updateClipPath('clip', undefined);
    assert.equal(parent.querySelector('#clip'), null);
});

test('opacity patches retain mask paths and groups; membership and clip-depth changes retire stale objects', () => {
    const {effects, parent} = harness();
    const rect = 'M0,0H100V40H0Z';
    const entries = [
        {path: rect, clips: [rect, ''], opacity: 1},
        {path: 'M0,0H20V20H0Z', clips: [], opacity: 1},
    ];
    effects.updateMask(9, entries, 100, 40);
    const mask = parent.querySelector('#pax-native-mask-9');
    const objects = mask.descendants();
    const clips = parent.querySelectorAll('[id^="pax-native-mask-9-clip-"]');
    for (let frame = 0; frame < 100; frame++) {
        effects.updateMask(9, entries.map(e => ({...e, opacity: frame / 100})), 100, 40);
        assert.deepEqual(mask.descendants(), objects, 'opacity must allocate no mask path/group objects');
        assert.deepEqual(parent.querySelectorAll('[id^="pax-native-mask-9-clip-"]'), clips);
    }
    const hole = objects.find(e => e.tag === 'path');
    effects.updateMask(9, [{path: rect, clips: [rect], opacity: 0.4}], 120, 50);
    assert.equal(mask.descendants().filter(e => e.tag === 'path').length, 1);
    assert.equal(mask.descendants().find(e => e.tag === 'path'), hole);
    assert.equal(mask.descendants().filter(e => e.tag === 'g').length, 1);
    assert.equal(parent.querySelectorAll('[id^="pax-native-mask-9-clip-"]').length, 1);
    assert.equal(hole.getAttribute('fill-opacity'), '0.4');
    effects.updateMask(9, [], 100, 40);
    assert.equal(effects.maskEntries.size, 0);
    effects.updateMask(9, entries, 100, 40);
    assert.notEqual(parent.querySelector('#pax-native-mask-9'), mask);
    effects.cleanUp();
    assert.equal(effects.maskEntries.size, 0);
});
