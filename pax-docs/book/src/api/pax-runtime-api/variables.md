# variables
<!-- summary: Property adapters that expose runtime values to expression scopes. -->
<!-- tags: api, pax-runtime-api -->

Property adapters that expose runtime values to expression scopes.

## Structs
### `Variable`
Runtime property adapter used by expression scopes.

#### Implementations
##### `get_as_pax_value`
<pre><code class="api-signature language-rust ignore">pub fn get_as_pax_value(&amp;self) -&gt; <a href="../../api/pax-runtime-api/pax_value.md#paxvalue">PaxValue</a></code></pre>

Reads the current value as a `PaxValue`.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new&lt;T: <a href="../../api/pax-runtime-api/properties.md#propertyvalue">PropertyValue</a> + <a href="../../api/pax-runtime-api/pax_value.md#topaxvalue">ToPaxValue</a>&gt;(untyped_property: UntypedProperty) -&gt; Self</code></pre>

Wraps an untyped property and exposes it as a `PaxValue`.

##### `new_from_typed_property`
<pre><code class="api-signature language-rust ignore">pub fn new_from_typed_property&lt;T: <a href="../../api/pax-runtime-api/properties.md#propertyvalue">PropertyValue</a> + <a href="../../api/pax-runtime-api/pax_value.md#topaxvalue">ToPaxValue</a>&gt;(property: <a href="../../api/pax-runtime-api/properties.md#property">Property</a>&lt;T&gt;) -&gt; Self</code></pre>

Wraps a typed property and exposes it as a `PaxValue`.

##### `read_pax_value_ref`
<pre><code class="api-signature language-rust ignore">pub fn read_pax_value_ref&lt;V&gt;(&amp;self, f: impl FnOnce(&amp;<a href="../../api/pax-runtime-api/pax_value.md#paxvalue">PaxValue</a>) -&gt; V) -&gt; V</code></pre>

Reads the current `PaxValue` by reference.

##### `try_typed_binding`
<pre><code class="api-signature language-rust ignore">pub fn try_typed_binding&lt;T: <a href="../../api/pax-runtime-api/properties.md#propertyvalue">PropertyValue</a> + CoercionRules&gt;(&amp;self, name: &amp;str) -&gt; Option&lt;<a href="../../api/pax-runtime-api/properties.md#property">Property</a>&lt;T&gt;&gt;</code></pre>

Creates an independent one-way binding when exact typed forwarding is
safe. Unlike a double binding, writes/easing on the result never mutate
the source. Mismatched types and custom conversions return `None`.
