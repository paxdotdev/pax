import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

// Exercise the shipped embed script without needing a browser or a WASM build.
// Visual layout and real iframe teardown are also checked in the review preview.
class Element {
    constructor(tag) {
        this.tagName = tag.toUpperCase();
        this.children = [];
        this.attributes = {};
        this.style = {};
        this.listeners = {};
        this.textContent = '';
    }
    setAttribute(name, value) { this.attributes[name] = value; }
    getAttribute(name) { return this.attributes[name] ?? null; }
    appendChild(child) { this.children.push(child); child.parentElement = this; return child; }
    append(...children) { children.forEach(child => this.appendChild(child)); }
    replaceChildren(...children) {
        this.children.forEach(child => { child.parentElement = null; });
        this.children = [];
        this.append(...children);
    }
    replaceWith(next) {
        const parent = this.parentElement;
        parent.children[parent.children.indexOf(this)] = next;
        next.parentElement = parent;
        this.parentElement = null;
    }
    addEventListener(name, listener) { this.listeners[name] = listener; }
    fire(name) { this.listeners[name]?.(); }
}

function find(root, className) {
    if (root.className === className) return root;
    for (const child of root.children) {
        const match = find(child, className);
        if (match) return match;
    }
}

async function setup({ available = true, height = '820', fetchOk = true } = {}) {
    const root = new Element('main');
    const host = new Element('pax-example');
    host.setAttribute('path', 'transition-grid');
    host.setAttribute('title', 'Transition Grid');
    host.setAttribute('height', height);
    root.append(host);
    const manifest = {
        path: 'transition-grid',
        run_command: 'pax-cli run --path examples/src/transition-grid --target web',
        app: { available, index: 'app/index.html', build_skipped: !available },
        files: [
            { path: 'src/lib.pax', language: 'pax', contents: '<Stacker />' },
            { path: 'src/lib.rs', language: 'rust', contents: 'pub struct Example {}' },
        ],
    };
    const requests = [];
    vm.runInNewContext(readFileSync(new URL('../book/theme/pax-examples.js', import.meta.url), 'utf8'), {
        URL,
        window: {},
        document: {
            currentScript: { src: 'https://docs.example/0.38.3/theme/pax-examples.js' },
            baseURI: 'https://docs.example/0.38.3/animation-motion.html',
            readyState: 'complete',
            createElement: tag => new Element(tag),
            querySelectorAll: () => [host],
        },
        fetch: async url => {
            requests.push(url);
            return { ok: fetchOk, status: 404, json: async () => manifest };
        },
    });
    await new Promise(resolve => setImmediate(resolve));
    return { root, requests };
}

test('app initializes automatically alongside source and standalone link', async () => {
    const { root, requests } = await setup();
    assert.equal(requests.length, 1);
    assert.ok(find(root, 'pax-example-frame'));
    assert.equal(find(root, 'pax-example-button').textContent, 'Restart');
    assert.equal(find(root, 'pax-example-status').textContent, '');
    assert.equal(find(root, 'pax-example-status').hidden, true);
    assert.ok(find(root, 'pax-example-source'));
    assert.equal(find(root, 'pax-example-title').textContent, 'Example: Transition Grid');
    assert.equal(find(root, 'pax-example-command'), undefined);
    const link = find(root, 'pax-example-standalone');
    assert.equal(link.href, 'https://docs.example/0.38.3/_pax_examples/transition-grid/app/index.html');
    assert.equal(link.rel, 'noopener noreferrer');
    assert.equal(link.target, '_blank');
});

test('Restart replaces the automatic iframe and keeps the selected source', async () => {
    const { root } = await setup();
    const tabs = find(root, 'pax-example-tabs');
    tabs.children[1].fire('click');
    const button = find(root, 'pax-example-button');
    const first = find(root, 'pax-example-frame');
    assert.equal(first.title, 'Transition Grid — interactive example');
    assert.equal(first.style.height, '820px');
    first.fire('load');
    assert.equal(find(root, 'pax-example-status').textContent, '');
    button.fire('click');
    const second = find(root, 'pax-example-frame');
    assert.notEqual(first, second);
    assert.equal(first.parentElement, null);
    assert.equal(first.src, second.src);
    assert.equal(find(root, 'pax-example-stage').children.length, 1);
    assert.equal(tabs.children[1].getAttribute('aria-selected'), 'true');
    first.fire('error');
    assert.equal(find(root, 'pax-example-status').hidden, true);
    second.fire('error');
    assert.match(find(root, 'pax-example-status').textContent, /Could not load/);
    assert.equal(find(root, 'pax-example-status').hidden, false);
    button.fire('click');
    assert.equal(find(root, 'pax-example-status').hidden, true);
});

test('invalid height falls back to the default', async () => {
    const { root } = await setup({ height: 'NaN' });
    assert.equal(find(root, 'pax-example-frame').style.height, '520px');
});

test('missing build still shows source without broken app controls', async () => {
    const { root } = await setup({ available: false });
    assert.ok(find(root, 'pax-example-source'));
    assert.ok(find(root, 'pax-example-missing'));
    assert.equal(find(root, 'pax-example-button'), undefined);
});

test('metadata fetch failure leaves an actionable error', async () => {
    const { root } = await setup({ fetchOk: false });
    assert.match(find(root, 'pax-example-error').textContent, /HTTP 404/);
});

test('chapter examples are discoverable before instruction or through an intro link', () => {
    for (const [file, example, nextHeading] of [
        ['event-handling-rust.md', 'space-game', '## Connect an action to Rust'],
        ['routing.md', 'router-playground', '## Routes and history'],
    ]) {
        const article = readFileSync(new URL(`../book/src/${file}`, import.meta.url), 'utf8');
        const embed = `path="${example}"`;
        assert.equal(article.split(embed).length - 1, 1);
        assert.ok(article.indexOf(embed) < article.indexOf(nextHeading));
    }
    const animation = readFileSync(new URL('../book/src/animation-motion.md', import.meta.url), 'utf8');
    assert.ok(animation.indexOf('[Transition Grid](#try-it-transition-grid)') < animation.indexOf('## Timelines'));
    assert.ok(animation.indexOf('path="transition-grid"') > animation.indexOf('## Container-owned motion'));
});
