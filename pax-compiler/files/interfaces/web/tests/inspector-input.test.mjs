import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

test('surface inspector input cannot hit-test the Pax content underneath', () => {
    const listeners = new Map(), interrupts = [];
    const window = {
        addEventListener: (name, fn) => listeners.set(name, fn),
        removeEventListener: (name, fn) => {assert.equal(listeners.get(name), fn); listeners.delete(name);},
    };
    class Element {
        constructor(inspector = false) {this.inspector = inspector;}
        closest(selector) {assert.equal(selector, '[data-pax-surface-inspector]'); return this.inspector ? this : null;}
    }
    const body = new Element();
    const module = {exports: {}};
    const source = readFileSync(new URL('../src/events/listeners.ts', import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code,
        {module, exports: module.exports, window, Element, document: {body, activeElement: body}});
    const dispose = module.exports.setupEventListeners({interrupt: value => {interrupts.push(value); return {prevent_default: false};}});
    const event = {target: new Element(true), clientX: 30, clientY: 40, button: 0, preventDefault() {
        assert.fail('inspector browser interaction must keep its default behavior');
    }};
    for (const name of ['click', 'mousedown', 'mouseup', 'mousemove', 'wheel', 'keydown', 'touchstart', 'touchend']) {
        listeners.get(name)(event);
    }
    assert.equal(interrupts.length, 0);
    event.target = body;
    listeners.get('click')(event);
    assert.equal(interrupts.length, 1, 'ordinary application clicks still route');
    assert.ok(interrupts[0].Click);
    dispose();
    assert.equal(listeners.size, 0);
});
