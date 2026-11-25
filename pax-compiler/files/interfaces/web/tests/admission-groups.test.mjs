import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/classes/admission-groups.ts', import.meta.url), 'utf8');
const module = {exports: {}};
vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code,
    {module, exports: module.exports});
const {AdmissionGroups} = module.exports;

test('incoming native content waits through application and an unrelated ready group opens', () => {
    const opened = [];
    const chassis = {
        ready_visible_groups: () => [[1, 7, [10]], [2, 8, [20, 21]]],
        open_visible_group: (...receipt) => opened.push(receipt),
    };
    const ready = new Set([10, 20]);
    const owner = new AdmissionGroups(chassis, {firstContentState: id => ready.has(id) ? 'ready' : 'text'});
    owner.advance([[{TextUpdate: {id: 10, content: 'replacement'}}]]);
    assert.deepEqual(opened, []);
    owner.advance([[]]);
    assert.deepEqual(opened, [[1, 7]]);
    ready.add(21);
    owner.advance([[{NativeImageUpdate: {id: 21, url: '/updated.png'}}]]);
    assert.deepEqual(opened, [[1, 7], [1, 7]]);
    owner.advance([[]]);
    assert.deepEqual(opened.at(-1), [2, 8]);
});

test('missing native producers identify the group, publication and element', () => {
    const groups = new AdmissionGroups({ready_visible_groups: () => [[17, 3, [42]]]},
        {firstContentState: () => 'missing'});
    groups.advance([[{TextCreate: {id: 42}}]]);
    assert.throws(() => groups.advance([[]]), /group 17 .*publication 3.*native content 42/);
});

test('replacement native elements retain their own inert state after publication', () => {
    const leaf = inert => ({inert, hiddenByGroup: false,
        setAttribute() { this.hiddenByGroup = true; },
        removeAttribute() { this.hiddenByGroup = false; }});
    const first = leaf(false), replacement = leaf(true);
    let current = first;
    const groups = new AdmissionGroups({}, {admissionNode: () => current});
    groups.publish([3]);
    assert.equal(first.inert, true);
    current = replacement;
    groups.publish([3]);
    assert.equal(first.inert, false);
    assert.equal(replacement.hiddenByGroup, true);
    groups.publish([]);
    assert.equal(replacement.hiddenByGroup, false);
    assert.equal(replacement.inert, true);
});
