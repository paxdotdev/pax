import test from 'node:test';
import assert from 'node:assert/strict';
import {surfaceRequire} from './surface-test-loader.mjs';
const {SurfaceTrace} = surfaceRequire('./surface-trace');

function harness(capacity = 8) {
    const listeners = new Map();
    const mount = {addEventListener: (name, listener) => listeners.set(name, listener),
        removeEventListener: name => listeners.delete(name)};
    return {mount, listeners, trace: new SurfaceTrace(mount, capacity)};
}
const candidate = (id, visible, transformSignature = '0,0') => ({
    surface: {id, hostSignature: 'host:1', surfaceSignature: '100x100', transformSignature},
    rank: {coverage: visible ? 0 : 3, ready: true},
});

test('completion deferrals survive trace rollover and measure only the next opportunity', () => {
    const h = harness(2);
    h.trace.recordCompletionRequest('queued');
    h.trace.recordCompletionRequest('coalesced');
    h.trace.recordCompletionRequest('used');
    const first = h.trace.completionDeferredAt;
    h.trace.recordCompletionRequest('used');
    assert.equal(h.trace.completionDeferredAt, first);
    for (let i = 0; i < 10; i++) h.trace.record('unrelated');
    assert.equal(h.trace.completionRequests.used, 2);
    h.trace.recordPresentationOpportunity(true);
    const event = h.mount.paxSurfaceTrace.at(-1);
    assert.equal(event.event, 'presentation-completion-wait');
    assert.equal(event.blocked, true);
    assert.ok(event.waitMs >= 0);
    h.trace.recordPresentationOpportunity(false);
    assert.equal(h.trace.completionWait.opportunities, 1);
    h.trace.dispose();
    assert.equal(h.trace.completionRequests.used, 0);
    assert.equal(h.trace.completionWait.opportunities, 0);
});

test('wheel input retains a bounded independent tail and opens phase timing before scroll moves', () => {
    const h = harness(3);
    for (let i = 0; i < 140; i++) h.trace.recordWheelInput({enteredAt: i});
    assert.equal(h.trace.wheelInputs.length, 128);
    assert.equal(h.trace.wheelInputs[0].enteredAt, 12);
    h.trace.recordPhase('tick', 2);
    assert.equal(h.mount.paxSurfaceTrace.at(-1).event, 'scroll-phase');
    h.trace.dispose();
    assert.equal(h.trace.wheelInputs.length, 0);
});

test('message batches retain type timings only during probes, scrolls, or slow work', () => {
    const h = harness();
    const types = [{type:'NativeMaskUpdate', count:12, totalMs:5, maxMs:1}];
    h.trace.recordNativeMessageBatch(types);
    assert.equal(h.mount.paxSurfaceTrace.length, 0);
    h.trace.observeScroll(7, 0, 20);
    h.trace.recordNativeMessageBatch(types);
    assert.equal(h.mount.paxSurfaceTrace.at(-1).event, 'native-message-batch');
    assert.equal(h.mount.paxSurfaceTrace.at(-1).duration, 5);
    h.trace.scrollUntil = -Infinity;
    h.trace.recordNativeMessageBatch([{type:'TextUpdate', count:100, totalMs:21, maxMs:2}]);
    assert.equal(h.mount.paxSurfaceTrace.at(-1).duration, 21);
    h.trace.dispose();
});

test('observed scroll opens bounded phase timing without synthesizing input', () => {
    const h = harness(32);
    h.trace.recordPhase('tick', 1);
    assert.equal(h.mount.paxSurfaceTrace.length, 0);
    h.trace.observeScroll(7, 0, 360);
    h.trace.recordPhase('tick', 2);
    assert.deepEqual(Array.from(h.mount.paxSurfaceTrace, e => e.event), ['scroll-sample', 'scroll-phase']);
    assert.equal(h.mount.paxSurfaceTrace[0].y, 360);
    h.trace.scrollUntil = -Infinity;
    h.trace.recordPhase('tick', 2);
    assert.equal(h.mount.paxSurfaceTrace.length, 2);
    h.trace.dispose();
});

test('reversals are per-axis observations and surface events carry current demand rank', () => {
    const h = harness(32);
    h.trace.observeScroll(7, 0, 200, [0,0]);
    h.trace.observeScroll(7, 0, 200.05); // subpixel noise is not intent
    h.trace.observeScroll(8, 0, -10); // independent branch
    h.trace.observeScroll(7, 0, 190);
    const reversals = h.mount.paxSurfaceTrace.filter(e => e.event === 'scroll-reversal');
    assert.equal(reversals.length, 1);
    assert.equal(reversals[0].scroller, 7);
    assert.deepEqual(Array.from(reversals[0].reversedAxes), [1]);
    assert.deepEqual(Array.from(reversals[0].direction), [0,-1]);
    const tile = candidate('tile', false); tile.branch=7; tile.rank.coverage=2;
    h.trace.observeDemand([tile]);
    const canvas={id:'tile', dataset:{surfaceEvent:'initialization-start',replayPriority:'5'}};
    h.listeners.get('pax-surface-trace')({target:canvas});
    assert.equal(h.mount.paxSurfaceTrace.at(-1).coverage, 2);
    assert.equal(h.mount.paxSurfaceTrace.at(-1).owner, 7);
    tile.rank.coverage=0;
    h.trace.observeDemand([tile]);
    canvas.dataset.replayPriority='-1';
    canvas.dataset.surfaceEvent='first-submit';
    h.listeners.get('pax-surface-trace')({target:canvas});
    assert.equal(h.mount.paxSurfaceTrace.at(-1).coverage, 0);
    assert.equal(h.mount.paxSurfaceTrace.at(-1).priority, -1);
    h.trace.dispose();
    assert.equal(h.trace.scrollDirections.size, 0);
});

test('trace is bounded and teardown removes the listener, events and demand state', () => {
    const h = harness(3);
    const events = h.mount.paxSurfaceTrace;
    for (let i = 0; i < 10; i++) h.trace.record('sample', {i});
    assert.deepEqual(Array.from(events, e => e.i), [7, 8, 9]);
    h.trace.observeDemand([candidate('tile', true)]);
    h.trace.dispose();
    assert.equal(h.listeners.size, 0);
    assert.equal(h.mount.paxSurfaceTrace, undefined);
    assert.equal(events.length, 0);
    assert.equal(h.trace.firstVisibleAt('tile'), undefined);
});

test('visible demand is recorded before allocation and only once per geometry identity', () => {
    const h = harness();
    h.trace.observeDemand([candidate('denied', true), candidate('warm', false)]);
    const start = h.trace.firstVisibleAt('denied');
    assert.ok(start > 0);
    assert.equal(h.trace.firstVisibleAt('warm'), undefined);
    h.trace.observeDemand([candidate('denied', true), candidate('warm', true)]);
    assert.equal(h.trace.firstVisibleAt('denied'), start);
    assert.equal(h.mount.paxSurfaceTrace.length, 2);
    h.trace.observeDemand([candidate('denied', true, '0,100')]);
    assert.equal(h.mount.paxSurfaceTrace.length, 3, 'retargeted content has a new visibility lifetime');
    assert.equal(h.trace.firstVisibleAt('warm'), undefined, 'forgotten demand does not accumulate');
});

test('late lease release retains the retired assignment identity', () => {
    const h = harness();
    h.listeners.get('pax-surface-trace')({target: {id: '', width: 100, height: 100,
        dataset: {surfaceEvent: 'lease-released', surfaceRetiredId: 'old-tile',
            surfaceRetiredRevision: '12', surfaceGeneration: '3', paxSurfaceOwners: '0'}}});
    const event = h.mount.paxSurfaceTrace[0];
    assert.equal(event.id, 'old-tile');
    assert.equal(event.contentRevision, '12');
    assert.equal(event.generation, '3');
    assert.equal(event.owners, 0);
});

test('transient visible readiness gaps remain inspectable after they settle', () => {
    const h = harness();
    const pending = candidate('tile', true);
    pending.rank.ready = false;
    h.trace.observeDemand([pending]);
    h.trace.observeDemand([pending]);
    h.trace.observeDemand([candidate('tile', true)]);
    const gaps = h.mount.paxSurfaceTrace.filter(e => e.event === 'visible-unready');
    assert.deepEqual(Array.from(gaps, e => Array.from(e.ids)), [['tile'], []]);
});

test('retirement distinguishes explicit denial, candidate omission and submitted generation', () => {
    const h = harness(16);
    const canvas = {id:'tile', width:100, height:100, dataset:{surfaceEvent:'retirement',
        surfaceGeneration:'3', surfaceContentRevision:'12', surfaceSubmittedRevision:'12'}};
    const emit = () => h.listeners.get('pax-surface-trace')({target:canvas});
    h.trace.observeDemand([candidate('tile', false)], [{id:'tile',reason:'optional-pixel-budget'}]);
    emit();
    assert.equal(h.mount.paxSurfaceTrace.at(-1).admission, 'optional-pixel-budget');
    assert.equal(h.mount.paxSurfaceTrace.at(-1).submitted, true);
    assert.equal(h.mount.paxSurfaceTrace.at(-1).generation, '3');
    h.trace.observeDemand([]);
    emit();
    assert.equal(h.mount.paxSurfaceTrace.at(-1).admission, 'not-candidate');
    h.trace.observeDemand([candidate('tile', false)]);
    canvas.dataset.surfaceGeneration = '4';
    canvas.dataset.surfaceContentRevision = '13';
    emit();
    assert.equal(h.mount.paxSurfaceTrace.at(-1).admission, 'admitted');
    assert.equal(h.mount.paxSurfaceTrace.at(-1).submitted, false);
    assert.equal(h.mount.paxSurfaceTrace.at(-1).submittedRevision, '12');
});

test('image readiness distinguishes decoding, drawing, and submission and clears retired owners', () => {
    const h = harness(32);
    const canvas = {id: 'tile', width: 100, height: 100, dataset: {surfaceGeneration: '1',
        imageNode: '12', imagePath: 'film', imageVersion: '1', imageBytes: '4'}};
    const emit = name => {canvas.dataset.surfaceEvent = name; h.listeners.get('pax-surface-trace')({target: canvas});};
    emit('image-waiting-decoded'); emit('image-waiting-decoded');
    assert.equal(h.mount.paxSurfaceTrace.length, 1, 'unchanged waiting state is not logged each frame');
    assert.equal(h.trace.pendingImages.size, 1);
    emit('image-upload'); emit('image-draw-ready');
    assert.equal(h.trace.pendingImages.size, 0);
    assert.equal(h.trace.recordedImages.get('tile').size, 1);
    delete canvas.dataset.imageNode;
    emit('first-submit');
    assert.equal(h.trace.recordedImages.size, 1, 'surface revision is not image readiness');
    emit('image-content-submit');
    assert.equal(h.trace.recordedImages.size, 0);
    emit('lease-released');
    assert.equal(h.trace.imageStates.size, 0);
    h.trace.dispose();
    assert.equal(h.trace.pendingImages.size, 0);
});

test('unsubmitted image diagnostics stay bounded and retirement clears them before lease release', () => {
    const h = harness(2);
    const canvas = {id: 'tile', width: 100, height: 100, dataset: {surfaceGeneration: '1',
        imagePath: 'film', imageVersion: '1', imageBytes: '4', surfaceEvent: 'image-draw-ready'}};
    for (const id of ['1', '2', '3']) {
        canvas.dataset.imageNode = id;
        h.listeners.get('pax-surface-trace')({target: canvas});
    }
    assert.equal(h.trace.imageStates.size, 2);
    assert.equal(h.trace.recordedImages.get('tile').size, 2);
    delete canvas.dataset.imageNode;
    canvas.dataset.surfaceEvent = 'retirement';
    h.listeners.get('pax-surface-trace')({target: canvas});
    assert.equal(h.trace.imageStates.size, 0);
    assert.equal(h.trace.recordedImages.size, 0);
});
