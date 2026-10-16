import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';
function load(name, context) {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(readFileSync(new URL(`../src/classes/${name}.ts`, import.meta.url), 'utf8'), {loader: 'ts', format: 'cjs'}).code, {...context, module, exports: module.exports});
    return module.exports;
}
const {InitialTextPresentation} = load('initial-text-presentation', {});
const font = ready => {const load = () => ready; return {getTextLoader: () => load, applyFontToDiv() {}};};
const drain = async () => { for (let i = 0; i < 6; ++i) await Promise.resolve(); };
function harness() {
    let callback;
    const observed = new Set(), messages = [], leaves = [], focused = [];
    let flushes = 0;
    const makeDiv = () => ({style: {}, dataset: {}, attrs: {}, isConnected: true,
        setAttribute(k,v) {this.attrs[k]=v;}, getAttribute(k) {return this.attrs[k];},
        addEventListener() {}, appendChild(n) {this.firstChild=n;}, focus() {focused.push(this);},
        textContent: "Example",
        querySelectorAll() {return this.hasLinks ? [{textContent: "Link"}] : [];},
        querySelector() {return this.hasLinks ? {} : null;},
        parentElement: {removeChild(n) {n.isConnected=false;}},
    });
    const layers = {addElement(leaf) {leaves.push(leaf);}};
    const {NativeElementPool} = load('native-element-pool', {
        require: name => name.endsWith('initial-text-presentation') ? {InitialTextPresentation} : name.endsWith('supported-objects') ? {DIV: 'div', RENDER_LAYER_MANAGER: 'layers'} : {},
        console, setTimeout, queueMicrotask,
        window: {getSelection: () => ({removeAllRanges() {}, addRange() {}})},
        document: {createRange: () => ({selectNodeContents() {}})},
        ResizeObserver: class {constructor(cb) {callback=cb;} observe(n) {observed.add(n);} unobserve(n) {observed.delete(n);}},
    });
    const pool = new NativeElementPool({getFromPool: kind => kind === 'layers' ? layers : makeDiv()});
    pool.chassis = {interrupt: m => messages.push(JSON.parse(JSON.stringify(m)))};
    pool.postAsyncInterruptFlush = () => ++flushes;
    pool.applyLeafPlacement = () => {};
    pool.textCreate({id: 1, renderLayerId: 0});
    return {pool, leaf: leaves[0], observed, messages, focused,
        measure: (width=100,height=20) => callback([{target: leaves[0], contentRect: {width,height}}]),
        flushes: () => flushes};
}

test('fallback metrics settle hidden layout; loaded metrics govern first presentation', async () => {
    const h=harness(); let resolve;
    const ready = new Promise(r=>resolve=r);
    const patch={id:1, size_x:100, size_y:-1, measure_generation:7, style:{font:font(ready)}};
    h.pool.textUpdate(patch);
    patch.id=999; // Production patch instances are returned to their object pool.
    h.measure(); h.pool.finishTextFrame();
    assert.deepEqual(h.messages,[{TextMeasurementResponse:{id:1,generation:7,width:100,height:20}}]);
    assert.equal(h.leaf.style.visibility,'hidden');
    resolve(); await drain();
    assert.equal(h.flushes(),2, 'font completion still addresses original node after patch reuse');
    h.pool.finishTextFrame(); h.pool.finishTextFrame();
    assert.equal(h.leaf.style.visibility,'hidden', 'fallback metrics cannot release the font gate');
    h.measure(100,24);
    assert.deepEqual(h.messages[1],{ChassisResizeRequestCollection:[{id:1,width:100,height:24}]});
    h.pool.finishTextFrame(); assert.equal(h.leaf.style.visibility,'hidden');
    h.pool.finishTextFrame(); assert.equal(h.leaf.style.visibility,'');
    h.measure(100,28);
    assert.deepEqual(h.messages[2],{ChassisResizeRequestCollection:[{id:1,width:100,height:28}]});
});

test('editable initial text focuses only after becoming visible', () => {
    const h=harness();
    h.pool.textUpdate({id:1, size_x:100, size_y:20, editable:true});
    assert.equal(h.focused.length,0);
    h.pool.finishTextFrame(); h.pool.finishTextFrame();
    assert.equal(h.leaf.style.visibility,'');
    assert.equal(h.focused[0],h.leaf.firstChild);
});

test('deleted text ignores late font completion and measurement delivery', async () => {
    const h=harness(); let resolve;
    const ready = new Promise(r=>resolve=r);
    h.pool.textUpdate({id:1, size_x:100, size_y:-1, measure_generation:1, style:{font:font(ready)}});
    h.pool.textDelete(1);
    resolve(); await drain(); h.measure(); h.pool.finishTextFrame();
    assert.equal(h.observed.size,0);
    assert.equal(h.messages.length,0);
    assert.equal(h.pool.pendingTextPresentation.size,0);
});

test('a burst of font readiness notifications requests one engine frame', async () => {
    const h=harness();
    for(let i=0;i<1365;i++) h.pool.requestTextFrame();
    assert.equal(h.flushes(),0);
    await drain();
    assert.equal(h.flushes(),1);
});


test('a partial main-style update still waits for the existing link font', async () => {
    const h=harness(); let resolve;
    h.leaf.firstChild.hasLinks=true;
    const main = font(Promise.resolve());
    const link = font(new Promise(r=>resolve=r));
    h.pool.textUpdate({id:1,size_x:100,size_y:20,style:{font:main},style_link:{font:link}});
    h.pool.textUpdate({id:1,style:{font:main,font_size:22}});
    await drain(); h.pool.finishTextFrame(); h.pool.finishTextFrame();
    assert.equal(h.leaf.style.visibility,'hidden');
    resolve(); await drain(); h.pool.finishTextFrame(); h.pool.finishTextFrame();
    assert.equal(h.leaf.style.visibility,'');
});


test('plain text does not wait for an unused link font', async () => {
    const h=harness();
    const main = font(Promise.resolve());
    const link = font(new Promise(() => {}));
    h.pool.textUpdate({id:1,size_x:100,size_y:20,style:{font:main},style_link:{font:link}});
    await drain(); h.pool.finishTextFrame(); h.pool.finishTextFrame();
    assert.equal(h.leaf.style.visibility,'');
});

test('startup waits for the latest measurement, then reveals without extra application ticks', async () => {
    const h = harness(); let resolve;
    const ready = new Promise(r => resolve = r);
    h.pool.textUpdate({id:1,size_x:100,size_y:-1,measure_generation:1,style:{font:font(ready)}});
    h.measure();
    assert.equal(h.pool.startupTextReady(), false);
    resolve(); await drain();
    assert.equal(h.pool.startupTextReady(), false, 'loaded font needs a fresh measurement');
    h.measure();
    assert.equal(h.pool.startupTextReady(), true);
    h.pool.textUpdate({id:1,size_x:120,measure_generation:2});
    assert.equal(h.pool.startupTextReady(), false, 'resize invalidates previously ready text');
    h.measure(120,20);
    assert.equal(h.pool.startupTextReady(), true);
    h.pool.finishTextFrame(true);
    assert.equal(h.leaf.style.visibility, '');
    assert.equal(h.pool.pendingTextPresentation.size, 0);
});

test('previewed text still holds startup open for a new measurement generation', async () => {
    const h = harness();
    h.pool.textUpdate({id:1,size_x:100,size_y:-1,measure_generation:1});
    h.measure(); await drain();
    h.pool.finishTextFrame(); h.pool.finishTextFrame();
    assert.equal(h.leaf.style.visibility, '');
    assert.equal(h.pool.pendingTextPresentation.size, 0);
    h.pool.textUpdate({id:1,size_x:120,measure_generation:2});
    assert.equal(h.pool.startupTextReady(), false);
    h.measure(120,20);
    assert.equal(h.pool.startupTextReady(), true);
});


test('content changes load the new unicode subset and ignore stale readiness', async () => {
    const h = harness(), requests = [], completions = [];
    const load = text => { requests.push(text); return new Promise(resolve => completions.push(resolve)); };
    const style = {font: {getTextLoader: () => load, applyFontToDiv() {}}};
    h.leaf.firstChild.textContent = 'Latin';
    h.pool.textUpdate({id: 1, size_x: 100, size_y: 20, style});
    h.leaf.firstChild.textContent = 'Ж';
    h.pool.textUpdate({id: 1});
    completions[0](); await drain();
    assert.equal(h.pool.startupTextReady(), false);
    completions[1](); await drain();
    assert.equal(h.pool.startupTextReady(), true);
    h.pool.textUpdate({id: 1, style});
    assert.deepEqual(requests, ['Latin', 'Ж'], 'unchanged content/style does not reset readiness');
});
