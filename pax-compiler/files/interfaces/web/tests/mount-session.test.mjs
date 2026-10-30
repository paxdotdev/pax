import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Exercise the entrypoint's actual session owner without loading a Wasm cartridge.
const entry = readFileSync(new URL('../src/index.ts', import.meta.url), 'utf8');
const owner = entry.slice(entry.indexOf('class MountSession'), entry.indexOf('/** Explicit browser entry'));
function session() {
    const module = {exports: {}};
    let disposals = 0, removals = 0;
    vm.runInNewContext(transformSync(`let liveSession = null, currentChassis = null;
        ${owner}
        export function create(options) { return liveSession = new MountSession(options); }
    `, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, DOMException,
        shutdownObservers: new WeakMap(), hosts: new Set(), host: {},
        document: {createElement: () => ({remove() { removals++; }})},
        disposeCurrentChassis() { disposals++; },
    });
    const cancellation = new AbortController();
    const app = module.exports.create({element: {appendChild() {}}, signal: cancellation.signal});
    return {app, cancellation, disposals: () => disposals, removals: () => removals};
}

test('mount readiness waits for initial presentation', async () => {
    const {app} = session();
    let mounted = false;
    app.mounted.then(() => { mounted = true; });
    await Promise.resolve();
    assert.equal(mounted, false);
    app.didMount();
    await app.mounted;
    assert.equal(mounted, true);
    app.finishStartup();
    app.close();
    await app.closed;
});

test('abort during mounting rejects readiness and retains cleanup until startup unwinds', async () => {
    const h = session();
    let closed = false;
    h.app.closed.then(() => { closed = true; });
    h.cancellation.abort();
    h.app.didMount(); // A stale completion cannot turn cancellation into success.
    await assert.rejects(h.app.mounted, error => error.name === 'AbortError');
    assert.equal(closed, false);
    h.app.finishStartup();
    await h.app.closed;
    h.app.close();
    assert.equal(h.disposals(), 1);
    assert.equal(h.removals(), 1);
});

test('startup service failure preserves its original error and closes the region', async () => {
    const h = session();
    const failure = new Error('mount handler failed');
    h.app.failStartup(failure);
    await assert.rejects(h.app.mounted, error => error === failure);
    h.app.finishStartup();
    await h.app.closed;
    assert.equal(h.disposals(), 1);
});

test('revealing beneath a loader starts presentation once but retains input until publication', () => {
    const state = {ready: false, unpaused: 0, frames: 0, disposed: 0};
    const attributes = new Map();
    const region = {
        inert: false,
        setAttribute(name, value) { attributes.set(name, value); },
        removeAttribute(name) { attributes.delete(name); },
    };
    const module = {exports: {}};
    vm.runInNewContext(transformSync(`
        let liveSession = null;
        const currentChassis = {
            prepare_startup_presentation: () => state.ready,
            publish_application() { state.unpaused++; },
        };
        const mountYieldEnabled = true, startupPresented = true;
        const nativePool = {startupTextReady: () => true, startupSurfacesReady: () => true};
        let startupScheduler = {dispose() { state.disposed++; }}, startupWork = {};
        function runFrame() { state.frames++; }
        ${owner}
        export const app = new MountSession({element: {appendChild() {}}, holdPresentation: true});
    `, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, state,
        document: {createElement: () => region},
    });
    const {app} = module.exports;
    assert.equal(app.reveal(), false);
    assert.ok(attributes.has('data-pax-preparing'));
    state.ready = true;
    assert.equal(app.reveal(), true);
    assert.equal(attributes.has('data-pax-preparing'), false);
    assert.equal(region.inert, true);
    assert.equal(attributes.get('aria-hidden'), 'true');
    assert.equal(state.unpaused, 1);
    assert.equal(state.frames, 1);
    assert.equal(app.reveal(), true);
    assert.equal(app.publish(), true);
    assert.equal(region.inert, false);
    assert.equal(attributes.has('aria-hidden'), false);
    assert.equal(state.unpaused, 1);
    assert.equal(state.disposed, 1);
});
