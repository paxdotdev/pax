import test from 'node:test';
import assert from 'node:assert/strict';
import {surfaceRequire} from './surface-test-loader.mjs';
const {SurfaceCapture, observeSurfaceReadiness, rectUnionArea} = surfaceRequire('./surface-capture');
const candidate = (id, layer = 1, branch = 7, left = 0) => ({layerId:layer, branch,
    surface:{id,left,top:0,width:100,height:100,hostSignature:'host',surfaceSignature:'size',transformSignature:'position'},
    rank:{coverage:0,ready:false}});
const dataset = () => ({hostSignature:'host',surfaceSignature:'size',transformSignature:'position',
    surfaceGeneration:'2',surfaceConfiguredGeneration:'2',surfaceContentRevision:'3',surfaceSubmittedRevision:'3',paxSurfaceOwners:'1'});
const clips = () => new Map([[7,[20,10,150,60]]]);

test('union clips overlap without double counting and ignores invalid geometry', () => {
    assert.equal(rectUnionArea([[0,0,100,100],[50,0,150,100],[20,20,30,30]]),15000);
    assert.equal(rectUnionArea([[0,0,0,10],[0,0,NaN,10],[0,0,10,10]]),100);
});

test('coverage stays per layer and clips each tile to its owner content viewport', () => {
    const candidates = [candidate('a'),candidate('b',1,7,50),candidate('c',2)];
    const report = observeSurfaceReadiness(candidates,[],new Map(),clips(),[0,0,1000,1000]);
    assert.equal(report.layers[0].area,6500);
    assert.equal(report.layers[0].unsubmittedArea,6500);
    assert.equal(report.layers[1].area,4000);
    assert.deepEqual(Array.from(report.surfaces[0].rect),[20,10,100,60]);
});

test('stages distinguish assignment, initialization, configuration and submission', () => {
    const tile = candidate('a'), d = dataset(), canvases = new Map([['a',{dataset:d}]]);
    const observe = () => observeSurfaceReadiness([tile],[{id:'a',reason:'capacity'}],canvases,clips(),[0,0,100,100]).surfaces[0];
    assert.equal(observe().stage,'submitted');
    d.surfaceSubmittedRevision = 'old';
    assert.equal(observe().stage,'submission');
    d.surfaceConfiguredGeneration = 'old';
    assert.equal(observe().stage,'configuration');
    d.paxSurfaceOwners = '0';
    assert.equal(observe().stage,'initializer');
    d.transformSignature = 'stale';
    assert.equal(observe().stage,'assignment');
    assert.equal(observe().generation,undefined,'old assignment identity cannot qualify new demand');
    assert.equal(observe().denial,'capacity');
    canvases.clear();
    assert.equal(observe().stage,'assignment');
});

test('current configuration and submission qualify CPU surfaces without GPU leases', () => {
    const d = dataset(); d.paxSurfaceOwners='0';
    const report = observeSurfaceReadiness([candidate('a')],[],new Map([['a',{dataset:d}]]),clips(),[0,0,100,100]);
    assert.equal(report.layers[0].unsubmittedArea,0);
});

test('warm and unavailable owner surfaces cannot inflate visible demand', () => {
    const warm = candidate('warm'); warm.rank.coverage=1;
    const result=observeSurfaceReadiness([warm,candidate('missing',1,8),candidate('root',3,-1)],[],new Map(),clips(),[0,0,25,50]);
    assert.equal(result.surfaces.length,1);
    assert.equal(result.layers[0].area,1250);
});

test('finite capture retains independent bounded causal and observation rings with aggregate gaps', () => {
    const c = new SurfaceCapture(10,{test:true},3);
    for(let i=0;i<10;i++) c.event(10+i,'submit',{generation:String(i)});
    for(const at of [10,20,100,110]) c.frame(at,false);
    const observation = observeSurfaceReadiness([candidate('a')],[],new Map(),clips(),[0,0,100,100]);
    c.observe(20,observation); c.observe(120,observation);
    c.phase('continuation.host_wait',20,1,20);
    c.stop(130); c.stop(200);
    c.event(140,'ignored'); c.frame(140,false); c.observe(140,observation);
    c.phase('continuation.host_wait',100,1,100);
    const report = c.snapshot();
    assert.equal(report.endedAt,130);
    assert.equal(report.events.length,3); assert.equal(report.overwritten.events,8);
    assert.equal(report.events.at(-1).event,'host-task-delivered');
    assert.equal(report.events.at(-1).at-report.events.at(-1).queuedAt,20);
    assert.equal(report.observations.length,3); assert.equal(report.overwritten.observations,3);
    assert.equal(report.summary.maxFrameGapMs,80);
    assert.equal(report.summary.frameGapsOver50Ms,1);
    assert.equal(report.summary.maxObservationGapMs,100);
    assert.equal(report.summary.unsubmittedObservations,2);
    assert.equal(report.summary.phases['continuation.host_wait'].totalMs,20);
    assert.match(report.semantics,/not GPU completion/);
});
