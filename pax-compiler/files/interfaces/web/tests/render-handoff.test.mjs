import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Exercise the actual service callback that decides whether the next rAF can tick.
const source = readFileSync(new URL('../src/index.ts', import.meta.url), 'utf8');
const service = source.slice(source.indexOf('const pump = new CooperativeRenderScheduler('),
    source.indexOf('cooperativeRender = pump;'));
const schedulerModule = {exports: {}};
vm.runInNewContext(transformSync(readFileSync(new URL('../src/utils/cooperative-render.ts', import.meta.url), 'utf8'),
    {loader:'ts', format:'cjs'}).code, {module:schedulerModule, exports:schedulerModule.exports});
function harness() {
    const state = {remaining: 4, pendingProperties: false, settled: true, calls: [],
        tasks: [], finishes: 0, observe: () => {}, report: {pending: true, visible_pending: true}};
    // Use the real coalescing/wakeup behavior, with explicit host-task delivery.
    class Scheduler extends schedulerModule.exports.CooperativeRenderScheduler {
        constructor(service, allowed) { super(service, allowed, job => state.tasks.push(job)); }
    }
    const module = {exports: {}};
    vm.runInNewContext(transformSync(`
        let ordinaryRenderPending = false, frameInProgress = false;
        const surfaceTraceEnabled = false;
        const chassis = {
            has_pending_properties: () => state.pendingProperties,
            render_work_is_settled: () => state.settled,
            service_render_work: () => {state.calls.push('render'); return state.report;},
        };
        const mount = {}, currentChassis = chassis, currentMount = mount;
        const frameScheduler = {suspended: false};
        const nativePool = {
            sampleFrameInputs() {throw Error('continuation published application input');},
            observeFrameInputs() {state.calls.push('observe'); state.observe();},
            syncRenderSurfaceLayouts() {state.calls.push('layout');},
        };
        const sharedWorkBudget = {
            get remaining() {return state.remaining;},
            run(budget, callback) {return callback(budget);},
        };
        function finishEngineTurn() {frameInProgress = false; state.finishes++;}
        ${service}
        export function run(fromFrame) {
            return {report: pump.service(fromFrame), holdsTick: ordinaryRenderPending, frameInProgress};
        }
        export function beginFrame() {pump.beginFrame();}
        export function wake() {return pump.request();}
    `, {loader:'ts', format:'cjs'}).code, {state, module, exports:module.exports, CooperativeRenderScheduler:Scheduler});
    return {state, ...module.exports};
}

test('finishing visible drawing releases ticks while offscreen work remains queued', () => {
    const h = harness();
    assert.equal(h.run(true).holdsTick, true);
    h.state.report = {pending:true, visible_pending:false};
    const result = h.run(false);
    assert.equal(result.holdsTick, false);
    assert.equal(result.report.pending, true, 'the render pump still owns optional work');
    h.state.remaining = 0;
    assert.equal(h.run(false).holdsTick, false, 'an exhausted warm continuation cannot freeze ticks');
    assert.equal(h.run(true).holdsTick, true, 'a new scene still requires classification');
});

test('required visible work retains the hold across exhausted slices', () => {
    const h = harness();
    h.run(true);
    h.state.remaining = 0;
    assert.equal(h.run(false).holdsTick, true);
    h.state.remaining = 4;
    h.state.report = {pending:true, visible_pending:true, blocked:true};
    assert.equal(h.run(false).holdsTick, false, 'blocked work must return to ordinary settlement');
});

test('continuations observe demand without publishing scroll input or bypassing settlement', () => {
    const h = harness();
    h.run(false);
    assert.deepEqual(h.state.calls, ['observe', 'layout', 'render']);
    for (const reason of ['pendingProperties', 'settled']) {
        h.state.calls.length = 0;
        h.state.pendingProperties = reason === 'pendingProperties';
        h.state.settled = reason !== 'settled';
        assert.equal(h.run(false).report.blocked, true);
        assert.deepEqual(h.state.calls, ['observe'], 'genuine changes still require an application tick');
    }
});

test('exhausted continuations skip input/layout work without leaking the engine-entry guard', () => {
    const h = harness();
    h.state.remaining = 0;
    const result = h.run(false);
    assert.equal(result.report.allowance_exhausted, true);
    assert.equal(result.holdsTick, false);
    assert.equal(result.frameInProgress, false);
    assert.equal(h.state.finishes, 1);
    assert.deepEqual(h.state.calls, []);
    assert.equal(h.run(true).holdsTick, true, 'unclassified frame still needs drawing before ticking again');
    assert.deepEqual(h.state.calls, []);
});

test('resuming an exhausted continuation observes fresh input and still requires settlement', () => {
    const h = harness();
    h.state.remaining = 0;
    h.run(false);
    h.state.observe = () => {h.state.settled = false;};
    h.state.remaining = 4;
    assert.equal(h.run(false).report.blocker, 'host_input_requires_settlement');
    assert.deepEqual(h.state.calls, ['observe']);
    h.state.calls.length = 0;
    h.state.observe = () => {};
    h.state.settled = true;
    h.run(false);
    assert.deepEqual(h.state.calls, ['observe', 'layout', 'render']);
});

test('completion wakes survive exhausted preflight and a fresh frame resumes work once', () => {
    const h = harness();
    h.state.remaining = 0;
    h.beginFrame();
    assert.equal(h.state.tasks.length, 0, 'exhaustion does not self-spin');
    assert.equal(h.wake(), 'queued');
    assert.equal(h.wake(), 'coalesced');
    h.state.tasks.shift()();
    assert.deepEqual(h.state.calls, [], 'completion does not replenish the frame allowance');
    assert.equal(h.state.tasks.length, 0);
    // A completion can remain queued as the next ordinary frame arrives.
    h.wake();
    h.state.remaining = 4;
    h.state.report = {pending:false, visible_pending:false};
    h.beginFrame();
    assert.deepEqual(h.state.calls, ['render']);
    h.state.tasks.shift()();
    assert.deepEqual(h.state.calls, ['render'], 'stale task cannot duplicate completed work');
    h.wake();
    h.state.tasks.shift()();
    assert.deepEqual(h.state.calls, ['render', 'observe', 'layout', 'render']);
});
