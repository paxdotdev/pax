import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Exercise the real host boundary, including rAF's last feedback check.
const source = readFileSync(new URL('../src/index.ts', import.meta.url), 'utf8');
const runFrame = source.slice(source.indexOf('function progressStartupFrame('), source.indexOf('function renderLoop ('));
function harness(holdPresentation = false) {
    const state = {holdPresentation, renderPending: false, surfacesReady: true, patchesPending: true, enginePending: false, textReady: false, remainderPending: false, needsNextFlowChild: false, measurementChecks: 0, wakes: 0, ticks: 0, events: []};
    const chassis = {
        prepare_startup_presentation: () => !state.enginePending,
        accept_startup_measurements: () => {
            state.measurementChecks++;
            return !state.needsNextFlowChild;
        },
        startup_input_pending: () => (state.inputQueue?.length ?? 0) > 0,
        replay_startup_input: () => {
            if (state.inputHeld) return;
            state.events.push(state.inputQueue.shift());
            state.enginePending = true;
        },
        startup_pending: () => state.remainderPending,
        resume_startup_after_presentation: () => state.events.push('resume'),
        tick: () => { state.ticks++; return []; },
        render: () => state.events.push('render'),
        begin_render_work: () => state.events.push('begin-render-work'),
        continue_render_work: () => state.events.push('continue-render-work'),
        service_render_work: () => ({pending: state.renderPending || state.warmPending, visible_pending: state.renderPending}),
        finish_startup_presentation: () => state.events.push('input-ready'),
    };
    const mount = {removeAttribute: () => state.events.push('reveal')};
    const module = {exports: {}};
    vm.runInNewContext(transformSync(`
        let currentChassis = chassis, currentMount = mount;
        let frameInProgress = false, startupPresented = false, startupViewportPresented = false, startupViewportReady = false, startupFeedbackFrameReady = false;
        let pendingAsyncInterruptFlush = false;
        let startupRenderStarted = false;
        let ordinaryRenderPending = false;
        const mountYieldEnabled = true, perfTraceEnabled = false;
        const frameScheduler = {suspended: false};
        let startupWork = state;
        let startupScheduler = {request() { state.wakes++; }, dispose() { state.events.push('disposed'); }};
        const liveSession = {options: {holdPresentation: state.holdPresentation},
            didPrepare() { state.events.push('prepared'); },
            didMount() { state.events.push('mounted'); }, failStartup(error) { throw error; }};
        const sharedWorkBudget = {remaining: 4, run(budget, callback) { return callback(budget); }};
        const objectManager = {};
        const renderEpoch = 0;
        const cooperativeRender = {beginFrame() { state.events.push('render-slice'); }};
        const nativePool = {
            beginTraceFrame() {},
            sampleFrameInputs() {}, startupTextReady: () => state.textReady,
            startupSurfacesReady: () => state.surfacesReady,
            syncRenderSurfaceLayouts() {},
            finishTextFrame(settled) { state.events.push(settled ? 'settled-text' : 'ordinary-text'); },
        };
        function initializeChassis() {}
        function processMessages() {}
        function withProfileMeasure(name, callback) { return callback(); }
        ${runFrame}
        export function frame() { runFrame(chassis, mount); }
        export function detach() { currentChassis = null; }
        export function reveal() {
            assertReady();
            liveSession.options.holdPresentation = false;
            startupViewportPresented = true;
            chassis.resume_startup_after_presentation();
        }
        function assertReady() { if (!startupViewportReady) throw Error('viewport not prepared'); }
    `, {loader: 'ts', format: 'cjs'}).code, {module, exports: module.exports, state, chassis, mount});
    return {state, ...module.exports};
}

test('publication waits for patches, fonts, and a quiet feedback frame without advancing lifecycle', () => {
    const h = harness();
    h.frame(); assert.equal(h.state.wakes, 1);
    h.state.patchesPending = false;
    h.frame(); assert.deepEqual(h.state.events, []);
    h.state.textReady = true;
    h.frame(); assert.deepEqual(h.state.events, []);
    // A resize/measurement arrives after rAF but before the next candidate frame.
    h.state.enginePending = true;
    h.frame(); assert.equal(h.state.wakes, 2);
    h.state.enginePending = false;
    h.frame(); assert.deepEqual(h.state.events, []);
    h.frame();
    assert.deepEqual(h.state.events, ['render', 'reveal', 'settled-text', 'resume', 'input-ready', 'disposed', 'mounted']);
    assert.equal(h.state.ticks, 0);
    h.frame(); assert.equal(h.state.ticks, 1);
});

test('held application continues expensive rendering across frames before declaring readiness', () => {
    const h = harness(true);
    h.state.patchesPending = false;
    h.state.textReady = true;
    h.state.renderPending = true;
    h.frame(); h.frame(); h.frame();
    assert.equal(h.state.events.filter(e => e === 'begin-render-work').length, 1);
    assert.ok(h.state.events.includes('continue-render-work'));
    assert.ok(!h.state.events.includes('mounted'));
    h.state.renderPending = false;
    h.state.surfacesReady = false;
    h.frame();
    assert.ok(!h.state.events.includes('mounted'));
    h.state.surfacesReady = true;
    h.frame();
    assert.ok(h.state.events.includes('mounted'));
    assert.ok(!h.state.events.includes('disposed'));
    h.frame();
    assert.equal(h.state.ticks, 0, 'a prepared application still waits for publication');
});

test('a detached mount cannot publish from a stale animation-frame callback', () => {
    const h = harness();
    h.state.patchesPending = false; h.state.textReady = true;
    h.frame(); h.detach(); h.frame();
    assert.deepEqual(h.state.events, []);
    assert.equal(h.state.ticks, 0);
});

test('loader exit drains queued input during background mounting without advancing animations', () => {
    const h = harness(true);
    h.state.patchesPending = false; h.state.textReady = true; h.state.remainderPending = true;
    h.frame(); h.frame(); h.reveal();
    h.state.events.length = 0;
    h.state.inputQueue = ['click', 'scroll']; h.state.inputHeld = true;
    h.frame();
    assert.equal(h.state.inputQueue.length, 2, 'the loader still owns input');
    h.state.inputHeld = false;
    h.frame();
    assert.deepEqual(h.state.inputQueue, ['scroll'], 'one event per frame');
    assert.ok(h.state.events.includes('click'));
    h.frame();
    assert.deepEqual(h.state.inputQueue, []);
    assert.ok(h.state.events.indexOf('click') < h.state.events.indexOf('scroll'));
    assert.ok(h.state.wakes > 0, 'input-driven properties schedule startup settlement');
    assert.equal(h.state.ticks, 0);
    assert.ok(!h.state.events.includes('mounted'));
});

test('held viewport waits for submitted surfaces before admitting deferred mounting', () => {
    const h = harness(true);
    h.state.patchesPending = false; h.state.textReady = true;
    h.state.remainderPending = true; h.state.renderPending = true;
    h.frame(); h.frame();
    assert.ok(!h.state.events.includes('prepared'));
    assert.ok(!h.state.events.includes('resume'));
    h.state.renderPending = false; h.state.surfacesReady = false;
    h.frame();
    assert.ok(!h.state.events.includes('resume'));
    h.state.surfacesReady = true; h.frame();
    assert.ok(h.state.events.includes('prepared'));
    assert.ok(!h.state.events.includes('mounted'));
    assert.ok(!h.state.events.includes('input-ready'));
    assert.ok(!h.state.events.includes('resume'));
    h.reveal();
    h.state.enginePending = true; h.frame();
    assert.ok(!h.state.events.includes('disposed'));
    assert.equal(h.state.ticks, 0);
    h.state.enginePending = false; h.state.remainderPending = false;
    h.frame(); h.frame();
    assert.ok(h.state.events.includes('mounted'));
    assert.ok(h.state.events.includes('input-ready'));
    h.frame(); assert.equal(h.state.ticks, 1);
});

test('resize after viewport preparation requires fresh settlement and surface submission', () => {
    const h = harness(true);
    h.state.patchesPending = false; h.state.textReady = true; h.state.remainderPending = true;
    h.frame(); h.frame();
    assert.ok(h.state.events.includes('prepared'));
    h.state.events.length = 0;
    h.state.enginePending = true; h.frame();
    assert.throws(() => h.reveal(), /not prepared/);
    h.state.enginePending = false; h.state.surfacesReady = false;
    h.frame(); h.frame();
    assert.ok(!h.state.events.includes('prepared'));
    h.state.surfacesReady = true; h.frame();
    assert.ok(h.state.events.includes('prepared'));
    assert.ok(!h.state.events.includes('resume'));
});

test('cooperative rendering begins only after the startup publication checkpoint', () => {
    const h = harness();
    h.state.patchesPending = false;
    h.state.textReady = true;
    h.frame();
    assert.deepEqual(h.state.events, []);
    h.frame();
    assert.ok(h.state.events.includes('mounted'));
    assert.ok(!h.state.events.includes('render-slice'));
    assert.equal(h.state.ticks, 0);
    h.frame();
    assert.equal(h.state.ticks, 1);
    assert.ok(h.state.events.includes('begin-render-work'));
    assert.ok(h.state.events.includes('render-slice'));
});

test('first viewport waits for measurement, then native work progresses without releasing input or init', () => {
    const h = harness();
    h.state.patchesPending = false;
    h.state.textReady = true;
    h.state.remainderPending = true;
    h.frame(); h.frame();
    assert.deepEqual(h.state.events, ['render', 'reveal', 'settled-text', 'resume']);
    assert.equal(h.state.ticks, 0);
    h.state.events.length = 0;
    h.state.enginePending = true;
    h.state.textReady = false;
    h.frame();
    assert.deepEqual(h.state.events, ['ordinary-text', 'reveal']);
    h.state.events.length = 0;
    h.state.enginePending = false;
    h.frame();
    assert.deepEqual(h.state.events, ['ordinary-text', 'reveal']);
    h.state.textReady = true;
    h.frame(); h.frame();
    assert.ok(h.state.events.includes('resume'));
    assert.ok(!h.state.events.includes('mounted'));
    assert.equal(h.state.ticks, 0);
    h.state.events.length = 0;
    h.state.remainderPending = false;
    h.state.textReady = false;
    h.frame();
    assert.ok(!h.state.events.includes('mounted'));
    h.state.textReady = true;
    h.frame(); h.state.events.length = 0; h.frame();
    assert.deepEqual(h.state.events, ['continue-render-work', 'reveal', 'settled-text', 'resume', 'input-ready', 'disposed', 'mounted']);
    h.frame(); assert.equal(h.state.ticks, 1);
});

test('revealed viewport feedback waits for measurement and bounded rendering before resuming background mounts', () => {
    const h = harness();
    h.state.patchesPending = false; h.state.textReady = true; h.state.remainderPending = true;
    h.frame(); h.frame();
    h.state.events.length = 0;
    h.state.enginePending = true;
    h.frame();
    h.state.enginePending = false;
    h.state.needsNextFlowChild = true;
    h.frame(); h.frame();
    assert.ok(!h.state.events.includes('resume'));
    assert.ok(!h.state.events.includes('render'));
    h.state.needsNextFlowChild = false;
    h.state.renderPending = true;
    h.frame(); h.frame(); h.frame();
    assert.ok(h.state.events.includes('continue-render-work'));
    assert.ok(!h.state.events.includes('resume'));
    h.state.renderPending = false; h.state.surfacesReady = false;
    h.frame();
    assert.ok(!h.state.events.includes('resume'));
    h.state.surfacesReady = true; h.frame();
    assert.equal(h.state.events.filter(e => e === 'resume').length, 1);
    assert.ok(!h.state.events.includes('mounted'));
    assert.ok(!h.state.events.includes('input-ready'));
    assert.equal(h.state.ticks, 0);
});

test('optional offscreen rendering cannot hold a settled viewport or starve mounting', () => {
    const h = harness(true);
    h.state.patchesPending = false; h.state.textReady = true;
    h.state.remainderPending = true; h.state.warmPending = true;
    h.frame(); h.frame();
    assert.ok(h.state.events.includes('prepared'));
    h.reveal();
    h.state.events.length = 0;
    h.frame(); h.frame();
    assert.ok(h.state.events.includes('resume'));
    assert.ok(!h.state.events.includes('mounted'));
    h.state.remainderPending = false;
    h.frame(); h.frame();
    assert.ok(h.state.events.includes('mounted'));
});


test('flow continuation is admitted only after native measurement and a quiet frame', () => {
    const h = harness();
    h.state.patchesPending = false;
    h.state.needsNextFlowChild = true;
    h.frame();
    assert.equal(h.state.measurementChecks, 0);
    h.state.textReady = true;
    h.frame();
    assert.equal(h.state.measurementChecks, 0);
    h.frame();
    assert.equal(h.state.measurementChecks, 1);
    assert.deepEqual(h.state.events, []);
    assert.equal(h.state.wakes, 1);
    h.state.needsNextFlowChild = false;
    h.frame(); h.frame();
    assert.ok(h.state.events.includes('mounted'));
    assert.equal(h.state.ticks, 0);
});

test('a hidden startup receives a presentation opportunity without waiting for rAF', () => {
    const flush = source.slice(source.indexOf('function requestFrameFlush('),
        source.indexOf('// After the coordinated first reveal'));
    for (const hidden of [true, false]) {
        const calls = [];
        const chassis = {}, mount = {};
        vm.runInNewContext(transformSync(`
            const currentChassis = chassis, currentMount = mount;
            const frameInProgress = false, startupWork = {}, startupPresented = false;
            const startupScheduler = {request() { calls.push('work'); }};
            function runFrame() { calls.push('presentation'); }
            ${flush}
            requestFrameFlush(chassis, mount);
        `, {loader: 'ts'}).code, {chassis, mount, calls, document: {hidden}});
        assert.deepEqual(calls, hidden ? ['work', 'presentation'] : ['work']);
    }
});
