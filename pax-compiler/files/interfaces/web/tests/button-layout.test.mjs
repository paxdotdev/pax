import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

test('native buttons do not participate in an inline baseline inside their positioned leaf', () => {
    const css = readFileSync(new URL('../src/styles/pax-web.css', import.meta.url), 'utf8');
    const rule = css.match(/\.pax-mount \.button-styles\s*\{([^}]+)\}/)?.[1];
    assert.ok(rule, 'native button rule exists');
    assert.match(rule, /display:\s*block\s*;/);
    assert.match(rule, /margin:\s*0\s*;/);
    assert.match(rule, /padding:\s*0\s*;/);
});
