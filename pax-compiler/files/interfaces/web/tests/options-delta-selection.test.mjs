import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Run the production create/update methods and their registered change handlers.
// This small DOM model covers selection reset and event bubbling, not browser
// input, accessibility or painting (which need browser integration checks).
class Element {
    constructor(tag) {
        this.tag = tag.toLowerCase();
        this.dataset = {};
        this.style = {};
        this.children = [];
        this.listeners = new Map();
        this._checked = false;
        this.checkedWrites = 0;
        this.queries = 0;
        this._selectedIndex = -1;
    }
    get firstChild() { return this.children[0] ?? null; }
    get parentElement() { return this.parentNode; }
    get options() { return this.children; }
    get checked() { return this._checked; }
    set checked(value) {
        this._checked = value;
        this.checkedWrites++;
    }
    get selectedIndex() { return this._selectedIndex; }
    set selectedIndex(index) {
        this._selectedIndex = index >= 0 && index < this.children.length ? index : -1;
    }
    set innerHTML(value) {
        assert.equal(value, '', 'this model only supports clearing children');
        for (const child of this.children) child.parentNode = null;
        this.children = [];
        this._selectedIndex = -1;
    }
    appendChild(child) {
        child.parentNode = this;
        this.children.push(child);
        // A non-multiple HTML select chooses its first option after rebuilding.
        // New radio inputs remain unchecked until code or user input selects one.
        if (this.tag === 'select' && this._selectedIndex < 0) this._selectedIndex = 0;
        return child;
    }
    setAttribute(name, value) { this[name] = value; }
    matches(selector) {
        assert.equal(selector, "input[type='radio']");
        return this.tag === 'input' && this.type === 'radio';
    }
    querySelector(selector) {
        this.queries++;
        for (const child of this.children) {
            if (child.matches(selector)) return child;
            const descendant = child.querySelector(selector);
            if (descendant) return descendant;
        }
        return null;
    }
    addEventListener(type, listener) {
        const listeners = this.listeners.get(type) ?? [];
        listeners.push(listener);
        this.listeners.set(type, listeners);
    }
    dispatchChange() {
        const event = {target: this};
        for (let current = this; current; current = current.parentNode) {
            for (const listener of current.listeners.get('change') ?? []) listener(event);
        }
    }
}

const source = readFileSync(new URL('../src/classes/native-element-pool.ts', import.meta.url), 'utf8');
const module = {exports: {}};
vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
    module, exports: module.exports,
    document: {createElement: tag => new Element(tag)},
    require: () => ({DIV: 'div', SELECT: 'select'}),
});
const {NativeElementPool} = module.exports;

function harness(kind) {
    const events = [];
    const mount = new Element('div');
    const pool = Object.create(NativeElementPool.prototype);
    Object.assign(pool, {
        nodesLookup: new Map(),
        objectManager: {getFromPool: tag => new Element(tag)},
        layers: {addElement: leaf => mount.appendChild(leaf)},
        chassis: {interrupt: message => events.push(JSON.parse(JSON.stringify(message)))},
        applyLeafPlacement() {},
    });
    let nextId = 1;
    return {
        events,
        create() {
            const id = nextId++;
            pool[`${kind}Create`]({id, renderLayerId: 0});
            const control = pool.nodesLookup.get(id).firstChild;
            return {
                id, control,
                update: patch => pool[`${kind}Update`]({id, ...patch}),
                selected: () => kind === 'dropdown'
                    ? control.selectedIndex
                    : control.children.findIndex(row => row.firstChild.checked),
                labels: () => control.children.map(child => kind === 'dropdown'
                    ? child.textContent : child.children[1].textContent),
                choose(index) {
                    assert.ok(index >= 0 && index < control.children.length);
                    if (kind === 'dropdown') {
                        control.selectedIndex = index;
                        control.dispatchChange();
                    } else {
                        // Native radio activation checks one member, then bubbles
                        // change to the production fieldset handler.
                        control.children.forEach((row, i) => { row.firstChild.checked = i === index; });
                        control.children[index].firstChild.dispatchChange();
                    }
                },
            };
        },
    };
}

test('radioList: geometry-only updates leave check state and option lookup untouched', () => {
    const h = harness('radioList');
    const node = h.create();
    node.update({options: ['Alpha', 'Beta', 'Gamma'], selected_id: 1});
    const rows = [...node.control.children];
    const before = rows.map(row => [row.queries, row.firstChild.checkedWrites]);
    node.update({size_x: 320, size_y: 120});
    node.update({});
    assert.deepEqual(node.control.children, rows, 'placement must retain the existing native inputs');
    assert.deepEqual(rows.map(row => [row.queries, row.firstChild.checkedWrites]), before);
    assert.equal(node.selected(), 1);
    assert.deepEqual(h.events, []);
});

for (const kind of ['dropdown', 'radioList']) {
    const eventName = kind === 'dropdown' ? 'FormDropdownChange' : 'FormRadioListChange';

    test(`${kind}: options-only deltas preserve the unchanged requested index`, () => {
        const h = harness(kind);
        const node = h.create();
        node.update({options: ['Alpha', 'Beta', 'Gamma'], selected_id: 1});
        assert.equal(node.selected(), 1);
        node.update({options: ['One', 'Two', 'Three']});
        assert.deepEqual(node.labels(), ['One', 'Two', 'Three']);
        assert.equal(node.selected(), 1, 'rebuilding DOM options must not reset selection');
        node.update({options: ['One', 'Two', 'Three']});
        node.update({});
        assert.equal(node.selected(), 1);
        assert.deepEqual(h.events, [], 'programmatic patches are not native user input');
    });

    test(`${kind}: empty/refill and invalid indices retain the requested selection`, () => {
        const h = harness(kind);
        const node = h.create();
        node.update({options: ['Alpha', 'Beta'], selected_id: 0});
        node.update({options: []});
        assert.equal(node.selected(), -1);
        node.update({options: ['Restored', 'Other']});
        assert.equal(node.selected(), 0, 'zero is a real requested selection');

        node.update({selected_id: 3});
        assert.equal(node.selected(), -1, 'out-of-range indices must not select the first item');
        node.update({options: ['A', 'B', 'C']});
        assert.equal(node.selected(), -1);
        node.update({options: []});
        node.update({options: ['A', 'B', 'C', 'D']});
        assert.equal(node.selected(), 3, 'refill makes the retained request valid again');

        node.update({options: [], selected_id: 1});
        node.update({options: ['A', 'B']});
        assert.equal(node.selected(), 1, 'selection-only changes while empty must survive refill');
        assert.deepEqual(h.events, []);
    });

    test(`${kind}: native change updates the retained request without leaking to a peer`, () => {
        const h = harness(kind);
        const node = h.create();
        const peer = h.create();
        for (const item of [node, peer]) {
            item.update({options: ['Alpha', 'Beta', 'Gamma'], selected_id: 0});
        }
        node.choose(2);
        assert.deepEqual(h.events, [{[eventName]: {id: node.id, selected_id: 2}}]);
        for (const item of [node, peer]) item.update({options: ['One', 'Two', 'Three']});
        assert.equal(node.selected(), 2, 'later options must preserve the latest user choice');
        assert.equal(peer.selected(), 0, 'each native node owns its requested selection');
        node.update({options: []});
        node.update({options: ['A', 'B', 'C']});
        assert.equal(node.selected(), 2);
        node.update({selected_id: 1});
        node.update({options: ['X', 'Y', 'Z']});
        assert.equal(node.selected(), 1, 'an explicit engine update supersedes the user choice');
        assert.equal(h.events.length, 1, 'only the native change may emit an interrupt');
    });
}
