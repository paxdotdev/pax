import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

class Element {
    dataset = {}; style = {}; attributes = new Map(); listeners = new Map();
    width = 300; height = 200; parentElement;
    contextRequests = [];
    gpuContext = {
        configured: true, retiredAtSize: [],
        unconfigure: () => {
            this.gpuContext.configured = false;
            this.gpuContext.retiredAtSize.push([this.width, this.height]);
        },
    };
    getAttribute(name) { return this.attributes.get(name) ?? null; }
    hasAttribute(name) { return this.attributes.has(name); }
    setAttribute(name, value) { this.attributes.set(name, value); }
    getContext(kind) { this.contextRequests.push(kind); return this.gpuContext; }
    appendChild(child) { child.parentElement = this; }
    removeChild(child) { child.parentElement = undefined; }
    addEventListener(name, callback) { this.listeners.set(name, callback); }
    releaseOwner(count) {
        this.setAttribute('data-pax-surface-owners', String(count));
        if (count === 0) this.parentElement?.listeners.get('pax-surface-released')?.({target:this});
    }
}

const module = {exports:{}};
const source = readFileSync(new URL('../src/classes/canvas-pool.ts', import.meta.url), 'utf8');
vm.runInNewContext(transformSync(source, {loader:'ts', format:'cjs'}).code, {
    module, exports:module.exports, require:() => ({}), queueMicrotask,
    document:{createElement:() => new Element()},
});
const {CanvasPool} = module.exports;

function harness(capacity) {
    let wakes = 0;
    const pool = new CanvasPool({getFromPool:() => new Element()}, capacity, () => wakes++);
    pool.attach(new Element());
    return {pool, wakes:() => wakes};
}

test('capacity never permits a canvas still owned by an asynchronous initializer to be reused', () => {
    const {pool} = harness(1);
    const old = pool.checkout();
    old.setAttribute('data-pax-surface-owners', '1');
    pool.release(old);
    assert.equal(pool.checkout(), null);
    assert.equal(old.width, 300, 'release must not clear a live GPU backing store');
    old.releaseOwner(0);
    assert.equal(pool.checkout(), old);
});

test('only the final surface owner clears the canvas and asynchronously wakes a blocked checkout', async () => {
    const h = harness(1);
    const old = h.pool.checkout();
    old.setAttribute('data-pax-surface-owners', '2');
    h.pool.release(old);
    old.releaseOwner(1);
    assert.equal(h.pool.checkout(), null);
    assert.equal(old.gpuContext.configured, true, 'a live surface keeps its configured context');
    assert.deepEqual(old.contextRequests, [], 'do not touch WebGPU until the final owner drops');
    assert.equal(h.wakes(), 0);
    old.releaseOwner(0);
    assert.equal(old.gpuContext.configured, false, 'retire the context before making it available');
    assert.deepEqual(old.gpuContext.retiredAtSize, [[300, 200]], 'unconfigure before shrinking');
    assert.equal(old.width, 1);
    assert.equal(old.height, 1);
    assert.equal(h.wakes(), 0, 'never re-enter Rust from its destructor');
    await Promise.resolve();
    assert.equal(h.wakes(), 1);
    assert.equal(h.pool.checkout(), old);
});

test('an already-unowned WebGPU canvas is unconfigured before immediate reuse', () => {
    const {pool} = harness(1);
    const old = pool.checkout();
    old.setAttribute('data-pax-surface-owners', '0');
    pool.release(old);
    assert.equal(pool.checkout(), old, 'reuse the DOM element, not its prior surface configuration');
    assert.equal(old.gpuContext.configured, false);
    assert.deepEqual(old.contextRequests, ['webgpu']);
    assert.deepEqual(old.gpuContext.retiredAtSize, [[300, 200]]);
});

test('the deferred availability notification cannot unconfigure a newly assigned surface', async () => {
    const h = harness(1);
    const old = h.pool.checkout();
    old.setAttribute('data-pax-surface-owners', '1');
    h.pool.release(old);
    old.releaseOwner(0);
    assert.equal(old.gpuContext.configured, false);
    const next = h.pool.checkout();
    next.setAttribute('data-pax-surface-owners', '1');
    next.gpuContext.configured = true;
    await Promise.resolve();
    assert.equal(next.gpuContext.configured, true);
    assert.equal(h.wakes(), 1);
});

test('releasing a Piet or unused canvas does not request a WebGPU context', () => {
    const {pool} = harness(1);
    const canvas = pool.checkout();
    pool.release(canvas);
    assert.deepEqual(canvas.contextRequests, []);
    assert.equal(canvas.width, 1);
    assert.equal(canvas.height, 1);
    assert.equal(pool.checkout(), canvas);
});

test('an unowned canvas remains usable while a former owner is still initializing another', () => {
    const {pool} = harness(2);
    const busy = pool.checkout();
    busy.setAttribute('data-pax-surface-owners', '1');
    pool.release(busy);
    const available = pool.checkout();
    assert.ok(available);
    assert.notEqual(available, busy);
    pool.release(available);
    assert.equal(available.width, 1);
    assert.equal(pool.checkout(), available);
});

test('capacity accounting distinguishes assigned, retiring, and reusable canvases', () => {
    const {pool} = harness(3);
    const survivor = pool.checkout(), retiring = pool.checkout();
    retiring.setAttribute('data-pax-surface-owners', '1');
    pool.release(retiring);
    assert.deepEqual({...pool.capacitySnapshot()},
        {limit: 3, allocated: 2, assigned: 1, retiring: 1, available: 1, backingPixels:120000,retiringPixels:60000});
    const replacement = pool.checkout();
    assert.notEqual(replacement, retiring);
    assert.equal(pool.checkout(), null);
    assert.equal(pool.capacitySnapshot().available, 0);
    retiring.releaseOwner(0);
    assert.equal(pool.capacitySnapshot().retiring, 0);
    assert.equal(pool.capacitySnapshot().available, 1);
    assert.equal(pool.checkout(), retiring);
    pool.release(survivor);
    pool.release(replacement);
    pool.release(retiring);
    assert.equal(pool.capacitySnapshot().available, 3);
});

test('physical reservations and retiring leases share one hard backing limit', () => {
    const {pool} = harness(4);
    pool.setBackingLimit(10000);
    const a = pool.checkout(6000);
    assert.ok(a);
    assert.equal(pool.checkout(5000),null,'in-flight reservation counts before canvas resize');
    a.width=100; a.height=60;
    a.setAttribute('data-pax-surface-owners','2');
    pool.release(a);
    assert.equal(pool.capacitySnapshot().retiringPixels,6000);
    const b = pool.checkout(4000);
    assert.ok(b);
    assert.equal(pool.capacitySnapshot().backingPixels,10000,'backing and reservation are not double-counted');
    assert.equal(pool.checkout(1),null);
    a.releaseOwner(1);
    assert.equal(pool.checkout(1),null,'one remaining owner still holds the backing');
    a.releaseOwner(0);
    assert.equal(pool.checkout(6000),a);
    assert.equal(pool.capacitySnapshot().backingPixels,10000);
    assert.equal(pool.reserveBacking(b,4001),false,'resize cannot exceed the hard limit');
});

test('retirement keeps an initializer reservation before its backing is configured', () => {
    const {pool}=harness(2);
    pool.setBackingLimit(10000);
    const pending=pool.checkout(8000);
    pending.setAttribute('data-pax-surface-owners','1');
    assert.equal(pending.width,1);
    pool.release(pending);
    assert.equal(pool.capacitySnapshot().retiringPixels,8000);
    assert.equal(pool.checkout(3000),null);
    pending.releaseOwner(0);
    assert.ok(pool.checkout(10000));
});
