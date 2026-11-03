import test from 'node:test';
import assert from 'node:assert/strict';
import {createSplitLoader} from '../src/utils/split-loader.js';

const minimal = Uint8Array.of(0, 97, 115, 109, 1, 0, 0, 0);
const instantiate = WebAssembly.instantiate;
const Instance = WebAssembly.Instance;
function deferred() {
    let resolve;
    const promise = new Promise(done => { resolve = done; });
    return {promise, resolve};
}
function fixture(bytes = minimal, imports = {}) {
    let requests = 0;
    const loader = createSplitLoader({
        url: 'fixture', getImports: () => imports,
        fetcher: async () => { requests++; return new Response(bytes); },
    });
    return {loader, requests: () => requests};
}
function delayInstallation(t) {
    const started = deferred(), finish = deferred();
    let calls = 0;
    t.mock.method(WebAssembly, 'instantiate', async (module, imports) => {
        assert.ok(module instanceof WebAssembly.Module);
        calls++;
        started.resolve();
        await finish.promise;
        return instantiate(module, imports);
    });
    return {started: started.promise, finish: finish.resolve, calls: () => calls};
}

test('uses asynchronous instantiation, never the size-limited synchronous constructor', async t => {
    t.mock.method(WebAssembly, 'Instance', function () {
        throw new RangeError('synchronous instantiation is not allowed');
    });
    const {loader} = fixture();
    assert.ok(await loader.load() instanceof Instance);
    assert.equal(loader.state.phase, 'ready');
});

test('loads an application payload larger than the browser synchronous limit', async () => {
    // A custom section makes this a valid >8 MiB module without expensive code.
    const payload = new Uint8Array(8 * 1024 * 1024 + 1);
    payload.set([1, 120]); // one-byte custom-section name: "x"
    let size = payload.length;
    const leb = [];
    do { const byte = size & 127; size >>>= 7; leb.push(byte | (size ? 128 : 0)); } while (size);
    const bytes = new Uint8Array(minimal.length + 1 + leb.length + payload.length);
    bytes.set(minimal);
    bytes.set([0, ...leb], minimal.length);
    bytes.set(payload, minimal.length + 1 + leb.length);
    const {loader} = fixture(bytes);
    assert.ok(await loader.load() instanceof Instance);
    assert.equal(loader.state.received, bytes.length);
    assert.equal(loader.state.phase, 'ready');
});

test('cancellation during installation preserves a single shared, reusable result', async t => {
    const installation = delayInstallation(t);
    const {loader, requests} = fixture();
    const phases = [];
    loader.observe(state => phases.push(state.phase));
    const original = loader.load();
    await installation.started;
    loader.cancel();
    // A replacement view must join the in-flight install, not install twice.
    const replacement = loader.load();
    assert.equal(replacement, original);
    assert.equal(loader.state.phase, 'installing');
    installation.finish();
    const [first, second] = await Promise.all([original, replacement]);
    assert.equal(first, second);
    assert.equal(await loader.load(), first);
    assert.equal(requests(), 1);
    assert.equal(installation.calls(), 1);
    assert.equal(loader.state.phase, 'ready');
    assert.ok(!phases.includes('cancelled'));
});

test('cancellation cannot hide a trapping install or allow another write to shared memory', async t => {
    const installation = delayInstallation(t);
    const memory = new WebAssembly.Memory({initial: 1});
    // Import env.memory, initialize byte 0 to 42, then trap in the start function.
    const trap = Uint8Array.from([
        ...minimal, 1, 4, 1, 96, 0, 0,
        2, 15, 1, 3, 101, 110, 118, 6, 109, 101, 109, 111, 114, 121, 2, 0, 1,
        3, 2, 1, 0, 8, 1, 0, 10, 5, 1, 3, 0, 0, 11,
        11, 7, 1, 0, 65, 0, 11, 1, 42,
    ]);
    const {loader, requests} = fixture(trap, {env: {memory}});
    const pending = loader.load();
    const rejected = assert.rejects(pending, error => {
        assert.match(error.message, /recreate the runtime/);
        assert.ok(error.cause instanceof WebAssembly.RuntimeError);
        return true;
    });
    await installation.started;
    loader.cancel();
    assert.equal(loader.load(), pending);
    installation.finish();
    await rejected;
    assert.equal(new Uint8Array(memory.buffer)[0], 42);
    assert.equal(loader.state.phase, 'error');
    assert.equal(loader.state.failedPhase, 'installing');
    assert.equal(loader.state.retryable, false);
    new Uint8Array(memory.buffer)[0] = 7;
    await assert.rejects(loader.load(), /recreate the runtime/);
    assert.equal(new Uint8Array(memory.buffer)[0], 7);
    assert.equal(requests(), 1);
    assert.equal(installation.calls(), 1);
});

test('each Rust callback settles exactly once after a delayed installation', async t => {
    const installation = delayInstallation(t);
    const {loader} = fixture();
    const calls = [], completed = deferred();
    const callback = loader.callback(() => ({get: index => (data, success) => {
        calls.push([index, data, success]);
        if (calls.length === 2) completed.resolve();
    }}));
    callback(2, 4);
    await installation.started;
    loader.cancel();
    callback(3, 5);
    assert.deepEqual(calls, []);
    installation.finish();
    await completed.promise;
    assert.deepEqual(calls, [[2, 4, true], [3, 5, true]]);
    assert.equal(installation.calls(), 1);
});
