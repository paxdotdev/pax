import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

function load(name, globals = {}) {
    const module = {exports: {}};
    vm.runInNewContext(transformSync(readFileSync(new URL(`../src/classes/${name}.ts`, import.meta.url), 'utf8'),
        {loader: 'ts', format: 'cjs'}).code, {module, exports: module.exports, require: () => ({}), ...globals});
    return module.exports;
}
const {PreparedMaskImage} = load('prepared-mask-image');
const drain = async () => {for (let i = 0; i < 6; i++) await Promise.resolve();};
const entries = (path = 'A') => [{path, opacity: 0.5, clips: []}];
function preparations() {
    const requests = [];
    const prepareNativeMaskRaster = request => {
        const job = {request, disposed: false, dispose() {this.disposed = true;}};
        job.ready = new Promise((resolve, reject) => {
            job.resolve = () => resolve({url: `blob:${request.entries[0].path}`,
                bounds: {left: 0, top: 0, right: 10, bottom: 5}});
            job.reject = reject;
        });
        requests.push(job);
        return job;
    };
    return {requests, prepareNativeMaskRaster};
}
function fixture() {
    const {requests, prepareNativeMaskRaster} = preparations();
    const element = {style: {webkitMaskImage: 'previous'}};
    return {element, requests, handoff: new PreparedMaskImage(element, prepareNativeMaskRaster)};
}
function applied(element, path) {
    assert.equal(element.style.webkitMaskImage, `linear-gradient(white, white), url("blob:${path}")`);
}

test('keeps ready coverage until its replacement and dimensions can be published together', async () => {
    const {handoff, element, requests} = fixture();
    handoff.update(entries(), 200, 100, 2);
    assert.equal(element.style.webkitMaskImage, 'previous');
    requests[0].resolve(); await drain();
    applied(element, 'A');
    assert.equal(element.style.webkitMaskSize, '200px 100px, 10px 5px');
    assert.equal(element.style.webkitMaskComposite, 'source-out');
    assert.equal(element.style.maskComposite, 'subtract');
    assert.equal(element.style.maskMode, 'alpha');
    handoff.update(entries('B'), 300, 150, 2);
    assert.equal(requests[0].disposed, false);
    requests[1].resolve(); await drain();
    applied(element, 'B');
    assert.equal(element.style.webkitMaskSize, '300px 150px, 10px 5px');
    assert.equal(requests[0].disposed, true);
    assert.equal(handoff.retained, requests[1]);
    assert.equal(handoff.pending, undefined);
});

test('coalesces bursts without starving completed work or preparing duplicates', async () => {
    const {handoff, requests, element} = fixture();
    handoff.update(entries(), 100, 100, 1);
    for (let i = 0; i < 100; i++) handoff.update(entries(`B${i}`), 200, 200, 1);
    assert.equal(requests.length, 1);
    requests[0].resolve(); await drain();
    applied(element, 'A');
    assert.equal(requests.length, 2);
    assert.equal(requests[1].request.entries[0].path, 'B99');
    requests[1].resolve(); await drain();
    handoff.update(entries('B99'), 200, 200, 1);
    assert.equal(requests.length, 2);
});

test('returning to ready geometry suppresses obsolete pending work', async () => {
    const {handoff, requests, element} = fixture();
    handoff.update(entries(), 100, 100, 1); requests[0].resolve(); await drain();
    handoff.update(entries('B'), 200, 200, 1); handoff.update(entries(), 100, 100, 1);
    requests[1].resolve(); await drain();
    applied(element, 'A');
    assert.equal(requests.length, 2);
    assert.equal(requests[1].disposed, true);
    assert.equal(requests[0].disposed, false);
});

test('failure retains coverage, drains newer work, and does not retry forever', async () => {
    const {handoff, requests, element} = fixture();
    handoff.update(entries(), 100, 100, 1); requests[0].resolve(); await drain();
    handoff.update(entries('bad'), 100, 100, 1);
    requests[1].reject(new Error('decode failed')); await drain();
    for (let i = 0; i < 20; i++) handoff.update(entries('bad'), 100, 100, 1);
    assert.equal(requests.length, 2);
    applied(element, 'A');
    handoff.update(entries('bad2'), 100, 100, 1); handoff.update(entries('C'), 300, 300, 1);
    requests[2].reject(new Error('encode failed')); await drain();
    assert.equal(requests[3].request.entries[0].path, 'C');
    requests[3].resolve(); await drain();
    applied(element, 'C');
});

test('jobs snapshot pooled patches and rebuild at a changed DPR without new geometry', async () => {
    const {handoff, requests} = fixture();
    const patch = entries(); patch[0].clips.push('clip A');
    handoff.update(patch, 100, 100, 1);
    patch[0].path = 'reused'; patch[0].clips[0] = 'reused';
    requests[0].resolve(); await drain();
    handoff.setPixelRatio(2);
    assert.equal(requests[1].request.scale, 2);
    assert.equal(requests[1].request.entries[0].path, 'A');
    assert.equal(requests[1].request.entries[0].clips[0], 'clip A');
    requests[1].resolve(); await drain();
    handoff.setPixelRatio(2);
    assert.equal(requests.length, 2);
});

test('disposal cancels retained and pending work even when the DOM element is reused', async () => {
    const {handoff, requests, element} = fixture();
    handoff.update(entries(), 100, 100, 1); requests[0].resolve(); await drain();
    handoff.update(entries('old'), 100, 100, 1); handoff.update(entries('queued'), 100, 100, 1);
    handoff.dispose(); element.style.webkitMaskImage = 'new owner';
    requests[1].resolve(); await drain();
    handoff.update(entries('late'), 100, 100, 1);
    assert.equal(element.style.webkitMaskImage, 'new owner');
    assert.equal(requests.length, 2);
    assert.ok(requests.every(job => job.disposed));
});

function managerFixture(safari = true) {
    const {requests, prepareNativeMaskRaster} = preparations();
    class Element {
        style = {}; dataset = {}; parentElement = null;
        classList = {add() {}, remove() {}, contains: () => false};
        querySelectorAll() {return [];}
    }
    const maskModule = load('prepared-mask-image', {require: () => ({prepareNativeMaskRaster})});
    const {RenderLayerManager} = load('render-layer-context', {
        HTMLElement: Element,
        navigator: {userAgent: safari ? 'AppleWebKit Version/26 Safari/605' : 'AppleWebKit Chrome/140 Safari/537', platform: 'MacIntel'},
        require: name => name === './prepared-mask-image' ? maskModule
            : name === './native-mask-rasterizer' ? {NativeMaskRasterizer: class {
                prepare = prepareNativeMaskRaster;
                dispose() {}
            }} : {},
    });
    const manager = new RenderLayerManager({});
    manager.effects = {updateMask() {}, cleanUp() {}};
    return {manager, element: new Element(), requests};
}

for (const clear of ['empty', 'invalid', 'delete', 'teardown']) {
    test(`renderer ${clear} invalidates pending completions`, async () => {
        const {manager, element, requests} = managerFixture();
        manager.updateElementMask(element, 1, entries(), 100, 100);
        if (clear === 'empty') manager.updateElementMask(element, 1, [], 100, 100);
        if (clear === 'invalid') manager.updateElementMask(element, 1, entries(), 0, 100);
        if (clear === 'delete') manager.removeElementMask(element, 1);
        if (clear === 'teardown') manager.cleanUp();
        element.style.webkitMaskImage = 'new owner';
        requests[0].resolve(); await drain();
        assert.equal(element.style.webkitMaskImage, 'new owner');
        assert.equal(manager.maskImages.size, 0);
        assert.equal(requests[0].disposed, true);
    });
}

test('clearing/readding a mask permits a new owner, not its old completion', async () => {
    const {manager, element, requests} = managerFixture();
    manager.updateElementMask(element, 1, entries(), 100, 100);
    manager.updateElementMask(element, 1, [], 100, 100);
    assert.equal(element.style.maskComposite, '');
    manager.updateElementMask(element, 1, entries('B'), 200, 150);
    requests[1].resolve(); await drain();
    requests[0].resolve(); await drain();
    applied(element, 'B');
    assert.equal(element.style.webkitMaskSize, '200px 150px, 10px 5px');
});

test('DPR changes update existing Safari masks', async () => {
    const {manager, element, requests} = managerFixture();
    manager.updateElementMask(element, 1, entries(), 100, 100);
    requests[0].resolve(); await drain();
    manager.updateMaskPixelRatio(2);
    assert.equal(requests[1].request.scale, 2);
});

test('Chrome retains synchronous SVG fragment masks without bitmap work', () => {
    const {manager, element, requests} = managerFixture(false);
    manager.updateElementMask(element, 7, entries(), 100, 100);
    assert.equal(element.style.maskImage, 'url(#pax-native-mask-7)');
    assert.equal(requests.length, 0);
});

test('native deletion cancels pending masks before forgetting its ID', async () => {
    const {manager, element, requests} = managerFixture();
    const {NativeElementPool} = load('native-element-pool');
    const pool = Object.create(NativeElementPool.prototype);
    Object.assign(pool, {layers: manager, nodesLookup: new Map([[9, element]])});
    manager.updateElementMask(element, 9, entries(), 100, 100);
    pool.forgetNativeElement(9);
    assert.equal(pool.nodesLookup.size, 0);
    assert.equal(manager.maskImages.size, 0);
    requests[0].resolve(); await drain();
    assert.equal(element.style.webkitMaskImage, '');
});

test('host disposal cancels mask preparation before detaching its mount', async () => {
    const {manager, element, requests} = managerFixture();
    const {NativeElementPool} = load('native-element-pool');
    const pool = Object.create(NativeElementPool.prototype);
    Object.assign(pool, {layers: manager, resizeObserver: {disconnect() {}},
        uninstallPageScrollActivityListeners() {}, setDocumentPageScrollMode() {},
        invalidateSurfaceHostObservation() {},
    });
    for (const name of ['canvases', 'admittedPlans', 'layerCanvasPlanCache', 'nodesLookup', 'textStates',
        'pendingTextPresentation', 'scrollerHosts', 'scrollerMeasurementStates', 'presentationRecords',
        'pendingScrollerUpdates']) pool[name] = new Map();
    manager.updateElementMask(element, 9, entries(), 100, 100);
    pool.mount = {set innerHTML(value) {assert.equal(manager.maskImages.size, 0);}};
    pool.dispose(); element.style.webkitMaskImage = 'next mount';
    requests[0].resolve(); await drain();
    assert.equal(element.style.webkitMaskImage, 'next mount');
});
