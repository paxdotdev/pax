import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function harness(diagnosticMode, prevent = true) {
    const listeners = new Map(), samples = [], interrupts = [];
    let now = 100, clockReads = 0;
    class Element { closest() { return null; } }
    const body = new Element();
    const module = {exports: {}};
    const source = readFileSync(new URL('../src/events/listeners.ts', import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(source, {loader:'ts', format:'cjs'}).code, {
        module, exports:module.exports, Element,
        performance: {now: () => {clockReads++; return now;}, timeOrigin: 1700000000000},
        window: {
            addEventListener: (name, fn, options) => listeners.set(name, {fn, options}),
            removeEventListener: (name, fn, options) => {
                assert.equal(listeners.get(name).fn, fn);
                const capture = o => typeof o === 'boolean' ? o : !!o?.capture;
                assert.equal(capture(listeners.get(name).options), capture(options));
                listeners.delete(name);
            },
        },
        document: {body, activeElement: body},
    });
    const dispose = module.exports.setupEventListeners({interrupt: value => {
        interrupts.push(value); now += 2; return {prevent_default: prevent};
    }}, diagnosticMode == null ? undefined : {passive: diagnosticMode, record: s => samples.push(s)});
    const event = {target: body, clientX: 10, clientY: 20, deltaX: 0, deltaY: 40,
        timeStamp: 70, isTrusted: true, cancelable: true, defaultPrevented: false,
        preventDefault() {if (this.cancelable) this.defaultPrevented = true;},
    };
    return {listeners, samples, interrupts, event, dispose, api: module.exports,
        clockReads: () => clockReads};
}

test('normal wheel path retains synchronous cancellation without timing overhead', () => {
    const h = harness();
    const listener = h.listeners.get('wheel');
    assert.equal(listener.options.passive, false);
    assert.equal(listener.options.capture, true);
    listener.fn(h.event);
    assert.equal(h.event.defaultPrevented, true);
    assert.equal(h.interrupts[0].Wheel.delta_y, 40);
    assert.equal(h.clockReads(), 0);
    h.dispose();
    assert.equal(h.listeners.size, 0);
});

test('blocking diagnostics separate delivery delay from synchronous dispatch time', () => {
    const h = harness(false);
    h.listeners.get('wheel').fn(h.event);
    assert.equal(h.samples[0].deliveryDelayMs, 30);
    assert.equal(h.samples[0].handlerMs, 2);
    assert.equal(h.samples[0].cancellationRequested, true);
    assert.equal(h.samples[0].defaultPrevented, true);
    assert.equal(h.samples[0].trusted, true);
    assert.equal(h.samples[0].passive, false);
    assert.equal('target' in h.samples[0], false);
    h.dispose();
});

test('passive A/B still dispatches handlers but explicitly cannot cancel browser scrolling', () => {
    const h = harness(true);
    const listener = h.listeners.get('wheel');
    assert.equal(listener.options.passive, true);
    h.event.preventDefault = () => assert.fail('never call preventDefault from a passive listener');
    listener.fn(h.event);
    assert.equal(h.interrupts.length, 1);
    assert.equal(h.samples[0].cancellationRequested, true);
    assert.equal(h.samples[0].defaultPrevented, false);
    h.dispose();
});

test('wheel timestamps support epoch hosts and reject incompatible clocks', () => {
    const h = harness();
    const {wheelDeliveryDelay} = h.api;
    const origin = 1700000000000;
    assert.equal(wheelDeliveryDelay(origin + 70, 100, origin), 30);
    assert.equal(wheelDeliveryDelay(100, 100, origin), 0);
    for (const timestamp of [0, -1, NaN, undefined, Infinity, 101, origin - 1]) {
        assert.equal(wheelDeliveryDelay(timestamp, 100, origin), null);
    }
    h.dispose();
});

test('passive comparison requires both diagnostic flags and an explicit value of 1', () => {
    const h = harness();
    const mode = query => h.api.wheelDiagnosticMode(new URLSearchParams(query));
    assert.equal(mode('pax_wheel_passive=1'), undefined);
    assert.equal(mode('pax_scroll_perf=1&pax_wheel_passive=1'), undefined);
    assert.equal(mode('pax_surface_trace=1&pax_wheel_passive=1'), undefined);
    assert.equal(mode('pax_surface_trace=1&pax_scroll_perf=1'), false);
    assert.equal(mode('pax_surface_trace=1&pax_scroll_perf=1&pax_wheel_passive=0'), false);
    assert.equal(mode('pax_surface_trace=1&pax_scroll_perf=1&pax_wheel_passive=1'), true);
    h.dispose();
});

test('noncancelable/synthetic wheel diagnostics do not claim cancelled trusted input', () => {
    const h = harness(false);
    h.event.cancelable = false;
    h.event.isTrusted = false;
    h.listeners.get('wheel').fn(h.event);
    assert.equal(h.samples[0].defaultPrevented, false);
    assert.equal(h.samples[0].cancelable, false);
    assert.equal(h.samples[0].trusted, false);
    h.dispose();
});
