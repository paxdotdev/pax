# properties
<!-- summary: API docs for pax-runtime-api::properties. -->
<!-- tags: api, pax-runtime-api -->

## Traits
### `PropertyBinding`
Owner-thread binding operations used by generated component factories.

Shared fields publish values while their evaluators stay in the entered
[`PropertyGraph`]. Local fields retain ordinary lazy graph semantics.

---

### `PropertyValue`
Value operations shared by local and transferable properties.

Values must be cloneable for `.get()`, interpolatable for transitions, and
`'static` because properties are stored in the runtime graph.

## Functions
### `register_millis`
<pre><code class="api-signature language-rust ignore">pub fn register_millis(prop: &amp;LocalProperty&lt;u64&gt;)</code></pre>

Registers the millisecond clock for the entered application graph.
Without an entered graph, registers the standalone fallback clock.

---

### `register_time`
<pre><code class="api-signature language-rust ignore">pub fn register_time(prop: &amp;LocalProperty&lt;u64&gt;)</code></pre>

Registers the frame clock for the entered application graph.
Without an entered graph, registers the standalone fallback clock.
