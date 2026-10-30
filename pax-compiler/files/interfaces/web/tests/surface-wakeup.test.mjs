import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Run the real host attachment and frame paths, with fake chassis/DOM endpoints.
// The schedulers are real: this catches accidentally routing surface completions
// through requestFrameFlush even when each scheduler's unit tests pass.
function harness(search = "") {
    const tasks = [], microtasks = [], frames = [], calls = {tick: 0, layout: 0, render: 0};
    let pool, pending = false, exhausted = false, duringRender, time = 0, renderCost = 0;
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
        finishTextFrame() {}
    }
    class Channel {
        port1 = {};
        port2 = {postMessage: () => tasks.push(() => this.port1.onmessage())};
    }
    const globals = {URLSearchParams, performance: {now: () => time}, queueMicrotask: fn => microtasks.push(fn), MessageChannel: Channel, requestAnimationFrame: fn => {frames.push(fn); return frames.length;},
        window: {location: {search}, addEventListener() {}, setInterval: () => 1},
        document: {hidden: false, addEventListener() {}}};
    function load(path, suffix = '') {
        const module = {exports: {}};
        let source = readFileSync(new URL(`../src/${path}.ts`, import.meta.url), 'utf8');
        if (path === 'index') {
            // Instantiate the same per-application closure and expose only test controls.
            source = source.replace('    const host = {initApplication, setSuspended};',
                suffix.replace('export function testAttach', 'function testAttach') +
                '\n    const host = {initApplication, setSuspended, testAttach};');
            suffix = '\nexport const testAttach = createApplicationHost().testAttach;';
        }
        vm.runInNewContext(transformSync(source + suffix, {loader: 'ts', format: 'cjs'}).code,
            {...globals, module, exports: module.exports, require: dependency});
        return module.exports;
    }
    function dependency(name) {
        if (name === './classes/native-element-pool') return {NativeElementPool: Pool};
        if (name === './pools/object-manager') return {ObjectManager: class {}};
        if (name.startsWith('./utils/') && /(?:frame-scheduler|cooperative-render|startup-scheduler|shared-work-budget)$/.test(name)) {
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
    const draw = () => { calls.render++; time += renderCost; duringRender?.(); return {pending: exhausted, allowance_exhausted: exhausted}; };
    const chassis = {set_property_waker() {}, activate_application() {},
        startup_pending: () => false, startup_input_pending: () => false,
        set_surface_ready_callback(fn) { this.complete = fn; },
        has_pending_properties: () => pending, render_work_is_settled: () => !pending,
        tick: () => { calls.tick++; pending = false; return []; },
        begin_render_work() {}, continue_render_work() {}, service_render_work: draw, present_ready_surfaces: draw, render: draw};
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
        duringRender: fn => duringRender = fn, renderCost: value => renderCost = value,
        nextFrame: () => { for (const frame of frames.splice(0)) frame(); }};
}

for (const search of ["", "?pax_render_yield=0&pax_wheel_passive=0&pax_frame_clip=svg"]) {
    test(`host completion/capacity bursts never run application ticks (query=${search})`, () => {
        const h = harness(search);
        for (let i = 0; i < 9; i++) { h.pool.capacity(); h.chassis.complete(); }
        h.drain();
        assert.deepEqual(h.calls, {tick: 0, layout: 1, render: 1});
    });
    test(`pending application changes wait for settlement (query=${search})`, () => {
        const h = harness(search);
        h.pending(true); h.pool.capacity(); h.drain();
        assert.deepEqual(h.calls, {tick: 0, layout: 0, render: 0});
        h.controls.frame(); h.drain();
        assert.equal(h.calls.tick, 1); assert.equal(h.calls.render, 1);
    });
    test(`old or suspended host completions cannot render (query=${search})`, () => {
        for (const action of ['replace', 'suspend']) {
            const h = harness(search);
            h.pool.capacity(); h.controls[action](); h.drain();
            assert.deepEqual(h.calls, {tick: 0, layout: 0, render: 0});
        }
    });
}

test('cooperative completion during an active service is preserved without replenishing allowance', () => {
    const h = harness();
    h.exhausted(true);
    h.duringRender(() => { h.duringRender(undefined); h.pool.capacity(); });
    h.pool.capacity(); h.drain();
    assert.equal(h.calls.tick, 0);
    assert.equal(h.calls.render, 2, 'one coalesced follow-up observes the exhausted allowance, then sleeps');
    h.exhausted(false); h.controls.frame(); h.drain();
    assert.equal(h.calls.tick, 0);
    assert.equal(h.calls.render, 3, 'a later display frame finishes the settled drawing');
    h.controls.frame();
    assert.equal(h.calls.tick, 1);
});

test('new application work preempts carried drawing and settles before rendering', () => {
    const h = harness();
    h.exhausted(true); h.controls.frame();
    h.pending(true); h.exhausted(false); h.controls.frame();
    assert.equal(h.calls.tick, 2);
    assert.equal(h.calls.render, 2);
});

test('published drawing can continue after setup exceeds the preparation allowance', () => {
    const h = harness();
    h.renderCost(8);
    h.exhausted(true);
    h.controls.frame();
    h.exhausted(false);
    h.nextFrame();
    h.controls.frame();
    h.drain();
    assert.equal(h.calls.tick, 1);
    assert.equal(h.calls.render, 2, 'setup must not starve drawing until the next tick resets it');
});

test('completion during presentation gets a host continuation without an application tick', () => {
    const h = harness();
    h.duringRender(() => { h.duringRender(undefined); h.pool.capacity(); });
    h.pool.capacity(); h.drain();
    assert.deepEqual(h.calls, {tick: 0, layout: 2, render: 2});
    h.controls.frame(); h.drain();
    assert.deepEqual(h.calls, {tick: 1, layout: 3, render: 3});
});
