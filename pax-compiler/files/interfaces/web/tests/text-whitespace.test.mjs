import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const module = {exports: {}};
vm.runInNewContext(transformSync(readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8'),
    {loader: 'ts', format: 'cjs'}).code, {
    module, exports: module.exports, console, require: () => ({}),
});

test('plain Text preserves whitespace through wrap and Markdown patch changes', () => {
    const pool = Object.create(module.exports.NativeElementPool.prototype);
    const text = {style: {}, dataset: {}, querySelectorAll: () => []};
    const leaf = {style: {width: '400px', height: '200px'}, firstChild: text};
    pool.nodesLookup = new Map([[7, leaf]]);
    pool.textStates = new Map([[7, {fixedSize() {}, waitForTextFonts() {}}]]);
    pool.resizeObserver = {unobserve() {}};
    pool.applyLeafPlacement = () => {};
    const content = '  leading\n    relative\n\ntrailing  \n<Text>{x}</Text> & \\n';
    pool.textUpdate({id: 7, content, wrap: true, markdown: false});
    assert.equal(text.innerText, content);
    assert.equal(text.style.whiteSpace, 'pre-wrap');
    pool.textUpdate({id: 7, wrap: false});
    assert.equal(text.style.whiteSpace, 'pre');
    pool.textUpdate({id: 7, markdown: true});
    assert.equal(text.style.whiteSpace, 'pre');
    pool.textUpdate({id: 7, wrap: true});
    assert.equal(text.style.whiteSpace, 'normal');
    pool.textUpdate({id: 7, markdown: false});
    assert.equal(text.style.whiteSpace, 'pre-wrap');
    assert.equal(text.innerText, content);
});
