# properties
<!-- summary: API docs for pax-runtime-api::properties. -->
<!-- tags: api, pax-runtime-api -->

## Structs
### `Property`
A reactive value node in Pax's property graph.

`Property<T>` is the primary state and binding primitive used by generated
components, PAXEL expressions, and Rust component logic.

#### Implementations
##### `cancel_transitions`
<pre><code class="api-signature language-rust ignore">pub fn cancel_transitions(&amp;self)</code></pre>

Stops the active transition and clears every queued transition segment.

The property is left at its current eased value. A subsequent [`Property::set`]
can therefore take immediate ownership without the cancelled transition
overwriting it on the next runtime tick.

##### `computed`
<pre><code class="api-signature language-rust ignore">pub fn computed(evaluator: impl Fn() -&gt; T + &#39;static, dependents: &amp;[UntypedProperty]) -&gt; Self</code></pre>

Creates a computed property from an evaluator and dependency list.

##### `computed_with_cutoff`
<pre><code class="api-signature language-rust ignore">pub fn computed_with_cutoff(evaluator: impl Fn() -&gt; T + &#39;static, dependents: &amp;[UntypedProperty], cutoff: impl Fn(&amp;T, &amp;T) -&gt; bool + &#39;static) -&gt; Self</code></pre>

Creates a computed property with a propagation cutoff.

The predicate receives the last accepted value and the newly evaluated
candidate. Returning `true` discards the candidate and stops outbound
invalidation at this property; returning `false` accepts and propagates
it. The first evaluation is always accepted.

##### `computed_with_cutoff_and_name`
<pre><code class="api-signature language-rust ignore">pub fn computed_with_cutoff_and_name(evaluator: impl Fn() -&gt; T + &#39;static, dependents: &amp;[UntypedProperty], cutoff: impl Fn(&amp;T, &amp;T) -&gt; bool + &#39;static, name: &amp;str) -&gt; Self</code></pre>

Creates a named cutoff computed property, useful for diagnostics.

##### `computed_with_name`
<pre><code class="api-signature language-rust ignore">pub fn computed_with_name(evaluator: impl Fn() -&gt; T + &#39;static, dependents: &amp;[UntypedProperty], name: &amp;str) -&gt; Self</code></pre>

Creates a named computed property, useful for diagnostics.

##### `ease_to`
<pre><code class="api-signature language-rust ignore">pub fn ease_to&lt;D: Into&lt;<a href="../../api/pax-runtime-api/animation.md#duration">Duration</a>&gt;&gt;(&amp;self, end_val: T, duration: D, curve: <a href="../../api/pax-runtime-api/animation.md#easingcurve">EasingCurve</a>)</code></pre>

Immediately starts an ease transition from the current value to end_val, over a duration, following curve.

Numeric arguments preserve the historical frame-based behavior. Use
`Duration::Milliseconds`, `Duration::Seconds`, or `Duration::Frames` to
select an explicit unit.

##### `ease_to_later`
<pre><code class="api-signature language-rust ignore">pub fn ease_to_later&lt;D: Into&lt;<a href="../../api/pax-runtime-api/animation.md#duration">Duration</a>&gt;&gt;(&amp;self, end_val: T, duration: D, curve: <a href="../../api/pax-runtime-api/animation.md#easingcurve">EasingCurve</a>)</code></pre>

Enqueues an ease transition from the current value to end_val, over a duration, following
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
<pre><code class="api-signature language-rust ignore">pub fn replace_with(&amp;self, target: <a href="../../api/pax-runtime-api/properties.md#property">Property</a>&lt;T&gt;)</code></pre>

Replaces this property's evaluator, dependencies, and value with `target`, while keeping dependents.

This can introduce circular dependencies if used carelessly. It is
intended for changing a property from literal to computed (or vice
versa) without severing existing outbound links.

##### `set`
<pre><code class="api-signature language-rust ignore">pub fn set(&amp;self, val: T)</code></pre>

Sets this properties value and sets the dirty bit recursively of all of
its dependencies if not already set

##### `set_if_neq`
<pre><code class="api-signature language-rust ignore">pub fn set_if_neq(&amp;self, val: T) -&gt; bool where T: PartialEq</code></pre>

Sets the value only when it differs from the current one.

Returns `true` when the write changed the property and dirtied dependents.

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
