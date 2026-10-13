import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const source = readFileSync(new URL('../src/utils/startup-scheduler.ts', import.meta.url), 'utf8');
const module = {exports:{}};
vm.runInNewContext(transformSync(source,{loader:'ts',format:'cjs'}).code,{module,exports:module.exports});
const {StartupScheduler} = module.exports;
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
