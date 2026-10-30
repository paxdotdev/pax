import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/loading.ts', import.meta.url), 'utf8')
    .replace("import {requestPreparationFrame} from './utils/shared-work-budget';", '')
    .replace("import {abortable} from './utils/abortable';", '')
    .replace("import(new URL('./__wasm_split.js', new URL(options.cartridgeUrl, document.baseURI)).href)", 'getSplit()');
const abortSource = readFileSync(new URL('../src/utils/abortable.ts', import.meta.url), 'utf8').replace('export function', 'function');
const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };
function harness({cancelOnFirstMount = false, ready = true, exitComplete = true} = {}) {
    const jobs = new Map(), phases = [], applications = [];
    let nextJob = 0, action, loaderClosed = false, exits = 0, retired = 0;
    const frame = callback => { const id = ++nextJob; jobs.set(id, callback); return id; };
    const controller = new AbortController();
    const loader = {publish: () => true, close() { loaderClosed = true; }, chassis: {
        loading_update(phase) { phases.push(phase); },
        take_loading_request() { const request = action; action = undefined; return request; },
        begin_presentation_exit() { exits++; }, presentation_exit_complete: () => exitComplete,
    }};
    const adapter = {load: async () => {}, observe: () => () => {}, state: {}};
    const module = {exports: {}};
    vm.runInNewContext(transformSync(abortSource + '\n' + source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, AbortController, WebAssembly, URL,
        document: {baseURI: 'https://example.test/', createElement: () => ({style: {}, dataset: {}, appendChild() {}, remove() {}})},
        requestAnimationFrame: frame, cancelAnimationFrame: id => jobs.delete(id), requestPreparationFrame: frame,
        getSplit: async () => ({loader: adapter, acquire: () => {
            let released = false; return () => { if (!released) { released = true; retired++; } };
        }}),
    });
    let factories = 0;
    const pending = module.exports.startLoadingApplication({element: {appendChild() {}}, cartridgeUrl: '/app', signal: controller.signal,
        startupData: new Uint8Array([7])}, {}, () => ({initApplication: async options => {
        if (factories++ === 0) return loader;
        let acknowledge;
        const app = {closed: new Promise(resolve => { acknowledge = resolve; }), isClosed: false, reveals: 0, publishes: 0,
            data: [...options.startupData],
            close() { this.isClosed = true; acknowledge(); },
            reveal() { this.reveals++; return ready && !this.isClosed; },
            publish() { this.publishes++; return !this.isClosed; },
        };
        applications.push(app);
        options.signal.addEventListener('abort', () => app.close(), {once: true});
        if (cancelOnFirstMount && applications.length === 1) action = 'cancel';
        return app;
    }}));
    return {pending, controller, phases, applications, request(value) { action = value; },
        exits: () => exits, retired: () => retired, loaderClosed: () => loaderClosed,
        async frame() { const callbacks = [...jobs.values()]; jobs.clear(); for (const callback of callbacks) { callback(); await flush(); } },
    };
}

test('a cancel queued at mount completion reaches cancelled, then retries without publishing the retired app', async () => {
    const h = harness({cancelOnFirstMount: true});
    await flush(); assert.equal(h.phases.at(-1), 'ready');
    await h.frame();
    assert.equal(h.phases.at(-1), 'cancelled');
    assert.equal(h.applications[0].isClosed, true);
    assert.equal(h.applications[0].reveals, 0);
    assert.equal(h.applications[0].publishes, 0);
    assert.equal(h.exits(), 0);
    h.request('retry'); await h.frame();
    for (let i = 0; i < 4 && h.phases.at(-1) !== 'active'; i++) await h.frame();
    const mounted = await h.pending;
    assert.equal(h.phases.at(-1), 'active');
    assert.equal(h.applications.length, 2);
    assert.deepEqual(h.applications.map(app => app.data), [[7], [7]]);
    assert.equal(h.applications[1].publishes, 1);
    assert.equal(h.loaderClosed(), true);
    assert.equal(h.retired(), 2);
    mounted.close(); await mounted.closed;
});

test('attempt cancellation breaks a stalled readiness/reveal loop', async () => {
    const h = harness({ready: false}); await flush(); await h.frame();
    assert.equal(h.phases.at(-1), 'ready');
    h.request('cancel'); await h.frame();
    assert.equal(h.phases.at(-1), 'cancelled');
    assert.equal(h.applications[0].isClosed, true);
    h.controller.abort(); await assert.rejects(h.pending, {name: 'AbortError'});
});

for (const atExit of [false, true]) test(`outer abort settles without another animation frame ${atExit ? 'during exit' : 'before reveal'}`, async () => {
    const h = harness({exitComplete: false}); await flush();
    if (atExit) { await h.frame(); assert.equal(h.phases.at(-1), 'revealing'); }
    h.controller.abort();
    await assert.rejects(h.pending, {name: 'AbortError'});
    assert.equal(h.loaderClosed(), true);
    assert.equal(h.applications[0].isClosed, true);
    assert.equal(h.applications[0].publishes, 0);
    assert.equal(h.retired(), 1);
});
