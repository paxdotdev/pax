import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

class Field {
    value = 'Until Monday!';
    listeners = new Map();
    addEventListener(name, callback) { this.listeners.set(name, callback); }
    fire(name, args = {}) {
        const event = {key: 'Enter', repeat: false, isComposing: false, keyCode: 13,
            defaultPrevented: false, preventDefault() { this.defaultPrevented = true; }, ...args};
        this.listeners.get(name)?.(event);
        return event;
    }
    blur() { assert.fail('submission must not blur the field'); }
}
class Area extends Field {}
const module = {exports: {}};
vm.runInNewContext(transformSync(readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8'), {loader: 'ts', format: 'cjs'}).code, {
    module, exports: module.exports, require: () => ({}), HTMLTextAreaElement: Area,
});
const {NativeElementPool} = module.exports;
function harness(area = false) {
    const messages = [];
    const pool = Object.create(NativeElementPool.prototype);
    pool.chassis = {interrupt(message) { messages.push(JSON.parse(JSON.stringify(message))); }};
    const field = area ? new Area() : new Field();
    pool.attachTextboxListeners(field, 42);
    return {field, messages};
}

test('Return submits exactly once with final text and blocks embedding HTML implicit submission', () => {
    const {field, messages} = harness();
    field.value = 'Final value';
    assert.equal(field.fire('keydown').defaultPrevented, true);
    field.fire('keyup');
    assert.deepEqual(messages, [{FormTextboxSubmit: {id: 42, text: 'Final value'}}]);
});

test('editing and blur/change do not submit', () => {
    const {field, messages} = harness();
    field.fire('input'); field.fire('change'); field.fire('blur');
    assert.deepEqual(messages, [
        {FormTextboxInput: {id: 42, text: 'Until Monday!'}},
        {FormTextboxChange: {id: 42, text: 'Until Monday!'}},
    ]);
});

test('multiline Return keeps native newline behavior', () => {
    const {field, messages} = harness(true);
    assert.equal(field.fire('keydown').defaultPrevented, false);
    assert.deepEqual(messages, []);
});

test('repeat, non-Return keys, and IME confirmation do not submit', () => {
    for (const args of [{repeat: true}, {key: 'a'}, {isComposing: true}, {keyCode: 229}]) {
        const {field, messages} = harness();
        field.fire('keydown', args);
        assert.deepEqual(messages, []);
    }
    const {field, messages} = harness();
    field.fire('compositionstart');
    field.fire('keydown');
    assert.deepEqual(messages, []);
    field.fire('compositionend');
    field.fire('keydown', {keyCode: 229});
    assert.deepEqual(messages, []);
    field.fire('keydown');
    assert.equal(messages.length, 1);
});
