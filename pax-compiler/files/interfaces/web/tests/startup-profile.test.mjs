import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/utils/startup-profile.ts', import.meta.url), 'utf8');
function harness(search) {
    const module = {exports: {}};
    const window = {location: {search}};
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, window, URLSearchParams,
        performance: {now: () => 42}, console: {info() {}},
    });
    return {window, capture: module.exports.captureStartupProfile};
}

test('startup capture is opt-in and does not consume a disabled chassis', () => {
    for (const search of ['', '?pax_startup_profile=0', '?pax_startup_profile=true']) {
        const h = harness(search);
        h.capture({take_startup_profile() { assert.fail('must stay disabled'); }}, false);
        assert.equal(h.window.paxStartupProfiles, undefined);
    }
});

test('completed loader/main captures retain attribution and bound retry history', () => {
    const h = harness('?pax_startup_profile=1');
    for (let i = 0; i < 12; i++) {
        const report = {services: i, service_ms: i, reactive: {totals: {effects: i}}};
        let pending = report;
        const chassis = {take_startup_profile() { const r = pending; pending = null; return r; }};
        h.capture(chassis, i % 2 === 0);
        h.capture(chassis, false);
    }
    const captures = h.window.paxStartupProfiles;
    assert.equal(captures.length, 8);
    assert.equal(captures[0].report.services, 4);
    assert.equal(captures[0].application, 'loader');
    assert.equal(captures.at(-1).application, 'main');
    assert.equal(captures.at(-1).completed_at_ms, 42);
});
