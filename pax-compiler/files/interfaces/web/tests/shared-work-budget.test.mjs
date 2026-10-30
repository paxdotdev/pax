import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const source = readFileSync(new URL('../src/utils/shared-work-budget.ts', import.meta.url), 'utf8');
const module = {exports:{}};
vm.runInNewContext(transformSync(source, {loader:'ts',format:'cjs'}).code, {module,exports:module.exports});
const {SharedWorkBudget} = module.exports;
test('applications share an allowance and an overrun delays subsequent work until the next frame', () => {
    let time=0; const frames=[];
    const budget=new SharedWorkBudget(4,()=>time,fn=>frames.push(fn));
    budget.run(3, available=>{assert.equal(available,3);time+=2;});
    budget.run(4, available=>{assert.equal(available,2);time+=3;});
    assert.equal(budget.remaining,0);
    assert.equal(frames.length,1);
    frames.shift()();assert.equal(budget.remaining,4);
});
test('a failed work item still consumes its elapsed time', () => {
    let time=0;
    const budget=new SharedWorkBudget(4,()=>time,()=>{});
    assert.throws(()=>budget.run(4,()=>{time+=3;throw Error('work failed');}));
    assert.equal(budget.remaining,1);
});
