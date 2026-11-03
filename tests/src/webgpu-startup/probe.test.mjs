import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';

const source = readFileSync(new URL('./probe.js', import.meta.url), 'utf8');

function probe(query = 'pax_renderer=gpu&gpu_test=real', requestAdapter) {
    const output = {dataset: {}, style: {}, textContent: ''};
    const layers = [];
    class Canvas {
        getContext(kind) {
            if (this.kind && this.kind !== kind) return null;
            this.kind = kind;
            return this.context ??= {};
        }
    }
    const gpu = {requestAdapter: requestAdapter ?? (() => Promise.resolve({
        requestDevice: () => Promise.resolve({}),
    }))};
    const window = {};
    const console = Object.fromEntries(['info', 'log', 'warn', 'error'].map(level => [level, () => {}]));
    runInNewContext(source, {
        URLSearchParams, location: {search: `?${query}`}, window, console,
        navigator: {gpu}, HTMLCanvasElement: Canvas,
        performance: {now: () => 0}, setInterval: () => 1, clearInterval: () => {},
        document: {
            body: {appendChild: () => output}, createElement: () => output,
            querySelectorAll: selector => {
                assert.equal(selector, 'canvas[data-layer-id]');
                return layers;
            },
            addEventListener: () => {}, visibilityState: 'visible',
        },
    });
    return {
        gpu, console, output,
        canvas(kind, attached = true) {
            const canvas = new Canvas();
            if (attached) layers.push(canvas);
            if (kind) canvas.getContext(kind);
            return canvas;
        },
        finish(mounted = true) {
            window.finishGpuProbe(mounted);
            return output.dataset.result;
        },
    };
}

async function acquireGpu(p) {
    const adapter = await p.gpu.requestAdapter({});
    await adapter.requestDevice({});
}

test('GPU mount passes without renderer logs; null acquisitions do not overwrite the context', async () => {
    const p = probe();
    await acquireGpu(p);
    const canvas = p.canvas('webgpu');
    assert.equal(canvas.getContext('2d'), null);
    p.canvas('2d', false); // Auxiliary canvas, outside the mounted Pax layers.
    p.canvas(); // Warm tile whose initialization is still pending.
    assert.equal(p.finish(), 'pass');
    assert.match(p.output.textContent, /renderer=gpu adapters=1 devices=1 canvases=1/);
});

test('CPU fallback ignores a detached GPU initialization canvas and stale log wording', async () => {
    const p = probe('pax_renderer=auto&gpu_test=device-reject');
    p.canvas('webgpu', false);
    await assert.rejects(acquireGpu(p), /Injected device failure/);
    p.canvas('2d');
    p.console.info('using WebGPU browser renderer');
    assert.equal(p.finish(), 'pass');
    assert.match(p.output.textContent, /renderer=cpu adapters=1 devices=1/);
});

test('explicit CPU requires no adapter or device requests', () => {
    const p = probe('pax_renderer=cpu&gpu_test=real');
    p.canvas('2d');
    assert.equal(p.finish(), 'pass');
    assert.match(p.output.textContent, /renderer=cpu adapters=0 devices=0/);
});

test('mount success and a matching log cannot hide missing or wrong layer contexts', async () => {
    for (const kinds of [[], ['2d'], ['webgpu', '2d']]) {
        const p = probe();
        await acquireGpu(p);
        p.canvas('webgpu', false);
        for (const kind of kinds) p.canvas(kind);
        p.console.info('using WebGPU browser renderer');
        assert.equal(p.finish(), 'fail', `contexts: ${kinds}`);
    }
});

test('context failure intercepts WebGPU only and still counts unexpected adapter requests', async () => {
    for (const requested of ['auto', 'gpu']) {
        const p = probe(`pax_renderer=${requested}&gpu_test=context-null`);
        const canvas = p.canvas(undefined, false);
        assert.equal(canvas.getContext('webgpu'), null);
        if (requested === 'auto') p.canvas('2d');
        assert.equal(p.finish(requested === 'auto'), 'pass');
    }
    const unexpected = probe('pax_renderer=auto&gpu_test=context-null');
    await unexpected.gpu.requestAdapter({});
    unexpected.canvas('2d');
    assert.equal(unexpected.finish(), 'fail');
});

test('a redundant adapter acquisition still fails an otherwise successful mount', async () => {
    const p = probe();
    await acquireGpu(p);
    await assert.rejects(p.gpu.requestAdapter({}), /redundant adapter acquisition/);
    p.canvas('webgpu');
    assert.equal(p.finish(), 'fail');
});

test('a pending real adapter request stays pending', async () => {
    let resolveAdapter;
    const pending = new Promise(resolve => { resolveAdapter = resolve; });
    const p = probe(undefined, () => pending);
    assert.equal(p.gpu.requestAdapter({}), pending);
    await Promise.resolve();
    assert.equal(p.output.dataset.result, undefined);
    resolveAdapter(null);
    await pending;
});
