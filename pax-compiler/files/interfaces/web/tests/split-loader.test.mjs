import assert from 'node:assert/strict';
import {createSplitLoader} from '../src/utils/split-loader.js';
const minimal=new Uint8Array([0,97,115,109,1,0,0,0]);
const response=()=>new Response(minimal,{headers:{'Content-Type':'application/wasm'}});
let cases=0;
function make(fetcher){return createSplitLoader({url:'fixture',getImports:()=>({}),fetcher});}
// Concurrent observers share one request and one installation.
let requests=0;const one=make(async()=>{requests++;return response();});
const [a,b]=await Promise.all([one.load(),one.load()]);assert.equal(a,b);assert.equal(requests,1);assert.equal(await one.load(),a);cases++;
// An immediate cancellation and a compiled-but-not-installed cancellation are retryable.
for(const phase of ['downloading','compiled','installing']){
 let cancelled=false;const scope=make(async()=>response());
 scope.observe(e=>{if(e.phase===phase&&!cancelled){cancelled=true;scope.cancel();}});
 await assert.rejects(scope.load(),{name:'AbortError'});
 scope.observe(()=>{});await scope.load();assert.equal(scope.state.phase,'ready');cases++;
}
// Cancellation never leaks across scopes, and a failed request is not memoized.
const left=make(async(_,{signal})=>new Promise((resolve,reject)=>{signal.addEventListener('abort',()=>reject(new DOMException('cancel','AbortError')),{once:true});}));
const right=make(async()=>response());const l=left.load(),r=right.load();left.cancel();await assert.rejects(l);await r;assert.equal(right.state.phase,'ready');cases++;
let fail=true;const retry=make(async()=>fail?new Response('',{status:503}):response());await assert.rejects(retry.load(),/HTTP 503/);fail=false;await retry.load();cases++;
// Compression does not turn encoded Content-Length into a decoded denominator.
const compressed=make(async()=>new Response(minimal,{headers:{'Content-Length':'2','Content-Encoding':'gzip'}}));await compressed.load();assert.equal(compressed.state.total,null);assert.equal(compressed.state.received,8);cases++;
// Progress callbacks cannot poison a successful installation; post-install cancel is harmless.
const done=make(async()=>response());await done.load();done.cancel();assert.equal(done.state.phase,'ready');cases++;
// Exact callback settlement for every Rust invocation sharing one fetch.
let callbacks=[];const table={get:index=>(data,success)=>callbacks.push([index,data,success])};
const cb=make(async()=>response()).callback(()=>table);cb(2,4);cb(3,5);await new Promise(r=>setTimeout(r,20));assert.deepEqual(callbacks,[[2,4,true],[3,5,true]]);cases++;
let reentrant, nested, fetches=0;
reentrant=make(async()=>{fetches++;return response();});
reentrant.observe(e=>{if(e.phase==='downloading')nested=reentrant.load();});
const outer=reentrant.load();await outer;assert.equal(nested,outer);assert.equal(fetches,1);cases++;
// A trapping start function may already have written shared memory. Never
// silently retry its installation into the same runtime.
const trap=Uint8Array.from([0,97,115,109,1,0,0,0,1,4,1,96,0,0,3,2,1,0,8,1,0,10,5,1,3,0,0,11]);
let installs=0;
const poison=make(async()=>{installs++;return new Response(trap);});
await assert.rejects(poison.load(),/recreate the runtime/);
await assert.rejects(poison.load(),/recreate the runtime/);
assert.equal(installs,1);assert.equal(poison.state.retryable,false);cases++;
let failCallbacks=[];const failed=make(async()=>new Response('',{status:503})).callback(()=>({get:i=>(d,ok)=>failCallbacks.push([i,d,ok])}));
failed(4,6);failed(5,7);await new Promise(r=>setTimeout(r,20));assert.deepEqual(failCallbacks,[[4,6,false],[5,7,false]]);cases++;
// Some browsers reject aborted streaming work with TypeError.
let notifyFetch;
const fetched=new Promise(resolve=>notifyFetch=resolve);
const safariAbort=make(async(_,{signal})=>new Promise((resolve,reject)=>{
 signal.addEventListener('abort',()=>reject(new TypeError('Fetch is aborted')),{once:true});notifyFetch();
}));
const cancelledStream=safariAbort.load();await fetched;safariAbort.cancel();
await assert.rejects(cancelledStream,{name:'AbortError'});assert.equal(safariAbort.state.phase,'cancelled');cases++;
// Removing one UI observer must leave the other invocation subscribed.
const observed=make(async()=>response());let retired=0,live=0;
const unsubscribe=observed.observe(()=>retired++);observed.observe(()=>live++);unsubscribe();
await observed.load();assert.equal(retired,0);assert.ok(live>0);cases++;
console.log(JSON.stringify({passed:true,cases}));
