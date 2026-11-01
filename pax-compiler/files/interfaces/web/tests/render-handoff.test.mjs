import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Exercise the actual service callback that decides whether the next rAF can tick.
const source = readFileSync(new URL('../src/index.ts', import.meta.url), 'utf8');
const service = source.slice(source.indexOf('const pump = new CooperativeRenderScheduler('),
    source.indexOf('cooperativeRender = pump;'));
function harness() {
    const state = {remaining: 4, pendingProperties: false, report: {pending: true, visible_pending: true}};
    const module = {exports: {}};
    vm.runInNewContext(transformSync(`
        let ordinaryRenderPending = false, frameInProgress = false;
        const surfaceTraceEnabled = false;
        const chassis = {
            has_pending_properties: () => state.pendingProperties,
            render_work_is_settled: () => true,
            service_render_work: () => state.report,
        };
        const mount = {}, currentChassis = chassis, currentMount = mount;
        const frameScheduler = {suspended: false};
        const nativePool = {sampleFrameInputs() {}, syncRenderSurfaceLayouts() {}};
        const sharedWorkBudget = {
            get remaining() {return state.remaining;},
            run(budget, callback) {return callback(budget);},
        };
        function finishEngineTurn() {frameInProgress = false;}
        class CooperativeRenderScheduler {
            constructor(service) {this.service = service;}
        }
        ${service}
        export function run(fromFrame) {
            return {report: pump.service(fromFrame), holdsTick: ordinaryRenderPending};
        }
    `, {loader:'ts', format:'cjs'}).code, {state, module, exports:module.exports});
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
