import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const window = {};
vm.runInNewContext(readFileSync(new URL('../book/theme/highlight-pax.js', import.meta.url), 'utf8'), {
    window,
    document: { readyState: 'complete', querySelectorAll: () => [] },
});

test('contextual references remain distinct from directives and event bindings', () => {
    const html = window.PaxHighlight.highlightPax(`
if @gpu { <Rectangle width={@viewport.width} @click=self.click
    fill=@gradient { 0%: RED 100%: BLUE }/> }
@settings { @mount: self.mount if @cpu == true { .card { width: {@base} } } }
@timeline reveal { playhead: {@millis} }
`);
    for (const name of ['gpu', 'viewport', 'cpu', 'base', 'millis']) {
        assert.ok(html.includes(`<span class="hljs-built_in">@${name}</span>`), name);
    }
    for (const name of ['settings', 'click', 'mount', 'gradient', 'timeline']) {
        assert.ok(html.includes(`<span class="hljs-meta">@${name}</span>`), name);
    }
    assert.ok(html.includes('<span class="hljs-title">reveal</span>'));
});

test('helper calls and literal sigils keep their existing highlighting', () => {
    const html = window.PaxHighlight.highlightPax('<Text text={Notes::label(@frames)} id="email@example.com $frames"/>');
    assert.ok(html.includes('<span class="hljs-symbol">Notes</span><span class="hljs-operator">::</span><span class="hljs-symbol">label</span>'));
    assert.ok(html.includes('<span class="hljs-string">&quot;email@example.com $frames&quot;</span>'));
});
