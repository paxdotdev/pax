import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
const module = {exports: {}};
vm.runInNewContext(transformSync(readFileSync(new URL('../src/classes/initial-text-presentation.ts', import.meta.url), 'utf8'), {loader: 'ts', format: 'cjs'}).code, {module, exports: module.exports});
const {InitialTextPresentation} = module.exports;
const drain = async () => { for (let i = 0; i < 5; ++i) await Promise.resolve(); };

test('autosized text waits for font, measurement, and parent layout feedback', async () => {
    const state = new InitialTextPresentation();
    state.updated = true;
    let loaded;
    const font = new Promise(resolve => loaded = resolve);
    let ready = 0;
    state.waitForFonts([font], () => ++ready);
    state.requestMeasurement(1);
    assert.equal(state.afterFrame(), false);
    loaded(); await drain();
    assert.equal(ready, 1);
    assert.equal(state.afterFrame(), false);
    assert.equal(state.measuredSize(), 1);
    assert.equal(state.afterFrame(), false);
    assert.equal(state.afterFrame(), true);
    state.requestMeasurement(2);
    assert.equal(state.presented, true, 'subsequent edits never hide already presented text');
});

test('superseded fonts cannot release text; failed current font permits fallback', async () => {
    const state = new InitialTextPresentation();
    state.updated = true;
    let oldLoaded, fail;
    const oldFont = new Promise(resolve => oldLoaded = resolve);
    const newFont = new Promise((_, reject) => fail = reject);
    let notifications = 0;
    state.waitForFonts([oldFont], () => ++notifications);
    state.waitForFonts([newFont], () => ++notifications);
    oldLoaded(); await drain();
    assert.equal(state.fontsReady, false);
    assert.equal(notifications, 0);
    fail(new Error('missing font')); await drain();
    assert.equal(notifications, 1);
    assert.equal(state.afterFrame(), false);
    assert.equal(state.afterFrame(), true);
});

test('fixed size does not require a resize and switching out of autosize cancels waiting', () => {
    const state = new InitialTextPresentation();
    assert.equal(state.afterFrame(), false, 'a create without its update stays hidden');
    state.updated = true;
    state.requestMeasurement(3);
    state.fixedSize();
    assert.equal(state.afterFrame(), false);
    assert.equal(state.afterFrame(), true);
});

test('continuous size updates cannot starve first paint after initial metrics', async () => {
    const state = new InitialTextPresentation(); state.updated = true;
    const font = Promise.resolve();
    state.waitForFonts([font], () => {}); await drain();
    state.requestMeasurement(1); state.measuredSize(); state.afterFrame();
    state.requestMeasurement(2);
    assert.equal(state.measuredSize(), 2);
    state.waitForFonts([font], () => assert.fail('same font should not restart readiness'));
    assert.equal(state.afterFrame(), true);
    assert.equal(state.presented, true);
});
