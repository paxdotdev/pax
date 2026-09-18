import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

const source = readFileSync(new URL('../src/utils/route-metadata.ts', import.meta.url), 'utf8');
function route(pattern, concrete_path, title, extra = {}) {
    return {pattern, concrete_path, depth:1, order:0, is_default:false,
        metadata:{title, description:title+' description', index:true, social_image:null, social_image_alt:null},
        ...extra};
}
function harness(routes) {
    const nodes = new Map();
    const requests = [];
    const document = {baseURI:'https://docs.example/0.39.0/_pax_examples/demo/app/',title:'',
        head:{querySelector:key=>nodes.get(key), appendChild(node) {
            const key = node.tag === 'link' ? 'link[rel="canonical"]' : `meta[${node.key}="${node.value}"]`;
            nodes.set(key,node); node.remove = () => nodes.delete(key);
        }},
        createElement:tag=>({tag,setAttribute(key,value) { this.key=key; this.value=value; }}),
    };
    const catalog = {version:1, site:{title:'Site',site_name:'Site',site_url:'https://app.example',base_path:'/',
        social_image:'assets/site.png',social_image_alt:'Site image'},routes};
    const module = {exports:{}};
    vm.runInNewContext(transformSync(source,{loader:'ts',format:'cjs'}).code, {
        module, exports:module.exports, URL, document, CSS:{escape:x=>x},
        fetch:async (url) => {requests.push(String(url));return {ok:true,json:async()=>catalog};},
    });
    return {document, requests, update:module.exports.updateDocumentRouteMetadata,
        meta:(name)=>nodes.get(`meta[name="${name}"]`)?.content,
        og:(name)=>nodes.get(`meta[property="${name}"]`)?.content,
        canonical:()=>nodes.get('link[rel="canonical"]')?.href};
}

test('concrete and symbolic routes apply documented indexing and canonical rules', async () => {
    const h = harness([
        route('/', '/', 'Home'),
        route('/notes/*', '/notes', 'Notes'),
        route('/people/:id', null, 'Person'),
        route('/*', null, 'Missing', {is_default:true}),
    ]);
    for (const [path,title,index,canonical] of [
        ['/', 'Home', true, 'https://app.example'],
        ['/notes/?tag=a#part', 'Notes', true, 'https://app.example/notes'],
        ['/notes/deeper', 'Notes', false, undefined],
        ['/people/ada', 'Person', false, undefined],
        ['/unknown', 'Missing', false, undefined],
    ]) {
        await h.update(new URL(path,'https://docs.example'));
        assert.equal(h.document.title,title);
        assert.equal(h.meta('description'),title+' description');
        assert.equal(h.meta('robots'),index?'index,follow':'noindex,follow');
        assert.equal(h.canonical(),canonical);
        assert.equal(h.og('og:url'),canonical);
    }
    assert.deepEqual(h.requests,['https://docs.example/0.39.0/_pax_examples/demo/app/route-metadata.json']);
});

test('deeper explicit route wins over its catch-all and nested default', async () => {
    const h = harness([
        route('/notes/*','/notes','Parent'),
        route('/notes/*','/notes','Missing',{depth:2,is_default:true}),
        route('/notes/about','/notes/about','About',{depth:2,order:2}),
    ]);
    await h.update(new URL('https://app.example/notes/about'));
    assert.equal(h.document.title,'About');
    assert.equal(h.canonical(),'https://app.example/notes/about');
});

test('navigation replaces social overrides and clears stale tags when unmatched', async () => {
    const special = route('/special','/special','Special');
    special.metadata.social_image = 'assets/special.png';
    special.metadata.social_image_alt = 'Special image';
    const h = harness([special,route('/','/','Home')]);
    await h.update(new URL('https://app.example/special'));
    assert.equal(h.og('og:image'),'https://app.example/assets/special.png');
    assert.equal(h.og('og:image:alt'),'Special image');
    await h.update(new URL('https://app.example/'));
    assert.equal(h.og('og:image'),'https://app.example/assets/site.png');
    await h.update(new URL('https://app.example/unknown'));
    assert.equal(h.document.title,'Site');
    assert.equal(h.meta('robots'),'noindex,follow');
    assert.equal(h.og('og:image'),undefined);
    assert.equal(h.meta('description'),undefined);
    assert.equal(h.canonical(),undefined);
});
