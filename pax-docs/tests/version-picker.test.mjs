import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

async function picker(manifest, url = 'https://docs.example/0.39.0/routing.html?view=full#metadata') {
    const nodes = [];
    const requests = [];
    const window = { location: new URL(url) };
    const rightButtons = { insertBefore(node) { nodes.push(node); } };
    const document = {
        currentScript: { src: 'https://docs.example/0.39.0/theme/pax-version.js' },
        readyState: 'complete',
        querySelector(selector) { return selector.includes('right-buttons') ? rightButtons : null; },
        createElement(tag) {
            return { tag, children: [], listeners: {},
                setAttribute() {},
                appendChild(child) { this.children.push(child); },
                addEventListener(event, handler) { this.listeners[event] = handler; },
            };
        },
    };
    vm.runInNewContext(readFileSync(new URL('../book/theme/pax-version.js', import.meta.url), 'utf8'), {
        URL, window, document,
        fetch: async (url, options) => {
            requests.push({ url, options });
            return { ok: true, json: async () => manifest };
        },
    });
    await new Promise(resolve => setImmediate(resolve));
    const select = nodes[0].children.find(node => node.tag === 'select');
    return { select, window, requests };
}

test('explicit latest wins over a newer non-promoted version', async () => {
    const { select, requests } = await picker({ latest: '0.38.3', versions: ['0.39.0', '0.38.3'] });
    assert.equal(select.children[0].textContent, 'Latest (0.38.3)');
    assert.equal(select.value, '0.39.0');
    assert.equal(requests[0].url, 'https://docs.example/versions.json');
    assert.equal(requests[0].options.cache, 'no-cache');
});

test('a catalog with no promoted latest does not label root as its newest version', async () => {
    for (const manifest of [{ latest: null, versions: ['0.39.0'] }, { versions: ['0.39.0'] }]) {
        const { select } = await picker(manifest);
        assert.equal(select.children[0].textContent, 'Latest');
    }
});

test('switching between root and a release preserves the article, query, and anchor', async () => {
    const manifest = { latest: '0.39.0', versions: ['0.39.0', '0.38.3'] };
    const { select, window } = await picker(manifest);
    select.value = 'latest';
    select.listeners.change();
    assert.equal(window.location.href, 'https://docs.example/routing.html?view=full#metadata');
    select.value = '0.38.3';
    select.listeners.change();
    assert.equal(window.location.href, 'https://docs.example/0.38.3/routing.html?view=full#metadata');
});

test('an empty development catalog stays disabled', async () => {
    const { select } = await picker({ latest: null, versions: [] }, 'http://localhost/getting-started.html');
    assert.equal(select.disabled, true);
    assert.equal(select.children[0].textContent, 'dev');
});

test('prerelease plus build metadata is recognized as a version prefix', async () => {
    const version = '0.39.0-rc.1+build.7';
    const { select, window } = await picker({ latest: '0.38.3', versions: [version, '0.38.3'] },
        `https://docs.example/${version}/api/index.html#public-crates`);
    assert.equal(select.value, version);
    select.value = 'latest';
    select.listeners.change();
    assert.equal(window.location.href, 'https://docs.example/api/index.html#public-crates');
});
