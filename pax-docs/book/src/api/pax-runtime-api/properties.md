# properties
<!-- summary: API docs for pax-runtime-api::properties. -->
<!-- tags: api, pax-runtime-api -->

## Structs
### `Property`
A reactive value node in Pax's property graph.

`Property<T>` is the primary state and binding primitive used by generated
components, PAXEL expressions, and Rust component logic.

#### Implementations
##### `computed`
<pre><code class="api-signature language-rust ignore">pub fn computed(evaluator: impl Fn() -&gt; T + &#39;static, dependents: &amp;[UntypedProperty]) -&gt; Self</code></pre>

Creates a computed property from an evaluator and dependency list.

##### `computed_with_name`
<pre><code class="api-signature language-rust ignore">pub fn computed_with_name(evaluator: impl Fn() -&gt; T + &#39;static, dependents: &amp;[UntypedProperty], name: &amp;str) -&gt; Self</code></pre>

Creates a named computed property, useful for diagnostics.

##### `ease_to`
<pre><code class="api-signature language-rust ignore">pub fn ease_to(&amp;self, end_val: T, time: u64, curve: <a href="/api/pax-runtime-api/animation.md#easingcurve">EasingCurve</a>)</code></pre>

Immediately starts an ease transition from the current value to end_val, over time frames, following curve.

##### `ease_to_later`
<pre><code class="api-signature language-rust ignore">pub fn ease_to_later(&amp;self, end_val: T, time: u64, curve: <a href="/api/pax-runtime-api/animation.md#easingcurve">EasingCurve</a>)</code></pre>

Enqueues an ease transition from the current value to end_val, over time frames, following
curve, which will start after all currently enqueued transitions finish.

##### `get`
<pre><code class="api-signature language-rust ignore">pub fn get(&amp;self) -&gt; T</code></pre>

Gets the currently stored value. Might be computationally
expensive in a large reactivity network since this triggers
re-evaluation of dirty property chains

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(val: T) -&gt; Self</code></pre>

Creates a literal property with an initial value.

##### `new_with_name`
<pre><code class="api-signature language-rust ignore">pub fn new_with_name(val: T, name: &amp;str) -&gt; Self</code></pre>

Creates a named literal property, useful for diagnostics.

##### `read`
<pre><code class="api-signature language-rust ignore">pub fn read&lt;V&gt;(&amp;self, f: impl FnOnce(&amp;T) -&gt; V) -&gt; V</code></pre>

Reads the inner value by reference.

Panics if this property is already borrowed, which can happen if `read`
is called inside a read of the same property.

##### `replace_with`
<pre><code class="api-signature language-rust ignore">pub fn replace_with(&amp;self, target: <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;T&gt;)</code></pre>

Replaces this property's evaluator, dependencies, and value with `target`, while keeping dependents.

This can introduce circular dependencies if used carelessly. It is
intended for changing a property from literal to computed (or vice
versa) without severing existing outbound links.

##### `set`
<pre><code class="api-signature language-rust ignore">pub fn set(&amp;self, val: T)</code></pre>

Sets this properties value and sets the dirty bit recursively of all of
its dependencies if not already set

##### `untyped`
<pre><code class="api-signature language-rust ignore">pub fn untyped(&amp;self) -&gt; UntypedProperty</code></pre>

Casts this property to its untyped version.

##### `update`
<pre><code class="api-signature language-rust ignore">pub fn update(&amp;self, f: impl FnOnce(&amp;mut T))</code></pre>

Get access to a mutable reference to the inner value T.
Will trigger updates for dependents of this property, regardless
of if the value actually changed

## Traits
### `PropertyValue`
Bound for values that can live inside Pax `Property<T>`.

Values must be cloneable for `.get()`, interpolatable for transitions, and
`'static` because properties are stored in the runtime graph.
