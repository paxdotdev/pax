import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/utils/background-admission.ts', import.meta.url), 'utf8');

test('a held submission lends bounded time across refreshes and settlement repays the debt', () => {
    const grants = [], module = {exports: {}};
    let now = 0, pendingInput = false, legal = true, extra = 0;
    const sharedWorkBudget = {remaining: 4, run: (n, work) => work(n)};
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, require: () => ({sharedWorkBudget}),
        performance: {now: () => now}, document: {hidden: false}, window: new EventTarget(),
        navigator: {scheduling: {isInputPending: () => pendingInput}}, AbortController,
    });
    const chassis = {
        configure_admission_resumption() {}, configure_visible_groups() {}, prioritize_background_owners() {},
        begin_background_mounting: () => true, admission_can_continue: () => legal,
        service_background_mounts(n) { grants.push(n); now += n + extra; return true; },
    };
    const admission = new module.exports.BackgroundAdmission(chassis, new EventTarget(), false, () => {}, false, true);
    admission.begin(); admission.enableInput(); admission.frame(1, now);
    admission.waitingForSubmission();
    assert.deepEqual(grants, [], 'first reveal keeps its animation allowance');
    admission.observedScroll(4, true);
    const wait = epoch => {admission.frame(epoch, now); admission.waitingForSubmission();};
    wait(1); wait(1); wait(2); wait(3);
    assert.deepEqual(grants, [1, 1], 'one unit per refresh and at most two milliseconds before settlement');
    admission.afterTick(12);
    pendingInput = true; wait(4); pendingInput = false;
    legal = false; wait(5); legal = true;
    sharedWorkBudget.remaining = 0.1; wait(6); sharedWorkBudget.remaining = 4;
    extra = 2; wait(7); wait(8);
    assert.deepEqual(grants, [1, 1, 1], 'charge an indivisible overrun before the next opportunity');
});

test('a reversal consumes the nested scroller or the delegated document at its live offset', () => {
    const module = {exports: {}}, root = new EventTarget(), intents = [];
    const document = {scrollingElement: {scrollTop: 900, scrollHeight: 3000, clientHeight: 600}};
    class Scroller {
        constructor(id, top, height) {
            this.id = id;
            this.scrollTop = top;
            this.scrollHeight = height;
            this.clientHeight = 200;
            this.ownerDocument = document;
            this.classList = {contains: name => name === 'scroller-container'};
            this.dataset = {};
        }
        closest() { return this; }
        getAttribute() { return String(this.id); }
    }
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, require: () => ({}), HTMLElement: Scroller,
        performance, document, window: new EventTarget(), AbortController,
    });
    const chassis = {
        configure_admission_resumption() {}, configure_visible_groups() {}, enable_indexed_mounts() {},
        mount_scroll_intent(id) { intents.push(id); },
    };
    const admission = new module.exports.BackgroundAdmission(chassis, root, true, () => {});
    const page = new Scroller(12, 0, 200), child = new Scroller(17, 75, 800);
    page.dataset.pageScrollDelegated = 'true';
    const wheel = deltaY => {
        const event = Object.assign(new Event('wheel'), {deltaX: 0, deltaY, composedPath: () => [child, page]});
        root.dispatchEvent(event);
    };
    wheel(-80);
    child.scrollTop = 0;
    wheel(-80);
    document.scrollingElement.scrollTop = 0;
    wheel(-80);
    child.scrollTop = 600;
    wheel(80);
    document.scrollingElement.scrollTop = 2400;
    wheel(80);
    assert.deepEqual(intents, [17, 12, undefined, 12, undefined]);
    admission.dispose();
});
