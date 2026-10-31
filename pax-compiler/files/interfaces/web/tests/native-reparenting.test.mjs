import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const module = {exports: {}};
const source = readFileSync(new URL('../src/classes/render-layer-context.ts', import.meta.url), 'utf8');
vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
    module, exports: module.exports, HTMLElement: class {},
    require: () => ({}),
});
const {RenderLayerManager} = module.exports;

function harness(atomic = true) {
    const document = {};
    const root = {ownerDocument: document, isConnected: true, moves: 0, appends: 0,
        contains(node) { return node === leaf || node === scroller; },
        appendChild(node) { this.appends++; node.parentElement = this; },
    };
    if (atomic) root.moveBefore = function(node, before) {
        assert.equal(before, null);
        assert.equal(node.parentElement, scroller);
        this.moves++;
        node.parentElement = this;
    };
    const scroller = {parentElement: root};
    const leaf = {parentElement: scroller, ownerDocument: document, isConnected: true,
        querySelectorAll: () => [],
    };
    const manager = Object.create(RenderLayerManager.prototype);
    manager.growTo = () => {};
    manager.getOrCreateContainer = () => root;
    return {manager, root, scroller, leaf};
}

test('a leaf moves out of a scroller into its ancestor root layer atomically', () => {
    const {manager, root, leaf} = harness();
    assert.ok(root.contains(leaf));
    manager.addElement(leaf, undefined, 0);
    assert.equal(leaf.parentElement, root);
    assert.equal(root.moves, 1);
    assert.equal(root.appends, 0);
    manager.addElement(leaf, undefined, 0);
    assert.equal(root.moves, 1, 'unchanged placement must not disturb native state');
});

test('older browsers and disconnected leaves use normal attachment', () => {
    for (const atomic of [true, false]) {
        const {manager, root, leaf} = harness(atomic);
        if (atomic) leaf.isConnected = false;
        manager.addElement(leaf, undefined, 0);
        assert.equal(leaf.parentElement, root);
        assert.equal(root.moves, 0);
        assert.equal(root.appends, 1);
    }
});
