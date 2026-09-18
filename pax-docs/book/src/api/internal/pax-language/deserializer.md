# deserializer
<!-- summary: API docs for pax-language::deserializer. -->
<!-- tags: api, pax-language -->

## Submodules
- [deserializer::error](deserializer/error.md)

## Structs
### `PaxDeserializer`
Serde bridge from Pax literal grammar nodes into runtime values.

#### Properties
##### `ast`
Type: `Pair`<'`de`, [`Rule`](../../../api/internal/pax-language/index.md#rule)>

#### Implementations
##### `from`
<pre><code class="api-signature language-rust ignore">pub fn from(ast: Pair&lt;&#39;de, <a href="../../../api/internal/pax-language/index.md#rule">Rule</a>&gt;) -&gt; Self</code></pre>

Wrap a pest pair as a deserializer.

## Functions
### `from_pax`
<pre><code class="api-signature language-rust ignore">pub fn from_pax(str: &amp;str) -&gt; Result&lt;<a href="../../../api/pax-runtime-api/pax_value.md#paxvalue">PaxValue</a>&gt;</code></pre>

Deserialize a Pax literal string into a runtime `PaxValue`.

---

### `from_pax_ast`
<pre><code class="api-signature language-rust ignore">pub fn from_pax_ast(ast: Pair&lt;&#39;_, <a href="../../../api/internal/pax-language/index.md#rule">Rule</a>&gt;) -&gt; Result&lt;<a href="../../../api/pax-runtime-api/pax_value.md#paxvalue">PaxValue</a>&gt;</code></pre>

Deserialize a parsed literal AST node into a runtime `PaxValue`.
