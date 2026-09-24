import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const context = vm.createContext({});
vm.runInContext(readFileSync(new URL('./blog-paths.js',import.meta.url),'utf8'),context);
test('blog paths resolve without changing query parameters, origin selection, or unrelated paths', () => {
    for (const [input,expected] of [['/blog','/index.html'], ['/blog/','/index.html'],
        ['/blog/pax-0-39-0','/pax-0-39-0/index.html'], ['/blog/pax-0-39-0/','/pax-0-39-0/index.html'],
        ['/blog/atom.xml','/atom.xml'], ['/blog/main.css','/main.css'],
        ['/blog/images/a.b/picture.png','/images/a.b/picture.png'], ['/blogger','/blogger'], ['/','/']]) {
        const querystring = {h:{value:'a%20b'},tag:{multiValue:[{value:'one'},{value:'two'}]}};
        const request = {uri:input,querystring,method:'GET',headers:{}};
        const result = context.handler({request});
        assert.equal(result,request);
        assert.equal(result.uri,expected);
        assert.equal(result.querystring,querystring);
    }
});
