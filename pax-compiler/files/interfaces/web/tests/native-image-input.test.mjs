import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

test('native images leave browser hit-testing to underlying scrollers', () => {
    const module = {exports: {}};
    const layers = {addElement(leaf) { this.leaf = leaf; }};
    vm.runInNewContext(transformSync(readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8'),
        {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, console,
        ResizeObserver: class {},
        require: name => name.endsWith('supported-objects')
            ? {DIV: 'div', IMAGE: 'img', RENDER_LAYER_MANAGER: 'layers'} : {},
    });
    const element = () => ({style: {}, attributes: {},
        setAttribute(name, value) { this.attributes[name] = value; },
        appendChild(child) { this.firstChild = child; },
    });
    const pool = new module.exports.NativeElementPool({getFromPool: kind => kind === 'layers' ? layers : element()});
    pool.nativeImageCreate({id: 42, renderLayerId: 0});
    assert.equal(layers.leaf.style.pointerEvents, 'none',
        'CSS occlusion masks only hide pixels; the entire leaf must pass through browser input');
    assert.equal(layers.leaf.firstChild.style.pointerEvents, undefined,
        'the img must inherit the leaf policy rather than re-enable hit-testing');
    assert.equal(layers.leaf.attributes.pax_id, '42', 'the image retains its engine identity');
});
