import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const source = readFileSync(new URL('../src/utils/startup-scheduler.ts', import.meta.url), 'utf8');
const module = {exports:{}};
vm.runInNewContext(transformSync(source,{loader:'ts',format:'cjs'}).code,{module,exports:module.exports});
const {StartupScheduler, StartupWork} = module.exports;
test('one service per task, coalesced wakeups, no recursive continuation', () => {
    const queue = []; let calls = 0;
    const scheduler = new StartupScheduler(() => ++calls < 3, () => true, job => queue.push(job));
    scheduler.request(); scheduler.request(); assert.equal(queue.length,1);
    queue.shift()(); assert.equal(calls,1); assert.equal(queue.length,1);
    queue.shift()(); queue.shift()(); assert.equal(calls,3); assert.equal(queue.length,0);
});
test('suspension parks work, resume requests it, replacement drops stale tasks', () => {
    const queue = []; let allowed = false, calls = 0;
    const scheduler = new StartupScheduler(() => {calls++; return true;}, () => allowed, job => queue.push(job));
    scheduler.request(); queue.shift()(); assert.equal(calls,0); assert.equal(queue.length,0);
    allowed = true; scheduler.request(); queue.shift()(); assert.equal(calls,1);
    scheduler.dispose(); queue.shift()(); scheduler.request(); assert.equal(calls,1); assert.equal(queue.length,0);
});

test('engine and patches share a deadline; backpressure preserves create/update/delete order', () => {
    let time = 0, engineCalls = 0;
    const applied = [], budgets = [];
    const work = new StartupWork(() => engineCalls === 2, budget => {
        budgets.push(budget);
        time += 3;
        return ++engineCalls === 1 ? ['create', 'update', 'delete'] : ['create-again'];
    }, message => { applied.push(message); time += 2; }, () => time);
    assert.equal(work.service(4), true);
    assert.deepEqual(applied, ['create']); // This atomic patch ends after the deadline.
    assert.equal(work.service(4), true);
    assert.deepEqual(applied, ['create', 'update', 'delete']);
    assert.equal(engineCalls, 1, 'no engine service while draining an old batch');
    assert.equal(work.service(4), false);
    assert.deepEqual(applied, ['create', 'update', 'delete', 'create-again']);
    assert.deepEqual(budgets, [4, 4]);
    assert.equal(work.patchesPending, false);
});

test('an engine overrun leaves patches for another task; zero-budget slices still progress', () => {
    let time = 0, ready = false, patches = 0;
    const work = new StartupWork(() => ready, () => { time += 20; ready = true; return [1, 2]; }, () => ++patches, () => time);
    assert.equal(work.service(4), true);
    assert.equal(patches, 0);
    assert.equal(work.service(0), true);
    assert.equal(patches, 1);
    assert.equal(work.service(0), false);
    assert.equal(patches, 2);
});

test('feedback reopens service without an ordinary tick; disposal abandons queued patches', () => {
    let ready = true, applied = 0;
    const work = new StartupWork(() => ready, () => { ready = true; return ['patch']; }, () => ++applied, () => 0);
    assert.equal(work.service(4), false);
    ready = false;
    const tasks = [];
    const scheduler = new StartupScheduler(() => work.service(0), () => true, task => tasks.push(task));
    scheduler.request(); tasks.shift()();
    assert.equal(work.patchesPending, true);
    scheduler.dispose(); tasks.shift()();
    assert.equal(applied, 0);
});
