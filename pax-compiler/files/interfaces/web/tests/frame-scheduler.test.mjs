import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/utils/frame-scheduler.ts', import.meta.url), 'utf8');
const module = {exports:{}};
vm.runInNewContext(transformSync(source, {loader:'ts',format:'cjs'}).code, {module,exports:module.exports});
const {FrameScheduler} = module.exports;
test('default browser callbacks keep their native calling convention', () => {
    const nativeModule = {exports:{}};
    let callback;
    let cancelled;
    vm.runInNewContext(transformSync(source, {loader:'ts',format:'cjs'}).code, {
        module:nativeModule, exports:nativeModule.exports,
        requestAnimationFrame: function(fn) { assert.equal(this, undefined); callback = fn; return 7; },
        cancelAnimationFrame: function(id) { assert.equal(this, undefined); cancelled = id; },
    });
    const scheduler = new nativeModule.exports.FrameScheduler(false);
    scheduler.schedule(()=>{});
    assert.equal(typeof callback, 'function');
    scheduler.setSuspended(true);
    assert.equal(cancelled,7);
});
function harness(suspended = false) {
    const callbacks = new Map();
    let next = 0;
    const scheduler = new FrameScheduler(suspended, callback => {
        callbacks.set(++next, callback); return next;
    }, handle => callbacks.delete(handle));
    return {scheduler, callbacks, flush() {
        const pending = [...callbacks.values()]; callbacks.clear(); pending.forEach(fn=>fn());
    }};
}
test('startup suspension waits for resume, including a resume before startup', () => {
    const h = harness(true); let count = 0;
    h.scheduler.schedule(()=>count++);
    h.flush(); assert.equal(count,0);
    h.scheduler.setSuspended(false); h.flush(); assert.equal(count,1);
    const early = harness(true);
    early.scheduler.setSuspended(false); early.scheduler.schedule(()=>count++);
    early.flush(); assert.equal(count,2);
});
test('repeated pause/resume preserves one pending callback and its state', () => {
    const h = harness(); let count = 0;
    const frame = () => { count++; h.scheduler.schedule(frame); };
    h.scheduler.schedule(frame); h.flush(); assert.equal(count,1);
    h.scheduler.setSuspended(true); h.flush(); assert.equal(count,1);
    h.scheduler.setSuspended(false); h.scheduler.setSuspended(false);
    assert.equal(h.callbacks.size,1); h.flush(); assert.equal(count,2);
});
test('clear discards a suspended callback when the application is replaced', () => {
    const h = harness(true); let count = 0;
    h.scheduler.schedule(()=>count++); h.scheduler.clear();
    h.scheduler.setSuspended(false); h.flush(); assert.equal(count,0);
});
