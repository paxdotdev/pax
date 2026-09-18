import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/utils/route-location.ts', import.meta.url), 'utf8');
function harness(href) {
    const window = {location:new URL(href)};
    const entries = [];
    window.history = Object.fromEntries(['pushState', 'replaceState'].map(method => [method,
        (state, _, url) => { entries.push({method, state, url:String(url)}); window.location = new URL(url); }]));
    const module = {exports:{}};
    vm.runInNewContext(transformSync(source, {loader:'ts',format:'cjs'}).code,
        {module, exports:module.exports, URL, window});
    return {...module.exports, window, entries};
}
const entry = 'https://docs.example/0.39.0/_pax_examples/router-playground/app/index.html';
test('normal routing preserves real paths, repeated query values and fragments', () => {
    const h = harness('https://app.example/teams/a?tag=one&tag=two#member');
    assert.equal(h.browserRouteLocation().href, h.window.location.href);
    const payload = h.serializeRouteLocation(h.browserRouteLocation());
    assert.deepEqual(JSON.parse(JSON.stringify(payload)), {
        path_segments:['teams','a'],query:{tag:['one','two']},fragment:'member',
    });
    h.pushRouteHistoryState(new URL('/notes', h.window.location));
    assert.equal(h.window.location.pathname, '/notes');
});
test('embedded first load starts at its explicit route and hides host query keys', () => {
    const h = harness(entry+'?pax_route=%2F&pax_suspended=1');
    assert.equal(h.browserRouteLocation().pathname, '/');
    assert.equal(h.browserRouteLocation().search, '');
    h.replaceCurrentRouteHistoryState(h.browserRouteLocation());
    assert.equal(h.window.location.pathname, new URL(entry).pathname);
    assert.equal(h.window.location.searchParams.get('pax_suspended'), '1');
});
test('embedded navigation and reload retain the entry path and complete virtual URL', () => {
    const h = harness(entry+'?pax_route=%2F');
    h.pushRouteHistoryState(new URL('/teams/design?tag=one&tag=two#member', h.browserRouteLocation()));
    assert.equal(h.window.location.pathname, new URL(entry).pathname);
    assert.equal(h.entries[0].method, 'pushState');
    const reload = harness(h.window.location.href);
    assert.equal(reload.browserRouteLocation().href, 'https://docs.example/teams/design?tag=one&tag=two#member');
    reload.pushRouteHistoryState(reload.browserRouteLocation());
    assert.equal(reload.entries[0].method, 'replaceState');
});
test('Back and Forward derive the selected route from each browser entry', () => {
    const h = harness(entry+'?pax_route=%2F');
    const first = h.window.location.href;
    h.pushRouteHistoryState(new URL('/guide', h.browserRouteLocation()));
    const second = h.window.location.href;
    h.window.location = new URL(first);
    assert.equal(h.browserRouteLocation().pathname, '/');
    h.window.location = new URL(second);
    assert.equal(h.browserRouteLocation().pathname, '/guide');
});
test('invalid or external embedded routes cannot replace the host origin', () => {
    for (const route of ['https://evil.example/', '//evil.example/', '/\\evil.example/', '/\\[invalid', 'relative']) {
        const h = harness(entry+'?pax_route='+encodeURIComponent(route));
        assert.equal(h.browserRouteLocation().href, 'https://docs.example/');
    }
});
test('bootstrap resolves embedded assets beside the physical entry without rewriting output', () => {
    const html = readFileSync(new URL('../public/index.html', import.meta.url), 'utf8');
    const script = html.match(/<script>([\s\S]*?)<\/script>/)[1];
    for (const query of ['', '?pax_route=%2Fnotes']) {
        const base = {href:'/'};
        const location = new URL(entry+query);
        const document = {querySelector:()=>base, head:{prepend:node=>assert.equal(node,base)}};
        vm.runInNewContext(script, {URL, URLSearchParams, window:{location}, document});
        assert.equal(base.href, query ? new URL('.', entry).href : '/');
    }
});
