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
function harness(permission, nativeHit = () => true, options = {}) {
    const window = new Target(), document = {activeElement: null}, interrupts = [];
    class Element extends Target {
        closest() { return null; }
        matches() { return false; }
        contains(target) { return target === this; }
        getBoundingClientRect() { return {left: 100, top: 50}; }
        focus() { document.activeElement = this; }
    }
    const mount = new Element(), outside = new Element();
    if (permission) window.DeviceOrientationEvent = {requestPermission: () => permission};
    const module = {exports: {}};
    vm.runInNewContext(code, {module, exports: module.exports, window, document, console,
        Element, HTMLElement: Element, HTMLInputElement: class {}, HTMLTextAreaElement: class {},
        getComputedStyle: e => e.style, performance: {now: () => 0}});
    const response = {prevent_default: false};
    const dispose = module.exports.setupEventListeners({native_pointer_hit_test: nativeHit, interrupt: message => { interrupts.push(message); return response; }}, mount, options);
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

test('delegated input uses the scene viewport; a painted tile cannot cancel a page pan', () => {
    let delegated = true;
    const hits = [];
    const h = harness(undefined, (...args) => { hits.push(args); return false; }, {pageScrollDelegated: () => delegated});
    h.window.scrollX = 0; h.window.scrollY = 649;
    h.mount.getBoundingClientRect = () => ({left: 0, top: -649});
    h.outside.closest = s => s === '.native-leaf[pax_id]' ? {getAttribute: () => '61'} : null;
    h.outside.matches = s => s === '.scroller-container[data-page-scroll-delegated]';
    let cancelled = 0;
    const finger = {identifier: 3, clientX: 185, clientY: 645};
    const e = {...h.event(h.outside), touches: [finger], changedTouches: [finger], preventDefault: () => cancelled++};
    h.mount.send('touchstart', e);
    assert.deepEqual([h.interrupts[0].TouchStart.touches[0].y, cancelled, hits.length], [645, 0, 0]);
    h.outside.matches = () => false;
    h.mount.send('touchstart', e);
    assert.deepEqual(hits[0], [61, 185, 645], 'a covered native control is still gated');
    assert.equal(cancelled, 0, 'a covered label cannot cancel the whole swipe on contact');
    h.mount.send('touchend', {...e, touches: []});
    assert.equal(cancelled, 1);
    h.outside.closest = s => s.startsWith('input,') ? h.outside : s === '.native-leaf[pax_id]' ? {getAttribute: () => '61'} : null;
    h.mount.send('touchstart', e);
    assert.equal(cancelled, 2, 'an occluded editor that acts on contact is still protected');
    delegated = false;
    h.mount.send('wheel', {...h.event(h.outside, 185, 645), deltaX: 0, deltaY: 10});
    assert.equal(h.interrupts.at(-1).Wheel.y, 1294, 'an embedded mount keeps local coordinates');
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

test('a covered horizontal shelf cannot be dragged but lets a vertical gesture reach the page', () => {
    const h = harness(undefined, () => false, {pageScrollDelegated: () => true});
    h.window.scrollX = h.window.scrollY = 0;
    const shelf = h.outside;
    Object.assign(shelf, {scrollWidth: 810, clientWidth: 320, scrollHeight: 80, clientHeight: 80, style: {overflowX: 'auto', overflowY: 'hidden'}});
    shelf.closest = s => s === '.native-leaf[pax_id]' ? {getAttribute: () => '72'} : null;
    shelf.matches = s => s === '.scroller-container:not([data-page-scroll-delegated])';
    let cancelled = 0;
    const gesture = (type, x, y) => {
        const point = {identifier: 1, clientX: x, clientY: y};
        h.mount.send(type, {...h.event(shelf), touches:[point], changedTouches:[point], preventDefault: () => cancelled++});
    };
    gesture('touchstart', 100, 100); gesture('touchmove', 100, 75);
    assert.equal(cancelled, 0);
    gesture('touchstart', 100, 100); gesture('touchmove', 65, 98);
    assert.equal(cancelled, 1);
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


test('covered native starts are blocked while engine events and keyboard activation survive', () => {
    const queries=[];
    const h=harness(undefined, (...args) => {queries.push(args); return false;});
    h.outside.closest=selector=>selector === '.native-leaf[pax_id]' ? {getAttribute:()=> '42'} : null;
    let prevented=0,stopped=0;
    const event={...h.event(h.outside),detail:1,preventDefault(){prevented++;},stopPropagation(){stopped++;}};
    h.mount.send('mousedown',event);
    h.mount.send('click',event);
    assert.deepEqual(queries,[[42,20,30],[42,20,30]]);
    assert.equal(prevented,2);assert.equal(stopped,2);
    assert.ok(h.interrupts[0].MouseDown);assert.ok(h.interrupts[1].Click);
    h.mount.send('click',{...event,detail:0});
    assert.equal(prevented,2,'keyboard activation is independent of visual occlusion');
    h.window.send('mousemove',{...event,buttons:1});
    assert.equal(queries.length,2,'an established drag is not re-targeted as content moves');
    h.dispose();
});
