import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const module = {exports: {}};
const source = readFileSync(new URL('../src/utils/surface-presentation.ts', import.meta.url), 'utf8');
let defaultJob;
vm.runInNewContext(transformSync(source, {loader:'ts',format:'cjs'}).code, {
    module,exports:module.exports,
    queueMicrotask: function(job) {
        assert.notEqual(this?.constructor?.name, 'SurfacePresentationScheduler');
        defaultJob = job;
    },
});
const {SurfacePresentationScheduler} = module.exports;

test('default microtask is called as a global, not with the scheduler as receiver', () => {
    let calls = 0;
    const scheduler = new SurfacePresentationScheduler(() => calls++, () => true);
    scheduler.request(); defaultJob();
    assert.equal(calls, 1);
});

test('completion bursts get one presentation opportunity without advancing application frames', () => {
    const jobs = []; let calls = 0;
    const scheduler = new SurfacePresentationScheduler(() => calls++, () => true, job => jobs.push(job));
    scheduler.request(); scheduler.request();
    assert.equal(jobs.length, 1); assert.equal(calls, 0);
    jobs.shift()(); scheduler.request(); scheduler.present();
    assert.equal(calls, 1); assert.equal(jobs.length, 0);
    scheduler.afterFrame(); scheduler.request(); jobs.shift()();
    assert.equal(calls, 2);
});

test('disposal suppresses queued old-chassis completions; suspension does not consume a pass', () => {
    const jobs = []; let calls = 0, allowed = false;
    const scheduler = new SurfacePresentationScheduler(() => calls++, () => allowed, job => jobs.push(job));
    scheduler.request(); jobs.shift()(); assert.equal(calls, 0);
    allowed = true; scheduler.present(); assert.equal(calls, 1);
    scheduler.afterFrame(); scheduler.request(); scheduler.dispose(); jobs.shift()();
    scheduler.afterFrame(); scheduler.request(); scheduler.present();
    assert.equal(calls, 1); assert.equal(jobs.length, 0);
});
