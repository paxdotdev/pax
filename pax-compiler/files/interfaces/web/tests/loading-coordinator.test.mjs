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
function harness({cancelOnFirstMount = false, ready = true, exitComplete = true, mounting = false, splitAdapter, loaderExit} = {}) {
    const jobs = new Map(), phases = [], applications = [], elements = [];
    let nextJob = 0, action, loaderClosed = false, exits = 0, retired = 0, loaders = 0;
    const frame = callback => { const id = ++nextJob; jobs.set(id, callback); return id; };
    const controller = new AbortController();
    const makeLoader = () => ({closed: Promise.resolve(), publish: () => true, close() { loaderClosed = true; }, chassis: {
        loading_update(phase) { phases.push(phase); },
        take_loading_request() { const request = action; action = undefined; return request; },
        begin_presentation_exit() { exits++; }, presentation_exit_complete: () => exitComplete,
    }});
    const adapter = splitAdapter ?? {load: async () => {}, observe: () => () => {}, state: {}};
    const module = {exports: {}};
    vm.runInNewContext(transformSync(abortSource + '\n' + source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, AbortController, WebAssembly, URL,
        document: {baseURI: 'https://example.test/', createElement: () => {
            const element = {style: {}, dataset: {}, hidden: false, appendChild() {}, remove() {}};
            elements.push(element); return element;
        }},
        requestAnimationFrame: frame, cancelAnimationFrame: id => jobs.delete(id), requestPreparationFrame: frame,
        getSplit: async () => ({loader: adapter, acquire: () => {
            let released = false; return () => { if (!released) { released = true; retired++; } };
        }}),
    });
    const pending = module.exports.startLoadingApplication({element: {appendChild() {}}, cartridgeUrl: '/app', signal: controller.signal,
        startupData: new Uint8Array([7]), loaderExit}, {}, () => ({initApplication: async options => {
        if (options.loader) {
            assert.equal(elements[2].hidden, false, 'loader preparation needs a slot that participates in layout');
            loaders++; return makeLoader();
        }
        let acknowledge, finish, fail;
        const app = {closed: new Promise(resolve => { acknowledge = resolve; }), isClosed: false, reveals: 0, publishes: 0,
            mounted: new Promise((resolve, reject) => { finish = resolve; fail = reject; }),
            finish() { finish(); }, fail(error) { fail(error); },
            data: [...options.startupData],
            close() { this.isClosed = true; acknowledge(); },
            reveal() { this.reveals++; return ready && !this.isClosed; },
            publish() { this.publishes++; return !this.isClosed; },
        };
        applications.push(app);
        if (!mounting) app.finish();
        options.signal.addEventListener('abort', () => app.close(), {once: true});
        if (cancelOnFirstMount && applications.length === 1) action = 'cancel';
        return app;
    }}));
    return {pending, controller, phases, applications, request(value) { action = value; },
        loaderHidden: () => elements[2].hidden,
        setReady(value) { ready = value; }, setExitComplete(value) { exitComplete = value; },
        exits: () => exits, retired: () => retired, loaders: () => loaders, loaderClosed: () => loaderClosed,
        async frame() { const callbacks = [...jobs.values()]; jobs.clear(); for (const callback of callbacks) { callback(); await flush(); } },
    };
}

test('a cancel queued at mount completion reaches cancelled, then retries without publishing the retired app', async () => {
    const h = harness({cancelOnFirstMount: true});
    await flush(); assert.equal(h.phases.at(-1), 'preparing');
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

test('viewport readiness resumes mounting without exiting the loader onto a paused application', async () => {
    const h = harness({mounting: true}); await flush();
    await h.frame(); await h.frame();
    assert.equal(h.phases.at(-1), 'preparing');
    assert.equal(h.applications[0].reveals, 1, 'the first viewport must release deferred mounting');
    assert.equal(h.exits(), 0, 'keep the active loader until the application clock can advance');
    assert.equal(h.applications[0].publishes, 0);
    let finished = false; h.pending.then(() => { finished = true; });
    await flush(); assert.equal(finished, false);
    h.applications[0].finish(); await flush();
    assert.equal(h.phases.at(-1), 'ready');
    assert.equal(h.exits(), 0);
    await h.frame();
    assert.equal(h.phases.at(-1), 'revealing');
    assert.equal(h.exits(), 1);
    assert.equal(h.applications[0].publishes, 0, 'input waits for loader exit');
    await h.frame();
    await h.pending;
    assert.equal(h.phases.at(-1), 'active');
    assert.equal(h.applications[0].publishes, 1);
});

test('first-viewport policy exits at settled readiness but keeps publication pending until full mount', async () => {
    const h = harness({loaderExit: 'first_viewport', mounting: true, ready: false, exitComplete: false});
    await flush(); await h.frame();
    assert.equal(h.phases.at(-1), 'preparing');
    assert.equal(h.exits(), 0, 'unsettled native content cannot start exit');
    h.setReady(true); await h.frame();
    assert.equal(h.phases.at(-1), 'ready');
    await h.frame();
    assert.equal(h.exits(), 1);
    assert.equal(h.phases.at(-1), 'revealing');
    assert.equal(h.loaderHidden(), false, 'authored exit must finish before hiding the loader slot');
    let finished = false; h.pending.then(() => { finished = true; });
    h.setExitComplete(true); await h.frame();
    assert.equal(h.loaderHidden(), true, 'even a loader without a fade ends before the full mount');
    assert.equal(finished, false);
    assert.equal(h.applications[0].publishes, 0);
    h.applications[0].finish(); await flush();
    const mounted = await h.pending;
    assert.equal(h.applications[0].publishes, 1);
    assert.equal(h.phases.at(-1), 'active');
    mounted.close(); await mounted.closed;
});

for (const exited of [false, true]) test(`first-viewport failure ${exited ? 'after' : 'during'} exit restores a visible loader and retries`, async () => {
    const h = harness({loaderExit: 'first_viewport', mounting: true, exitComplete: exited});
    await flush(); await h.frame(); await h.frame(); await h.frame();
    assert.equal(h.exits(), 1);
    assert.equal(h.loaderHidden(), exited);
    h.applications[0].fail(new Error('late mount failure')); await flush();
    assert.equal(h.phases.at(-1), 'failed');
    assert.equal(h.loaders(), 2);
    assert.equal(h.loaderHidden(), false);
    assert.equal(h.applications[0].isClosed, true);
    assert.equal(h.applications[0].publishes, 0);
    h.request('retry'); await h.frame();
    assert.equal(h.applications.length, 2);
    h.setExitComplete(true); h.applications[1].finish();
    for (let i = 0; i < 4 && h.phases.at(-1) !== 'active'; i++) await h.frame();
    const mounted = await h.pending;
    assert.equal(h.phases.at(-1), 'active');
    assert.equal(h.applications[1].publishes, 1);
    mounted.close(); await mounted.closed;
});

test('abort after first-viewport loader exit cancels remaining mounts without publishing input', async () => {
    const h = harness({loaderExit: 'first_viewport', mounting: true});
    await flush(); await h.frame(); await h.frame(); await h.frame();
    assert.equal(h.loaderHidden(), true);
    h.controller.abort(); await assert.rejects(h.pending, {name: 'AbortError'});
    assert.equal(h.applications[0].isClosed, true);
    assert.equal(h.applications[0].publishes, 0);
});

test('failure during remaining mounts keeps the existing loader for error and retry', async () => {
    const h = harness({mounting: true, exitComplete: false}); await flush(); await h.frame();
    assert.equal(h.phases.at(-1), 'preparing');
    h.applications[0].fail(new Error('late mount failure')); await flush();
    assert.equal(h.phases.at(-1), 'failed');
    assert.equal(h.loaders(), 1, 'the loader has not exited while the main app is still mounting');
    assert.equal(h.exits(), 0);
    assert.equal(h.applications[0].isClosed, true);
    assert.equal(h.applications[0].publishes, 0);
    h.controller.abort(); await assert.rejects(h.pending, {name: 'AbortError'});
});

test('abort during remaining mounts cancels the wait without exposing the paused application', async () => {
    const h = harness({mounting: true}); await flush(); await h.frame(); await h.frame();
    assert.equal(h.exits(), 0);
    h.controller.abort(); await assert.rejects(h.pending, {name: 'AbortError'});
    assert.equal(h.applications[0].isClosed, true);
    assert.equal(h.applications[0].publishes, 0);
});

test('a late mount failure retries with the active loader and publishes only the replacement application', async () => {
    const h = harness({mounting: true}); await flush(); await h.frame(); await h.frame();
    h.applications[0].fail(new Error('late mount failure')); await flush();
    assert.equal(h.phases.at(-1), 'failed');
    h.request('retry'); await h.frame();
    assert.equal(h.applications.length, 2);
    h.applications[1].finish();
    for (let i = 0; i < 4 && h.phases.at(-1) !== 'active'; i++) await h.frame();
    const mounted = await h.pending;
    assert.equal(h.phases.at(-1), 'active');
    assert.equal(h.loaders(), 1);
    assert.equal(h.applications[0].publishes, 0);
    assert.equal(h.applications[1].publishes, 1);
    assert.equal(h.retired(), 2);
    mounted.close(); await mounted.closed;
});

test('attempt cancellation breaks a stalled readiness/reveal loop', async () => {
    const h = harness({ready: false}); await flush(); await h.frame();
    assert.equal(h.phases.at(-1), 'preparing');
    h.request('cancel'); await h.frame();
    assert.equal(h.phases.at(-1), 'cancelled');
    assert.equal(h.applications[0].isClosed, true);
    h.controller.abort(); await assert.rejects(h.pending, {name: 'AbortError'});
});

for (const atExit of [false, true]) test(`outer abort settles without another animation frame ${atExit ? 'during exit' : 'before reveal'}`, async () => {
    const h = harness({exitComplete: false}); await flush();
    if (atExit) { await h.frame(); await h.frame(); assert.equal(h.phases.at(-1), 'revealing'); }
    h.controller.abort();
    await assert.rejects(h.pending, {name: 'AbortError'});
    assert.equal(h.loaderClosed(), true);
    assert.equal(h.applications[0].isClosed, true);
    assert.equal(h.applications[0].publishes, 0);
    assert.equal(h.retired(), 1);
});

test('closing a view during async installation never mounts it when the shared install finishes', async () => {
    let finish;
    const installation = new Promise(resolve => { finish = resolve; });
    const observers = new Set();
    const adapter = {
        load: () => installation,
        observe(callback) { observers.add(callback); return () => observers.delete(callback); },
        state: {phase: 'installing'},
    };
    const retired = harness({splitAdapter: adapter});
    await flush();
    for (const observer of observers) observer(adapter.state);
    assert.equal(retired.phases.at(-1), 'preparing');
    retired.controller.abort();
    await assert.rejects(retired.pending, {name: 'AbortError'});
    assert.equal(retired.loaderClosed(), true);
    assert.equal(retired.retired(), 1);
    assert.equal(observers.size, 0);
    const phasesAtClose = [...retired.phases];

    // A later view can wait for the same uncancellable installation.
    const live = harness({splitAdapter: adapter});
    await flush();
    assert.equal(live.applications.length, 0);
    finish();
    await flush();
    for (let i = 0; i < 4 && live.phases.at(-1) !== 'active'; i++) await live.frame();
    const mounted = await live.pending;
    assert.equal(live.phases.at(-1), 'active');
    assert.equal(live.applications.length, 1);
    assert.equal(retired.applications.length, 0);
    assert.deepEqual(retired.phases, phasesAtClose);
    assert.equal(observers.size, 0);
    mounted.close(); await mounted.closed;
});
