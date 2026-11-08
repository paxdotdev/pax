import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

const source = await readFile(new URL('./static/pax-header.js', import.meta.url), 'utf8');
const {navigationDestination} = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

test('embedded links use document navigation with normal URLs and targets', () => {
    const base = 'https://www.pax.dev/blog/a-place-for-notes/';
    assert.deepEqual(navigationDestination({url: '/', target: 'current'}, base), {url: 'https://www.pax.dev/', newTab: false});
    assert.deepEqual(navigationDestination({url: '/blog/?tag=Notes#posts', target: 'current'}, base), {url: 'https://www.pax.dev/blog/?tag=Notes#posts', newTab: false});
    assert.deepEqual(navigationDestination({url: 'https://docs.pax.dev/', target: 'new'}, base), {url: 'https://docs.pax.dev/', newTab: true});
});

test('unsupported protocols and malformed navigation do not execute', () => {
    for (const detail of [null, {}, {url: 'javascript:alert(1)'}, {url: 'data:text/html,hello'}, {url: 'http://['}]) {
        assert.equal(navigationDestination(detail, 'https://www.pax.dev/'), null);
    }
});

test('the embedded header neither owns the page nor loads the root website cartridge', () => {
    assert.match(source, /pageScroll: false/);
    assert.match(source, /documentRouting: false/);
    assert.match(source, /_pax\/header\/pax-cartridge/);
    assert.match(source, /if \(!event.persisted\) close\(\)/);
});
