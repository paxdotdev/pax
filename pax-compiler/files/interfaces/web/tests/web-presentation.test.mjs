import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/classes/web-presentation.ts', import.meta.url), 'utf8');

function harness(onApply = () => {}) {
    const log = [];
    const listeners = new Map();
    const root = {
        setAttribute() {}, removeAttribute() {},
        addEventListener: (name, fn) => listeners.set(name, fn),
        removeEventListener: name => listeners.delete(name),
    };
    class Canvas {
        id = 'tile';
        isConnected = true;
        dataset = {surfaceGeneration: '1', surfaceContentRevision: '1'};
        style = {removeProperty() {}};
        ownerDocument = {createElement: () => new Canvas()};
        after(front) { this.front = front; }
        remove() { this.isConnected = false; }
        addEventListener(name, handler) { this.retirement = handler; }
        removeEventListener() { this.retirement = undefined; }
        retire() { this.retirement({target:this}); this.retirement = undefined; }
        closest() { return root; }
        getContext() { return {transferFromImageBitmap: image => log.push('draw ' + image.name)}; }
        submit(name) {
            const image = {name, close() { log.push('close ' + name); }};
            listeners.get('pax-tile-submission')({target: this, detail: image, preventDefault() {}});
        }
    }
    const module = {exports: {}};
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code,
        {module, exports: module.exports, HTMLCanvasElement: Canvas});
    const state = {coverage: 'A', complete: 0n, fence: 1n};
    const chassis = {
        set_displayed_scroll_reader() {}, prepare_displayed_input: () => true,
        publish_displayed_input: () => log.push('input'),
        web_submission_fence: () => state.fence,
        web_submission_complete: serial => serial <= state.complete,
    };
    const applied = [];
    const owner = new module.exports.WebPresentation(root, chassis,
        messages => messages.forEach(msg => { applied.push(msg); log.push(Object.keys(msg)[0]); onApply(msg); }),
        () => state.coverage, () => []);
    owner.settled();
    log.length = 0;
    return {canvas: new Canvas(), owner, state, log, applied};
}

test('scroll observations coalesce across a rejected fence and preserve command and ownership barriers', () => {
    const {owner, state, applied} = harness();
    const offset = (axis, value) => ({ScrollerUpdate: {id: 4, subtree_depth: 0, [axis]: value}});
    const geometry = n => ({ScrollerUpdate: {id: 5, subtree_depth: 0, presented_bounds: [0,n,10,n+10],
        presented_clip_bounds: [0,0,10,10], presentation_projection: []}});
    for (let n = 0; n < 2500; n++) owner.receive([geometry(n), offset('presentation_scroll_y', n)]);
    owner.settled();
    owner.receive([offset('presentation_scroll_x', 3), geometry(2500)]);
    state.coverage = 'B';
    state.complete = 1n;
    assert.equal(owner.publish(), false);
    const reparent = {ScrollerUpdate: {id: 4, subtree_depth: 0, parent_frame: null}};
    const claim = {ScrollerUpdate: {...geometry(2501).ScrollerUpdate, content_layer_id: 3}};
    const command = {ScrollerUpdate: {id: 4, subtree_depth: 0, scroll_y: 0}};
    owner.receive([reparent, geometry(2502), claim, geometry(2503), command, command, offset('presentation_scroll_y', 0)]);
    owner.settled();
    assert.equal(owner.publish(), true);
    assert.deepEqual(applied, [
        {ScrollerUpdate: {id: 4, subtree_depth: 0, presentation_scroll_x: 3, presentation_scroll_y: 2499}},
        geometry(2500), reparent, geometry(2502), claim, geometry(2503), command, command, offset('presentation_scroll_y', 0),
    ]);
});

test('a late fence holds its native prefix, image and input and preserves later messages', () => {
    const {canvas, owner, state, log} = harness();
    canvas.submit('old');
    canvas.submit('first');
    owner.receive([{ButtonUpdate: {}}, {ImageLoad: {}}]);
    owner.settled();
    assert.equal(owner.awaitingFence, true);
    owner.receive([{ButtonDelete: {}}]);
    owner.publish();
    assert.deepEqual(log, ['close old', 'ImageLoad']);
    state.complete = 1n;
    owner.publish();
    assert.equal(owner.awaitingFence, false);
    assert.deepEqual(log.slice(-3), ['ButtonUpdate', 'draw first', 'input']);
    state.fence = 2n;
    owner.settled();
    owner.publish();
    assert.equal(log.includes('ButtonDelete'), false);
    state.complete = 2n;
    owner.publish();
    assert.deepEqual(log.slice(-2), ['ButtonDelete', 'input']);
});

test('a retained tile publishes its CSS rectangle with its bitmap after a late fence', () => {
    const {canvas, owner, state} = harness();
    const rect = () => [canvas.front.style.left, canvas.front.style.top, canvas.front.style.width, canvas.front.style.height];
    Object.assign(canvas.dataset, {tileOriginX:'5', tileOriginY:'8', logicalWidth:'100', logicalHeight:'60'});
    canvas.submit('initial');
    owner.settled();
    state.complete = 1n;
    assert.equal(owner.publish(), true);
    const initial = ['5px', '8px', '100px', '60px'];
    assert.deepEqual(rect(), initial);

    Object.assign(canvas.dataset, {tileOriginX:'7', tileOriginY:'9', logicalWidth:'80', logicalHeight:'55',
        surfaceContentRevision:'2'});
    state.fence = 2n;
    canvas.submit('resized');
    owner.settled();
    assert.equal(owner.publish(), false);
    assert.deepEqual(rect(), initial);
    state.complete = 2n;
    assert.equal(owner.publish(), true);
    assert.deepEqual(rect(), ['7px', '9px', '80px', '55px']);
});

test('a changed visible assignment invalidates the old completion and retains native order', () => {
    const {canvas, owner, state, log} = harness();
    owner.receive([{ButtonCreate: {}}]);
    canvas.submit('retired');
    assert.equal(owner.settled(), true);
    owner.receive([{ButtonUpdate: {}}]);
    canvas.dataset.surfaceGeneration = '2';
    state.coverage = 'B';
    state.complete = 1n;
    assert.equal(owner.publish(), false);
    assert.deepEqual(log, ['close retired']);
    canvas.submit('replacement');
    state.fence = 2n;
    owner.settled();
    owner.publish();
    assert.deepEqual(log, ['close retired']);
    state.complete = 2n;
    owner.publish();
    assert.deepEqual(log.slice(1), ['ButtonCreate', 'ButtonUpdate', 'draw replacement', 'input']);
});

test('a newly required tile invalidates coverage even if a prepared lease is still current', () => {
    const {canvas, owner, state, log} = harness();
    canvas.submit('keep');
    owner.settled();
    state.coverage = 'A+B';
    state.complete = 1n;
    owner.publish();
    assert.deepEqual(log, []);
    owner.dispose();
    assert.deepEqual(log, ['close keep']);
});

test('a large responsive replacement survives a late fence and a coverage retry in order', () => {
    const {canvas, owner, state, applied, log} = harness();
    const replacement = [];
    for (let id = 1; id <= 2500; id++) {
        replacement.push({TextCreate: {id}}, {TextUpdate: {id, content: 'x'.repeat(1024)}});
    }
    assert.equal(owner.canSettleFeedback(), true);
    owner.receive(replacement);
    assert.equal(owner.canSettleFeedback(), false);
    canvas.submit('old coverage');
    owner.settled();
    const command = {ScrollerUpdate: {id: 0, scroll_y: 0}};
    owner.receive([command, command]);
    assert.equal(owner.publish(), false);
    assert.deepEqual(applied, []);
    state.complete = state.fence;
    state.coverage = 'resized';
    canvas.dataset.surfaceGeneration = '2';
    assert.equal(owner.publish(), false);
    assert.deepEqual(log, ['close old coverage']);
    assert.equal(owner.canSettleFeedback(), true);
    canvas.submit('new coverage');
    state.fence++;
    owner.settled();
    assert.equal(owner.publish(), false);
    assert.deepEqual(applied, []);
    state.complete = state.fence;
    assert.equal(owner.publish(), true);
    assert.deepEqual(applied, [...replacement, command, command]);
    assert.deepEqual(log.slice(-2), ['draw new coverage', 'input']);
});

test('missing visible coverage holds an armed revision until it can seal', () => {
    const {owner, state} = harness();
    state.coverage = undefined;
    assert.equal(owner.settled(), false);
    assert.equal(owner.awaitingFence, false);
    state.coverage = 'B';
    assert.equal(owner.settled(), true);
    assert.equal(owner.publish(), false);
    state.complete = 1n;
    assert.equal(owner.publish(), true);
});

test('a native delete that retires its canvas during publication releases its queued image', () => {
    const {canvas, owner, state, log} = harness(() => { canvas.isConnected = false; });
    canvas.submit('retired island');
    owner.receive([{ScrollerDelete: 7}]);
    owner.settled();
    state.complete = state.fence;
    assert.equal(owner.publish(), true);
    assert.deepEqual(log, ['ScrollerDelete', 'close retired island', 'input']);
});

test('a recycled submission endpoint keeps its outgoing bitmap visible through a late ownership change', () => {
    const {canvas, owner, state, log} = harness();
    canvas.submit('outgoing');
    owner.settled();
    state.complete = 1n;
    owner.publish();
    const outgoing = canvas.front;
    canvas.retire();
    canvas.dataset.surfaceGeneration = '2';
    canvas.submit('incoming');
    state.fence = 2n;
    owner.settled();
    assert.equal(owner.publish(), false);
    assert.ok(outgoing.isConnected);
    assert.equal(canvas.front, outgoing);
    state.complete = 2n;
    assert.equal(owner.publish(), true);
    assert.equal(outgoing.isConnected, false);
    assert.ok(canvas.front.isConnected);
    assert.deepEqual(log.slice(-2), ['draw incoming', 'input']);
});
