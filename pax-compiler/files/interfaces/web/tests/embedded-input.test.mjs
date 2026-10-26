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
    const response = {prevent_default: false};
    const dispose = module.exports.setupEventListeners({interrupt: message => { interrupts.push(message); return response; }}, mount);
    const event = (target, clientX = 120, clientY = 80) => ({target, clientX, clientY, button: 0, preventDefault() {}, stopPropagation() {}});
    return {window, document, mount, outside, dispose, interrupts, event, response};
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

test('focus loss cancels a drag once and later outside moves cannot continue it', () => {
    const h = harness();
    h.mount.send('mousedown', h.event(h.mount));
    h.window.send('mousemove', {...h.event(h.outside, 80, 40), buttons: 1});
    h.window.send('blur', {});
    assert.equal(h.interrupts[2].MouseCancel.button, 'Left');
    assert.equal(h.interrupts[2].MouseCancel.x, -20);
    h.window.send('mousemove', h.event(h.outside));
    h.window.send('blur', {});
    assert.equal(h.interrupts.length, 3);
    h.dispose();
});

test('a lost release cancels capture before hover resumes; disposal also cancels', () => {
    const h = harness();
    h.mount.send('mousedown', h.event(h.mount));
    h.window.send('mousemove', {...h.event(h.mount), buttons: 0});
    assert.ok(h.interrupts[1].MouseCancel);
    assert.ok(h.interrupts[2].MouseMove);
    h.mount.send('mousedown', h.event(h.mount));
    h.dispose();
    assert.ok(h.interrupts.at(-1).MouseCancel);
});

test('Escape reaches dismissal handlers from native focus without stealing text or IME input', () => {
    const h = harness();
    h.document.activeElement = h.outside;
    let prevented = 0;
    const key = (value, isComposing = false) => ({...h.event(h.mount), key: value, isComposing,
        preventDefault() { prevented++; }});
    h.mount.send('keydown', key('a'));
    h.mount.send('keydown', key('Escape', true));
    assert.equal(h.interrupts.length, 0);
    h.response.prevent_default = true;
    h.mount.send('keydown', key('Escape'));
    h.mount.send('keyup', key('Escape'));
    assert.equal(h.interrupts[0].KeyDown.key, 'Escape');
    assert.equal(h.interrupts[1].KeyUp.key, 'Escape');
    assert.equal(prevented, 2);
    h.dispose();
});

test('click and wheel still reach Pax when a transparent native leaf is not the DOM target', () => {
    const h = harness();
    // NativeImage and non-selectable Text leave browser hit-testing to the
    // underlying host; engine dispatch must rely on position, not a leaf ID.
    h.mount.send('click', h.event(h.mount));
    h.mount.send('wheel', {...h.event(h.mount), deltaX: 0, deltaY: 80});
    assert.equal(h.interrupts[0].Click.x, 20);
    assert.equal(h.interrupts[0].Click.y, 30);
    assert.equal(h.interrupts[1].Wheel.delta_y, 80);
    h.dispose();
});
