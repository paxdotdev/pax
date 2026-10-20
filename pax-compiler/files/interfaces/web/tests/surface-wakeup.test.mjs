import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Run the real host attachment and frame paths, with fake chassis/DOM endpoints.
// The schedulers are real: this catches accidentally routing surface completions
// through requestFrameFlush even when each scheduler's unit tests pass.
function harness(cooperative) {
    const tasks = [], microtasks = [], calls = {tick: 0, layout: 0, render: 0};
    let pool, pending = false, exhausted = false, duringRender;
    class Pool {
        constructor() { pool = this; }
        attach() {}
        setPostAsyncInterruptFlush(fn) { this.interrupt = fn; }
        setSurfaceWorkWake(fn) { this.capacity = fn; }
        sampleFrameInputs() {}
        invalidateSurfaceHostObservation() {}
        syncRenderSurfaceLayouts() { calls.layout++; }
        beginTraceFrame() {}
        recordSurfaceCompletionRequest() {}
        recordSurfacePresentation() {}
    }
    class Channel {
        port1 = {};
        port2 = {postMessage: () => tasks.push(() => this.port1.onmessage())};
    }
    const globals = {URLSearchParams, performance, queueMicrotask: fn => microtasks.push(fn), MessageChannel: Channel,
        window: {location: {search: cooperative ? '?pax_render_yield=1' : ''}, addEventListener() {}, setInterval: () => 1},
        document: {hidden: false, addEventListener() {}}};
    function load(path, suffix = '') {
        const module = {exports: {}};
        const source = readFileSync(new URL(`../src/${path}.ts`, import.meta.url), 'utf8');
        vm.runInNewContext(transformSync(source + suffix, {loader: 'ts', format: 'cjs'}).code,
            {...globals, module, exports: module.exports, require: dependency});
        return module.exports;
    }
    function dependency(name) {
        if (name === './classes/native-element-pool') return {NativeElementPool: Pool};
        if (name === './pools/object-manager') return {ObjectManager: class {}};
        if (name.startsWith('./utils/') && /(?:frame-scheduler|surface-presentation|cooperative-render)$/.test(name)) {
            return load(name.slice(2));
        }
        return {};
    }
    const host = load('index', `
        export function testAttach(chassis, mount) {
            currentChassis = chassis; currentMount = mount; initializedChassis = true;
            attachChassis(chassis, mount);
            return {frame: () => runFrame(chassis, mount),
                replace: () => { currentChassis = null; },
                suspend: () => setSuspended(true)};
        }
    `);
    const draw = () => { calls.render++; duringRender?.(); return {pending: exhausted, allowance_exhausted: exhausted}; };
    const chassis = {set_property_waker() {}, activate_application() {},
        set_surface_ready_callback(fn) { this.complete = fn; },
        has_pending_properties: () => pending, render_work_is_settled: () => !pending,
        tick: () => { calls.tick++; pending = false; return []; },
        begin_render_work() {}, service_render_work: draw, present_ready_surfaces: draw, render: draw};
    const controls = host.testAttach(chassis, {});
    controls.frame();
    Object.assign(calls, {tick: 0, layout: 0, render: 0});
    function drain() {
        for (let limit = 0; microtasks.length || tasks.length; limit++) {
            assert.ok(limit < 20, 'completion work must not busy-loop');
            (microtasks.shift() ?? tasks.shift())();
        }
    }
    return {pool, chassis, calls, controls, drain, tasks, microtasks,
        pending: value => pending = value, exhausted: value => exhausted = value,
        duringRender: fn => duringRender = fn};
}

for (const cooperative of [false, true]) {
    test(`host completion/capacity bursts never run application ticks (yield=${cooperative})`, () => {
        const h = harness(cooperative);
        for (let i = 0; i < 9; i++) { h.pool.capacity(); h.chassis.complete(); }
        h.drain();
        assert.deepEqual(h.calls, {tick: 0, layout: 1, render: 1});
    });
    test(`pending application changes wait for settlement (yield=${cooperative})`, () => {
        const h = harness(cooperative);
        h.pending(true); h.pool.capacity(); h.drain();
        assert.deepEqual(h.calls, {tick: 0, layout: 0, render: 0});
        h.controls.frame(); h.drain();
        assert.equal(h.calls.tick, 1); assert.equal(h.calls.render, 1);
    });
    test(`old or suspended host completions cannot render (yield=${cooperative})`, () => {
        for (const action of ['replace', 'suspend']) {
            const h = harness(cooperative);
            h.pool.capacity(); h.controls[action](); h.drain();
            assert.deepEqual(h.calls, {tick: 0, layout: 0, render: 0});
        }
    });
}

test('cooperative completion during an active service is preserved without replenishing allowance', () => {
    const h = harness(true);
    h.exhausted(true);
    h.duringRender(() => { h.duringRender(undefined); h.pool.capacity(); });
    h.pool.capacity(); h.drain();
    assert.equal(h.calls.tick, 0);
    assert.equal(h.calls.render, 2, 'one coalesced follow-up observes the exhausted allowance, then sleeps');
    h.exhausted(false); h.controls.frame(); h.drain();
    assert.equal(h.calls.tick, 1);
    assert.equal(h.calls.render, 3, 'ordinary frame resumes progress');
});

test('synchronous completion during presentation leaves progress to the next ordinary frame', () => {
    const h = harness(false);
    h.duringRender(() => { h.duringRender(undefined); h.pool.capacity(); });
    h.pool.capacity(); h.drain();
    assert.deepEqual(h.calls, {tick: 0, layout: 1, render: 1});
    h.controls.frame(); h.drain();
    assert.deepEqual(h.calls, {tick: 1, layout: 2, render: 2});
});
