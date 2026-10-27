import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const source = readFileSync(new URL('../src/utils/cooperative-render.ts', import.meta.url), 'utf8');
function load(globals={}) {
    const module={exports:{}};
    vm.runInNewContext(transformSync(source,{loader:'ts',format:'cjs'}).code,{module,exports:module.exports,...globals});
    return module.exports;
}
const {CooperativeRenderScheduler}=load();

test('optional host-wait timing measures queued serviced tasks, not cancelled tasks',()=>{
    let now=10; const tasks=[], samples=[];
    const {CooperativeRenderScheduler: Timed}=load({performance:{now:()=>now}});
    const pump=new Timed(()=>({pending:true}),()=>true,job=>tasks.push(job),ms=>samples.push(ms));
    pump.beginFrame(); now=25; tasks.shift()();
    assert.deepEqual(samples,[15]);
    pump.dispose(); now=50; tasks.shift()();
    assert.deepEqual(samples,[15]);
});

test('ordinary drawing and completion storms share one non-reentrant task pump',()=>{
    const tasks=[]; let calls=0; const origins=[];
    const pump=new CooperativeRenderScheduler(frame=>{
        calls++; origins.push(frame);
        if(calls===2) { pump.request(); pump.request(); }
        return {pending:calls<3};
    },()=>true,job=>tasks.push(job));
    assert.equal(pump.request(),'used','no continuation before a settled frame');
    pump.beginFrame();
    assert.equal(calls,1); assert.equal(tasks.length,1);
    for(let i=0;i<20;i++) assert.equal(pump.request(),'coalesced');
    tasks.shift()(); assert.equal(tasks.length,1);
    tasks.shift()(); assert.equal(tasks.length,0);
    assert.deepEqual(origins,[true,false,false]);
});

test('new frame supersedes queued intent without duplicate tasks or old selection',()=>{
    const tasks=[]; let frame=1; const selected=[];
    const pump=new CooperativeRenderScheduler(()=>{selected.push(frame); return {pending:true};},()=>true,job=>tasks.push(job));
    pump.beginFrame(); frame=2; pump.beginFrame();
    assert.equal(tasks.length,1);
    tasks.shift()(); assert.deepEqual(selected,[1,2,2]);
    pump.invalidate(); tasks.shift()();
    assert.deepEqual(selected,[1,2,2], 'resize/cancellation cannot resume old work');
    assert.equal(tasks.length,0);
});

test('blocked and exhausted work stop; a completion does not create a new frame',()=>{
    for(const outcome of [{blocked:true},{allowance_exhausted:true},{pending:false}]) {
        const tasks=[]; const origins=[];
        const pump=new CooperativeRenderScheduler(frame=>{origins.push(frame);return {pending:true,...outcome};},()=>true,job=>tasks.push(job));
        pump.beginFrame(); assert.equal(tasks.length,0);
        pump.request(); tasks.shift()(); assert.equal(tasks.length,0);
        assert.deepEqual(origins,[true,false]);
    }
});

test('completion during a blocked service survives its result and is consumed once',()=>{
    const tasks=[]; let calls=0;
    const pump=new CooperativeRenderScheduler(()=>{
        calls++;
        if(calls===1) return {pending:false,blocked:true};
        if(calls===2) {
            // A synchronous initializer finishes while an older unit reports
            // blockage. Its wake must not be overwritten by that result.
            assert.equal(pump.request(),'coalesced');
            return {pending:false,blocked:true};
        }
        return {pending:false};
    },()=>true,job=>tasks.push(job));
    pump.beginFrame();
    assert.equal(tasks.length,0);
    assert.equal(pump.request(),'queued'); // Async completion after the first service.
    tasks.shift()();
    assert.equal(tasks.length,1);
    tasks.shift()();
    assert.equal(calls,3);
    assert.equal(tasks.length,0);
});

test('a settled frame resumes blocked completion work without another completion',()=>{
    const tasks=[]; let settled=false, submissions=0;
    const pump=new CooperativeRenderScheduler(()=>{
        if(!settled) return {pending:false,blocked:true};
        submissions++;
        return {pending:false};
    },()=>true,job=>tasks.push(job));
    pump.beginFrame();
    pump.request();
    tasks.shift()();
    assert.equal(submissions,0);
    assert.equal(tasks.length,0,'blocked work must not spin');
    settled=true;
    pump.beginFrame();
    assert.equal(submissions,1);
    assert.equal(tasks.length,0);
});

test('suspension, replacement and disposal reject queued continuations',()=>{
    const tasks=[]; let allowed=true,calls=0;
    const pump=new CooperativeRenderScheduler(()=>{calls++;return {pending:true};},()=>allowed,job=>tasks.push(job));
    pump.beginFrame(); allowed=false; tasks.shift()(); assert.equal(calls,1);
    pump.invalidate(); allowed=true; pump.beginFrame(); assert.equal(calls,2);
    pump.dispose(); tasks.shift()(); assert.equal(calls,2); assert.equal(pump.request(),'disposed');
});

test('host scheduling uses scheduler.yield or a posted MessageChannel task, not a microtask',async()=>{
    let yielded=0,called=0,resolve;
    const fast=load({scheduler:{yield(){yielded++;return new Promise(r=>resolve=r);}}});
    fast.enqueueRenderTask(()=>called++); assert.equal(yielded,1); assert.equal(called,0);
    resolve(); await Promise.resolve(); assert.equal(called,1);
    let posted=0,channel;
    class Channel { constructor(){channel=this;this.port1={};this.port2={postMessage:()=>posted++};} }
    const fallback=load({MessageChannel:Channel,queueMicrotask:()=>{throw Error('not a host boundary');}});
    fallback.enqueueRenderTask(()=>called++); assert.equal(posted,1); assert.equal(called,1);
    channel.port1.onmessage(); assert.equal(called,2);
});
