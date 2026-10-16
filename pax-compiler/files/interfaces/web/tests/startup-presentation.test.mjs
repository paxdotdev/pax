import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Exercise the real host boundary, including rAF's last feedback check.
const source = readFileSync(new URL('../src/index.ts', import.meta.url), 'utf8');
const runFrame = source.slice(source.indexOf('function previewStartupFrame('), source.indexOf('function renderLoop ('));
function harness(preview = false) {
    const state = {patchesPending: true, enginePending: false, textReady: false, remainderPending: false, needsNextFlowChild: false, measurementChecks: 0, wakes: 0, ticks: 0, events: []};
    const chassis = {
        prepare_startup_presentation: () => !state.enginePending,
        accept_startup_measurements: () => {
            state.measurementChecks++;
            return !state.needsNextFlowChild;
        },
        startup_input_pending: () => false,
        startup_pending: () => state.remainderPending,
        resume_startup_after_presentation: () => state.events.push('resume'),
        tick: () => { state.ticks++; return []; },
        render: () => state.events.push('render'),
        finish_startup_presentation: () => state.events.push('input-ready'),
    };
    const mount = {removeAttribute: () => state.events.push('reveal')};
    const module = {exports: {}};
    vm.runInNewContext(transformSync(`
        let currentChassis = chassis, currentMount = mount;
        let frameInProgress = false, startupPresented = false, startupViewportPresented = false, startupFeedbackFrameReady = false;
        let pendingAsyncInterruptFlush = false;
        const mountYieldEnabled = true, perfTraceEnabled = false, startupPreviewEnabled = preview;
        const frameScheduler = {suspended: false};
        let startupWork = state;
        let startupScheduler = {request() { state.wakes++; }, dispose() { state.events.push('disposed'); }};
        const liveSession = {didMount() { state.events.push('mounted'); }};
        const surfacePresentation = undefined, objectManager = {};
        const nativePool = {
            sampleFrameInputs() {}, startupTextReady: () => state.textReady,
            syncRenderSurfaceLayouts() {},
            finishTextFrame(settled) { state.events.push(settled ? 'settled-text' : 'ordinary-text'); },
        };
        function initializeChassis() {}
        function processMessages() {}
        function withProfileMeasure(name, callback) { return callback(); }
        ${runFrame}
        export function frame() { runFrame(chassis, mount); }
        export function detach() { currentChassis = null; }
    `, {loader: 'ts', format: 'cjs'}).code, {module, exports: module.exports, state, chassis, mount, preview});
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

test('a detached mount cannot publish from a stale animation-frame callback', () => {
    const h = harness();
    h.state.patchesPending = false; h.state.textReady = true;
    h.frame(); h.detach(); h.frame();
    assert.deepEqual(h.state.events, []);
    assert.equal(h.state.ticks, 0);
});

test('preview reveals native readiness without rendering unfinished canvas work or completing startup', () => {
    const h = harness(true);
    h.state.enginePending = true;
    h.frame();
    assert.deepEqual(h.state.events, ['ordinary-text', 'reveal']);
    assert.equal(h.state.ticks, 0);
    h.state.events.length = 0;
    h.state.patchesPending = false; h.state.enginePending = false;
    h.frame();
    assert.deepEqual(h.state.events, ['render', 'ordinary-text', 'reveal']);
    assert.equal(h.state.ticks, 0);
    h.state.events.length = 0;
    h.state.textReady = true;
    h.frame(); h.state.events.length = 0; h.frame();
    assert.deepEqual(h.state.events, ['render', 'reveal', 'settled-text', 'resume', 'input-ready', 'disposed', 'mounted']);
    h.frame(); assert.equal(h.state.ticks, 1);
});


test('first viewport reveals together, then native work progresses without releasing input or init', () => {
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
    assert.deepEqual(h.state.events, ['render', 'reveal', 'ordinary-text', 'resume']);
    assert.equal(h.state.ticks, 0);
    h.state.events.length = 0;
    h.state.remainderPending = false;
    h.frame();
    assert.ok(!h.state.events.includes('mounted'));
    h.state.textReady = true;
    h.frame(); h.state.events.length = 0; h.frame();
    assert.deepEqual(h.state.events, ['render', 'reveal', 'settled-text', 'resume', 'input-ready', 'disposed', 'mounted']);
    h.frame(); assert.equal(h.state.ticks, 1);
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
