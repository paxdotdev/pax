import test from 'node:test';
import assert from 'node:assert/strict';
import {buildSync} from 'esbuild';
import vm from 'node:vm';

const code = buildSync({entryPoints:[new URL('../src/utils/navigation.ts', import.meta.url).pathname],
    bundle:true, write:false, platform:'browser', format:'cjs'}).outputFiles[0].text;
function harness(href = 'https://app.example/settings', prefixes = ['/blog']) {
    const requests = [], history = [], routes = [], opened = [], assigned = [];
    const location = new URL(href);
    location.assign = url => assigned.push(url);
    const window = {location, open:(...args)=>opened.push(args), history:Object.fromEntries(
        ['pushState','replaceState'].map(method=>[method, (...args)=>history.push({method,args})]))};
    const document = {baseURI:href, getElementById:id=>prefixes === null ? null :
        {textContent:JSON.stringify({server_owned_prefixes:prefixes})}};
    const module = {exports:{}};
    vm.runInNewContext(code, {module, exports:module.exports, URL, window, document, console,
        fetch:async url=>{requests.push(String(url));return {ok:false};}});
    return {window, requests, history, routes, opened, assigned,
        navigate:(url,target='current')=>module.exports.navigateBrowser(url,target,url=>routes.push(url.href))};
}

test('server-owned navigation loads the resolved document without app history, interrupts, or metadata requests', () => {
    for (const destination of ['/blog', '/blog/', '/blog/post?tag=a&tag=b#examples',
        'https://app.example/blog/post', '/%62log/post', '/other/../blog/post']) {
        const h = harness();
        h.navigate(destination);
        assert.deepEqual(h.assigned, [new URL(destination,h.window.location).href]);
        assert.deepEqual([h.history,h.routes,h.requests,h.opened], [[],[],[],[]]);
    }
});
test('ordinary routes and absent configuration preserve Pax routing with complete path boundaries', () => {
    for (const [destination,prefixes] of [['/blogger',['/blog']], ['/Blog',['/blog']],
        ['/blog%2Fpost',['/blog']], ['/settings',['/blog']], ['/blog',[]], ['/blog',null]]) {
        const h = harness(undefined,prefixes);
        h.navigate(destination);
        assert.equal(h.history.length,1);
        assert.equal(h.routes.length,1);
        assert.equal(h.requests.length,1);
        assert.deepEqual(h.assigned,[]);
    }
});
test('cross-origin and new-tab links retain browser target behavior', () => {
    const h = harness();
    h.navigate('https://other.example/blog');
    h.navigate('/blog','new');
    assert.deepEqual(h.opened,[['https://other.example/blog','_self'],['/blog','_blank']]);
    assert.deepEqual([h.history,h.routes,h.assigned],[[],[],[]]);
});
test('relative links resolve against the virtual app URL and leave query-backed embeds in the current frame', () => {
    const h = harness('https://app.example/demos/demo/index.html?pax_route=%2Fsettings%2F');
    h.navigate('../blog/post?q=one#part');
    assert.deepEqual(h.assigned,['https://app.example/blog/post?q=one#part']);
    assert.deepEqual(h.history,[]);
});
test('root prefix delegates all same-origin current-tab paths', () => {
    const h = harness(undefined,['/']);
    h.navigate('/anything');
    assert.deepEqual(h.assigned,['https://app.example/anything']);
});
test('encoded UTF-8 path spellings match normalized configuration', () => {
    const h = harness(undefined,['/caf%C3%A9']);
    h.navigate('/caf%c3%a9/post');
    assert.equal(h.assigned.length,1);
});
