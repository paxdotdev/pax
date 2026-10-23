import test from 'node:test';
import assert from 'node:assert/strict';
import {surfaceRequire} from './surface-test-loader.mjs';
import {sharedSurfaceDecision, sharedSurfaceDecisions} from './shared-surface-policy.mjs';
const {materializeSurfaceSchedule} = surfaceRequire('./surface-scheduler');
const candidate = (id, branch, coverage, distance = 0, retained = false) => ({
    layerId: branch, branch, surface: {id, backingWidth: 100, backingHeight: 200},
    rank: {coverage, distance, retained},
});

// Cross-boundary tests exercise real Rust decisions and the presentation
// adapter. Shared Rust tests own ordering, budgets, activity and geometry.
const scheduleSurfaces = (candidates, capacity) => {
    const snapshot = {version: 1, now: 0, capacity, hosts: [],
        candidates: candidates.map(c => ({id: c.surface.id, owner: null,
            rect: [0,0,100,200], retained: c.rank.retained, ready: !!c.rank.ready, pixels: 20000}))};
    return materializeSurfaceSchedule(candidates, sharedSurfaceDecision(snapshot));
};

test('real Rust bridge output materializes unchanged descriptors and signed replay ranks', () => {
    const input = [candidate('b', 2, 0), candidate('a', 1, 0)];
    const result = scheduleSurfaces(input, 4);
    assert.equal(result.admitted.length, 2);
    assert.equal(result.admitted.map(c => c.surface.id).join(','), 'a,b');
    assert.equal(result.plans.get(1).surfaces[0].replayPriority, -2);
    assert.equal(result.plans.get(2).surfaces[0].replayPriority, -1);
    assert.equal(result.overloaded, false);
    assert.equal(result.backingPixels, 40000);
});

test('invalid prepared branch is denied without freezing healthy admission and recovers', () => {
    const host = (id, width) => ({id, parent: null, bounds: [0,0,100,100],
        visible:[0,0,100,100],viewport:[width,100],content:[100,1000],
        offset:[0,0],observed:[0,0],basis:[1,0,0,1],active:true,geometry:'same'});
    const prepare = (now, width) => ({version:1, now, viewport:[0,0,100,100],
        hosts:[host(1,100),host(2,width)]});
    const admit = now => ({version:2,now,capacity:64,candidates:[1,2].map(owner => ({
        id:`tile-${owner}`,owner,rect:[0,0,100,100],retained:true,ready:true,pixels:10000}))});
    const [bad, first, good, recovered] = sharedSurfaceDecisions([
        prepare(0,-28),admit(0),prepare(16,100),admit(16)]);
    assert.equal(bad[1].unavailableReason,'negative surface extent');
    assert.deepEqual(first.admitted,[0]);
    assert.equal(first.denied[0][1],'outside-preparation-trajectory');
    assert.equal(good[1].unavailableReason,undefined);
    assert.deepEqual(recovered.admitted,[0,1]);
});

test('real bridge reports overload without hiding denial behind allocation counters', () => {
    const result = scheduleSurfaces([candidate('a', 1, 0), candidate('b', 2, 0)], 1);
    assert.equal(result.overloaded, true);
    assert.equal(result.denied[0].reason, 'visible-demand-exceeds-budget');
});

test('batched observations preserve activity across snapshots and expire it', () => {
    const snapshot = (now, y) => ({version:1, now, capacity:3, hosts:[{
        id:1,parent:null,bounds:[0,0,100,100],visible:[0,0,100,100],viewport:[100,100],
        offset:[0,y],observed:[0,y],basis:[1,0,0,1],active:true,geometry:'same',
    }],candidates:[{id:'ahead',owner:1,rect:[0,300,100,400],retained:false,ready:false,pixels:10000}]});
    const results=sharedSurfaceDecisions([snapshot(0,0),snapshot(20,50),snapshot(300,50)]);
    assert.equal(results[0].ranks[0].coverage,4);
    assert.equal(results[1].ranks[0].coverage,1);
    assert.equal(results[2].ranks[0].coverage,4);
});

test('recursive preparation and admission use the same immutable observation transaction', () => {
    const hosts=(y)=>[
        {id:1,parent:null,bounds:[0,0,400,300],visible:[0,0,400,300],viewport:[400,300],content:[400,5000],
            offset:[0,y],observed:[0,y],basis:[1,0,0,1],active:true,geometry:'root'},
        {id:2,parent:1,bounds:[0,400-y,400,500-y],visible:[0,0,400,300],viewport:[400,100],content:[5000,100],
            offset:[0,0],observed:[0,0],basis:[1,0,0,1],active:false,geometry:'shelf'},
    ];
    const prepare=(now,y)=>({version:1,now,viewport:[0,0,400,300],hosts:hosts(y)});
    const [idle,moving,decision]=sharedSurfaceDecisions([prepare(0,0),prepare(50,100),{
        version:2,now:50,capacity:64,hosts:[],resources:{viewportPixels:120000,unreleasedPixels:0},
        candidates:[
            {id:'upcoming',owner:2,rect:[0,0,400,100],retained:false,ready:false,pixels:40000},
            {id:'idle-horizontal',owner:2,rect:[600,0,1000,100],retained:false,ready:false,pixels:40000},
        ]}]);
    assert.deepEqual(idle[1].prepared,[0,0,400,100]);
    assert.deepEqual(moving[1].prepared,[0,0,400,100]);
    assert.deepEqual(decision.admitted,[0]);
    assert.equal(decision.ranks[0].coverage,1);
    assert.equal(decision.denied[0][1],'outside-preparation-trajectory');
    assert.equal(decision.optionalPixels,40000);
    assert.equal(decision.optionalLimit,240000);
});

test('unreleased physical cost prevents admission without consuming the same allocation twice',()=>{
    const s={version:1,now:0,capacity:64,hosts:[],
        resources:{viewportPixels:1000000,unreleasedPixels:64*1024*1024-10000},
        candidates:[{id:'visible',owner:null,rect:[0,0,100,100],retained:true,ready:true,pixels:10000},
            {id:'extra',owner:null,rect:[0,0,100,100],retained:false,ready:false,pixels:1}]};
    const d=sharedSurfaceDecision(s);
    assert.deepEqual(d.admitted,[0]);
    assert.equal(d.backingPixels,10000);
    assert.equal(d.denied[0][1],'backing-pixel-budget');
    assert.equal(d.overloaded,true);
});

test('safety survives idle boundaries; retention reuses ready content but yields under pressure',()=>{
    const hosts=[{id:1,parent:null,bounds:[0,0,100,100],visible:[0,0,100,100],
        viewport:[100,100],content:[100,2000],offset:[0,500],observed:[0,500],
        basis:[1,0,0,1],active:true,geometry:'root'}];
    const prepare={version:1,now:5000,viewport:[0,0,100,100],hosts};
    const candidates=[
        {id:'visible',owner:1,rect:[0,500,100,600],retained:true,ready:true,pixels:10000},
        {id:'safety',owner:1,rect:[0,600,100,650],retained:false,ready:false,pixels:5000},
        {id:'retained',owner:1,rect:[0,660,100,690],retained:true,ready:true,pixels:3000},
        {id:'cold',owner:1,rect:[0,410,100,440],retained:false,ready:false,pixels:3000},
        {id:'unsubmitted',owner:1,rect:[0,410,100,440],retained:true,ready:false,pixels:3000},
        {id:'too-far',owner:1,rect:[0,710,100,740],retained:true,ready:true,pixels:3000},
    ];
    const schedule=capacity=>({version:2,now:5000,capacity,hosts:[],candidates});
    const [,room,pressure]=sharedSurfaceDecisions([prepare,schedule(64),schedule(2)]);
    assert.deepEqual(room.admitted,[0,1,2]);
    assert.deepEqual(room.ranks.map(r=>r.coverage),[0,4,6,7,7,7]);
    assert.deepEqual(pressure.admitted,[0,1]);
    assert.ok(pressure.denied.some(([i,why])=>i===2&&why==='preparation-budget'));
});

test('readiness feedback extends forward horizon without duplicate-sample inflation',()=>{
    const snapshot=(now,readiness)=>({version:1,now,viewport:[0,0,100,100],hosts:[],readiness});
    // An exposed host makes the calibrated horizon observable in the DTO.
    const h={id:1,parent:null,bounds:[0,0,100,100],visible:[0,0,100,100],viewport:[100,100],
        content:[100,2000],offset:[0,0],observed:[0,0],basis:[1,0,0,1],active:true,geometry:'root'};
    const inputs=[snapshot(0,[]),snapshot(10,[['revision1',220]]),snapshot(20,[['revision1',900]])];
    inputs.forEach(i=>i.hosts=[h]);
    assert.deepEqual(sharedSurfaceDecisions(inputs).map(p=>p[0].horizonMs),[250,490,490]);
});

test('forward reserve borrows spare capacity without granting nested horizontal overscan',()=>{
    const hosts=y=>[
        {id:1,parent:null,bounds:[0,0,100,100],visible:[0,0,100,100],viewport:[100,100],
            content:[100,4000],offset:[0,y],observed:[0,y],basis:[1,0,0,1],active:true,geometry:'root'},
        {id:2,parent:1,bounds:[0,500-y,100,600-y],visible:[0,0,0,0],viewport:[100,100],
            content:[4000,100],offset:[0,0],observed:[0,0],basis:[1,0,0,1],active:false,geometry:'shelf'}];
    const prepare=(now,y)=>({version:1,now,viewport:[0,0,100,100],hosts:hosts(y)});
    const candidates=[
        {id:'visible',owner:1,rect:[0,10,100,110],retained:true,ready:true,pixels:10000},
        {id:'forward',owner:2,rect:[0,0,100,100],retained:false,ready:false,pixels:30000},
        {id:'idle-child-axis',owner:2,rect:[200,0,300,100],retained:false,ready:false,pixels:10000},
    ];
    const schedule=(now,capacity=64)=>({version:2,now,capacity,hosts:[],candidates,
        resources:{viewportPixels:10000,unreleasedPixels:0}});
    const [, , moving, , paused, pressure, , expired]=sharedSurfaceDecisions([
        prepare(0,0),prepare(50,10),schedule(50),prepare(500,10),schedule(500),schedule(500,4),
        prepare(1600,10),schedule(1600)]);
    assert.deepEqual(moving.admitted,[0,1]);
    assert.equal(moving.ranks[1].coverage,5);
    assert.equal(moving.borrowedForwardPixels,30000);
    assert.equal(moving.optionalLimit,20000);
    assert.deepEqual(paused.admitted,[0,1]);
    assert.deepEqual(pressure.admitted,[0], 'borrowing leaves physical slot headroom');
    assert.deepEqual(expired.admitted,[0,1], 'idle filling remains eligible at lower priority');
    assert.ok(expired.ranks[1].distance > paused.ranks[1].distance);
    assert.equal(moving.ranks[2].coverage,7);
});

test('reverse buffer is funded before forward demand and materializes the new execution rank',()=>{
    const prepare=(now,y)=>({version:1,now,viewport:[0,0,100,100],hosts:[{
        id:1,parent:null,bounds:[0,0,100,100],visible:[0,0,100,100],viewport:[100,100],
        content:[100,4000],offset:[0,y],observed:[0,y],basis:[1,0,0,1],active:true,geometry:'root'}]});
    const candidates=[
        {id:'visible',owner:1,rect:[0,500,100,550],retained:true,ready:true,pixels:10000},
        {id:'ahead',owner:1,rect:[0,620,100,640],retained:false,ready:false,pixels:15000},
        {id:'reverse',owner:1,rect:[0,410,100,430],retained:false,ready:false,pixels:15000},
        {id:'old-far-forward',owner:1,rect:[0,900,100,950],retained:true,ready:true,pixels:10000},
    ];
    const schedule=(now,capacity=64)=>({version:2,now,capacity,candidates,
        resources:{viewportPixels:10000,unreleasedPixels:0}});
    const [, ,moving,pressure, ,reversed]=sharedSurfaceDecisions([
        prepare(0,400),prepare(50,500),schedule(50),schedule(50,2),prepare(100,480),schedule(100)]);
    assert.equal(moving.ranks[2].coverage,2, 'cold reverse tile gets a protected preparation class');
    assert.deepEqual(pressure.admitted,[0,2], 'same small budget chooses the reverse buffer over speculation');
    assert.ok(moving.admitted.indexOf(1)<moving.admitted.indexOf(2), 'incoming executes before replenishment');
    assert.equal(reversed.ranks[2].coverage,1);
    assert.equal(reversed.ranks[1].coverage,2);
    assert.equal(reversed.ranks[3].coverage,7);
    const descriptors=candidates.map(c=>candidate(c.id,1,0));
    const result=materializeSurfaceSchedule(descriptors,reversed);
    assert.deepEqual(result.admitted.map(c=>c.surface.id),['visible','reverse','ahead']);
    assert.ok(result.plans.get(1).surfaces[0].replayPriority<=0);
    assert.ok(result.plans.get(1).surfaces[1].replayPriority<result.plans.get(1).surfaces[2].replayPriority);
});

test('nested ready reverse surfaces borrow beyond the optional allowance before far forward work',()=>{
    const prepare=(now,y)=>({version:1,now,viewport:[0,0,100,100],hosts:[
        {id:1,parent:null,bounds:[0,0,100,100],visible:[0,0,100,100],viewport:[100,100],
            content:[100,4000],offset:[0,y],observed:[0,y],basis:[1,0,0,1],active:true,geometry:'root'},
        {id:2,parent:1,bounds:[0,400-y,100,450-y],visible:[0,0,0,0],viewport:[100,50],
            content:[2000,50],offset:[0,0],observed:[0,0],basis:[1,0,0,1],active:false,geometry:'shelf'},
    ]});
    const candidates=[
        {id:'visible',owner:1,rect:[0,500,100,550],retained:true,ready:true,pixels:10000},
        {id:'reverse-shelf',owner:2,rect:[0,0,100,50],retained:true,ready:true,pixels:30000},
        {id:'far-forward',owner:1,rect:[0,900,100,950],retained:false,ready:false,pixels:30000},
        {id:'idle-child-axis',owner:2,rect:[300,0,400,50],retained:true,ready:true,pixels:10000},
    ];
    const schedule=(capacity=64,unreleasedPixels=0)=>({version:2,now:50,capacity,candidates,
        resources:{viewportPixels:10000,unreleasedPixels}});
    const [, , room, slots, pixels]=sharedSurfaceDecisions([
        prepare(0,400),prepare(50,500),schedule(),schedule(5),schedule(64,64*1024*1024-55000)]);
    assert.equal(room.ranks[1].coverage,2, 'reverse exposure inherits the parent axis');
    assert.equal(room.ranks[3].coverage,7, 'no added horizontal child warmth');
    assert.deepEqual(room.admitted,[0,1,2]);
    assert.equal(room.optionalLimit,20000);
    assert.equal(room.borrowedReversePixels,30000);
    assert.equal(room.borrowedForwardPixels,30000);
    assert.deepEqual(room.borrowingLimit,[60,64*1024*1024-10000]);
    assert.deepEqual(slots.admitted,[0], 'four slots of headroom remain intact');
    assert.deepEqual(pixels.admitted,[0,1], 'reverse coverage gets limited borrowing pixels first');
    assert.equal(pixels.borrowedForwardPixels,0);
    assert.equal(pixels.backingPixels,40000);
});
