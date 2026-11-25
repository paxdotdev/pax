import test from 'node:test';
import assert from 'node:assert/strict';
import {surfaceRequire} from './surface-test-loader.mjs';
const {SurfaceCoordinator} = surfaceRequire('./surface-coordinator');
test('publication requires configured and submitted visible surfaces, but not warm tiles', () => {
    const canvases=new Map();
    const layers={syncLayerCanvasLayouts(provider){provider(0);}};
    const owner=new SurfaceCoordinator(canvases,layers,()=>{});
    owner.sync(()=>({active:true,surfaces:[{id:'visible',replayPriority:0},{id:'warm',replayPriority:2}]}));
    assert.equal(owner.presentationReady(),false);
    const canvas={dataset:{surfaceGeneration:'1',surfaceContentRevision:'7'}};
    canvases.set('visible',canvas);
    assert.equal(owner.presentationReady(),false);
    canvas.dataset.surfaceConfiguredGeneration='1';
    assert.equal(owner.presentationReady(),false);
    canvas.dataset.surfaceSubmittedRevision='7';
    assert.equal(owner.presentationReady(),true);
    canvas.dataset.surfaceContentRevision='8';
    assert.equal(owner.presentationReady(),false,'a changed scene needs a fresh submission');
    canvas.dataset.surfaceSubmittedRevision='8';
    assert.equal(owner.presentationReady(),true);
    canvas.dataset.surfaceGeneration='2';
    assert.equal(owner.presentationReady(),false,'a resized surface needs configuration');
});

test('reversing releases a configured lease and rearms the diagnostic for a new revision', t => {
    const warning = t.mock.method(console, 'warn', () => {});
    const canvas = {id: 'tile', dataset: {surfaceGeneration: '1',
        surfaceConfiguredGeneration: '1', surfaceContentRevision: '7'}};
    const layers = {syncLayerCanvasLayouts(provider) { provider(0); }};
    const owner = new SurfaceCoordinator(new Map([['tile', canvas]]), layers, () => {});
    const priority = replayPriority => owner.sync(() => ({active: true, surfaces: [{id: 'tile', replayPriority}]}));
    priority(-1);
    owner.presentationReady('no visible work reported', 0);
    owner.presentationReady('no visible work reported', 1010);
    owner.presentationReady('no visible work reported', 2020);
    assert.equal(warning.mock.callCount(), 1);
    assert.equal(warning.mock.calls[0].arguments[1].revision, '7');
    assert.equal(owner.presentationReady(), false);

    priority(2);
    assert.equal(owner.presentationReady(), true);
    priority(-1);
    canvas.dataset.surfaceContentRevision = '8';
    canvas.dataset.surfaceSubmittedRevision = '7';
    assert.equal(owner.presentationReady('no visible work reported', 3000), false);
    assert.equal(warning.mock.callCount(), 1);
    canvas.dataset.surfaceSubmittedRevision = '8';
    owner.presentationReady('no visible work reported', 4020);
    assert.equal(owner.presentationReady(), true);
    assert.equal(warning.mock.callCount(), 1);
});

test('a changing lease cannot hide another tile that still owes its revision', t => {
    const warning = t.mock.method(console, 'warn', () => {});
    const tile = id => ({id, dataset: {surfaceGeneration: '5', surfaceConfiguredGeneration: '5',
        surfaceContentRevision: '7'}});
    const first = tile('changing'), second = tile('stalled');
    const owner = new SurfaceCoordinator(new Map([[first.id, first], [second.id, second]]),
        {syncLayerCanvasLayouts(provider) { provider(0); }}, () => {});
    owner.sync(() => ({active: true, surfaces: [first, second].map(({id}) => ({id, replayPriority: 0}))}));
    owner.presentationReady('replay target', 0);
    first.dataset.surfaceContentRevision = '8';
    assert.equal(owner.presentationReady('replay target', 1010), false);
    assert.equal(warning.mock.callCount(), 1);
    assert.equal(warning.mock.calls[0].arguments[1].id, 'stalled');
});
