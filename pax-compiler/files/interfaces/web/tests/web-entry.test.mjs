import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function moduleFrom(source, globals = {}) {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code,
        {module, exports: module.exports, ...globals});
    return module.exports;
}
const {abortable} = moduleFrom(readFileSync(new URL('../src/utils/abortable.ts', import.meta.url), 'utf8'));
const entry = readFileSync(new URL('../src/index.ts', import.meta.url), 'utf8');
// Keep the public async boundary and real sharing/cancellation logic. Only
// module acquisition and application mounting are replaced with controlled I/O.
const source = entry.slice(entry.indexOf('type CartridgeLoad'), entry.indexOf('const nativeMessageOwners'))
    .replace('import(key)', 'importGlue(key)');
const deferred = () => { let resolve, reject; const promise = new Promise((a, b) => {resolve = a; reject = b;}); return {promise, resolve, reject}; };
const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };
function harness({importGate, installGate, loader = false} = {}) {
    const document = {baseURI: 'https://example.test/', querySelector: () => element};
    class HTMLElement { constructor() { this.ownerDocument = document; } }
    const element = new HTMLElement(), requests = [], mounts = [], loadingCalls = [];
    let imports = 0, installations = 0;
    const glue = {default: async () => { installations++; await installGate?.promise; },
        pax_set_shutdown_waker() {}, ...(loader ? {init_loading_screen() {}} : {})};
    const api = moduleFrom(source, {
        URL, URLSearchParams, Uint8Array, AbortController, abortable, HTMLElement, document,
        window: {location: {search: ''}}, hosts: new Set(), queueMicrotask,
        async importGlue() { imports++; await importGate?.promise; return glue; },
        fetch(_url, {signal}) { const result = deferred(); requests.push({...result, signal}); return result.promise; },
        createApplicationHost: () => ({initApplication: async options => { mounts.push(options); return options; }}),
        startLoadingApplication: async options => { loadingCalls.push(options); return options; },
    });
    return {api, element, requests, mounts, loadingCalls, imports: () => imports, installations: () => installations};
}

test('startup data is snapshotted at invocation before bootstrap and loader waits', async () => {
    for (const loader of [false, true]) {
        const h = harness({loader}), bytes = new Uint8Array([1, 2]);
        const pending = h.api.init_web({element: h.element, cartridgeUrl: '/app', startupData: bytes});
        bytes[0] = 9;
        await flush(); h.requests[0].resolve({ok: true});
        const options = await pending;
        bytes[1] = 8;
        assert.deepEqual([...options.startupData], [1, 2]);
        assert.notEqual(options.startupData, bytes);
        assert.equal(loader ? h.loadingCalls.length : h.mounts.length, 1);
    }
});

test('invalid public inputs reject before loading a cartridge', async () => {
    const h = harness();
    for (const options of [{startupData: []}, {startupData: new Uint8Array(65537)}, {entry: 42}]) {
        await assert.rejects(h.api.init_web({element: h.element, cartridgeUrl: '/app', ...options}));
    }
    assert.equal(h.imports(), 0);
});

test('abort while importing detaches immediately and never starts the fetch', async () => {
    const gate = deferred(), h = harness({importGate: gate}), controller = new AbortController();
    const pending = h.api.init_web({element: h.element, cartridgeUrl: '/app', signal: controller.signal});
    await flush(); controller.abort();
    await assert.rejects(pending, {name: 'AbortError'});
    gate.resolve(); await flush();
    assert.equal(h.requests.length, 0);
    assert.equal(h.mounts.length, 0);
});

test('cancelling one caller preserves the shared bootstrap for a live caller', async () => {
    const h = harness(), controller = new AbortController();
    const left = h.api.init_web({element: h.element, cartridgeUrl: '/app', signal: controller.signal});
    const right = h.api.init_web({element: h.element, cartridgeUrl: '/app'});
    await flush();
    assert.equal(h.requests.length, 1);
    controller.abort(); await assert.rejects(left, {name: 'AbortError'});
    assert.equal(h.requests[0].signal.aborted, false);
    h.requests[0].resolve({ok: true}); await right;
    await h.api.init_web({element: h.element, cartridgeUrl: '/app'});
    assert.equal(h.requests.length, 1);
    assert.equal(h.installations(), 1);
    assert.equal(h.mounts.length, 2);
});

test('last caller aborts the fetch; immediate retry waits for abandoned work to settle', async () => {
    const h = harness(), controller = new AbortController();
    const pending = h.api.init_web({element: h.element, cartridgeUrl: '/app', signal: controller.signal});
    await flush(); controller.abort(); await assert.rejects(pending, {name: 'AbortError'});
    assert.equal(h.requests[0].signal.aborted, true);
    const retry = h.api.init_web({element: h.element, cartridgeUrl: '/app'});
    await flush(); assert.equal(h.requests.length, 1);
    h.requests[0].reject(h.requests[0].signal.reason);
    await flush(); assert.equal(h.requests.length, 2);
    h.requests[1].resolve({ok: true}); await retry;
    assert.equal(h.installations(), 1);
});

test('abandoned installation may finish and is reused without concurrent initialization', async () => {
    const gate = deferred(), h = harness({installGate: gate}), controller = new AbortController();
    const pending = h.api.init_web({element: h.element, cartridgeUrl: '/app', signal: controller.signal});
    await flush(); h.requests[0].resolve({ok: true}); await flush();
    assert.equal(h.installations(), 1);
    controller.abort(); await assert.rejects(pending, {name: 'AbortError'});
    const retry = h.api.init_web({element: h.element, cartridgeUrl: '/app'});
    await flush(); assert.equal(h.installations(), 1); assert.equal(h.mounts.length, 0);
    gate.resolve(); await retry;
    assert.equal(h.installations(), 1); assert.equal(h.requests.length, 1); assert.equal(h.mounts.length, 1);
});

test('legacy mount retains standalone defaults while init_web keeps embedding defaults', async () => {
    const h = harness();
    const pending = h.api.mount('#app', '/app');
    await flush(); h.requests[0].resolve({ok: true});
    const legacy = await pending;
    assert.equal(legacy.pageScroll, true); assert.equal(legacy.documentRouting, true);
    const embedded = await h.api.init_web({element: h.element, cartridgeUrl: '/app'});
    assert.equal(embedded.pageScroll, undefined); assert.equal(embedded.documentRouting, undefined);
});
