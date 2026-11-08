import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, readFile, writeFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {runInNewContext} from 'node:vm';
import {minifyHeaderBundle} from './minify-header.mjs';

test('minification preserves classic-script Pax, ESM exports, and legal notices', async t => {
    const dir = await mkdtemp(join(tmpdir(), 'pax-header-minify-'));
    t.after(() => rm(dir, {recursive: true, force: true}));
    await writeFile(join(dir, 'pax-interface-web.js'), `
        /*! Header fixture license */
        var Pax = (() => {
            const unusedExplanation = 'not public';
            return {init_web: async function (options) { return options.element; }};
        })();
    `);
    await writeFile(join(dir, 'pax-cartridge.js'), `
        export async function __pax_init_web_fixture(host) { return host; }
        export { __pax_init_web_fixture as init_web };
        export default function initialize() { return new URL('pax-cartridge_bg.wasm', import.meta.url); }
    `);
    const wasm = Buffer.from([0, 97, 115, 109]);
    await writeFile(join(dir, 'pax-cartridge_bg.wasm'), wasm);
    await minifyHeaderBundle(dir);
    const interfaceSource = await readFile(join(dir, 'pax-interface-web.js'), 'utf8');
    assert.match(interfaceSource, /Header fixture license/);
    const context = {};
    runInNewContext(interfaceSource, context);
    assert.equal(await context.Pax.init_web({element: 'mount'}), 'mount');
    const cartridge = await readFile(join(dir, 'pax-cartridge.js'), 'utf8');
    const module = await import(`data:text/javascript;base64,${Buffer.from(cartridge).toString('base64')}`);
    assert.equal(await module.init_web('mount'), 'mount');
    assert.equal(typeof module.default, 'function');
    assert.match(cartridge, /pax-cartridge_bg\.wasm/);
    assert.deepEqual(await readFile(join(dir, 'pax-cartridge_bg.wasm')), wasm);
});
