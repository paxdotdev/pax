import test from 'node:test';
import assert from 'node:assert/strict';
import {buildSync} from 'esbuild';
import vm from 'node:vm';

const code = buildSync({entryPoints: [new URL('../src/events/listeners.ts', import.meta.url).pathname],
    bundle: true, write: false, platform: 'browser', format: 'cjs'}).outputFiles[0].text;
class Target {
    listeners = new Map();
    addEventListener(type, callback) { const list = this.listeners.get(type) ?? new Set(); list.add(callback); this.listeners.set(type, list); }
    removeEventListener(type, callback) { this.listeners.get(type)?.delete(callback); }
    send(type, event) { for (const listener of this.listeners.get(type) ?? []) listener(event); }
    listenerCount() { return [...this.listeners.values()].reduce((n, set) => n + set.size, 0); }
}
function harness(permission) {
    const window = new Target(), document = {activeElement: null}, interrupts = [];
    class Element extends Target {
        closest() { return null; }
        contains(target) { return target === this; }
        getBoundingClientRect() { return {left: 100, top: 50}; }
        focus() { document.activeElement = this; }
    }
    const mount = new Element(), outside = new Element();
    if (permission) window.DeviceOrientationEvent = {requestPermission: () => permission};
    const module = {exports: {}};
    vm.runInNewContext(code, {module, exports: module.exports, window, document, console,
        Element, HTMLElement: Element, HTMLInputElement: class {}, HTMLTextAreaElement: class {}, performance: {now: () => 0}});
    const dispose = module.exports.setupEventListeners({interrupt: message => { interrupts.push(message); return {prevent_default: false}; }}, mount);
    const event = (target, clientX = 120, clientY = 80) => ({target, clientX, clientY, button: 0, preventDefault() {}, stopPropagation() {}});
    return {window, document, mount, outside, dispose, interrupts, event};
}
test('embedded pointer input is local and only an initiated drag continues outside', () => {
    const h = harness();
    h.window.send('mousemove', h.event(h.outside));
    h.window.send('mouseup', h.event(h.outside));
    assert.equal(h.interrupts.length, 0);
    h.mount.send('mousedown', h.event(h.mount));
    assert.equal(h.document.activeElement, h.mount);
    assert.equal(h.interrupts[0].MouseDown.x, 20);
    assert.equal(h.interrupts[0].MouseDown.y, 30);
    h.window.send('mousemove', h.event(h.outside, 80, 40));
    assert.equal(h.interrupts[1].MouseMove.x, -20);
    assert.equal(h.interrupts[1].MouseMove.y, -10);
    h.window.send('mouseup', h.event(h.outside));
    h.window.send('mousemove', h.event(h.outside));
    assert.equal(h.interrupts.length, 3);
    h.dispose();
    assert.equal(h.mount.listenerCount() + h.window.listenerCount(), 0);
});
test('late sensor permission cannot reattach callbacks after disposal', async () => {
    let resolve;
    const h = harness(new Promise(done => { resolve = done; }));
    h.window.paxRequestDeviceSensorPermissions();
    h.dispose();
    resolve('granted');
    await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
    assert.equal(h.window.listenerCount() + h.mount.listenerCount(), 0);
    assert.equal(h.window.paxRequestDeviceSensorPermissions, undefined);
    assert.equal(h.interrupts.length, 0);
});
