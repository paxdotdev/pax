# engine::piet_render_context
<!-- summary: API docs for pax-runtime::engine::piet_render_context. -->
<!-- tags: api, pax-runtime -->

## Structs
### `PietRenderer`
Legacy/test `RenderContext` implementation backed by piet.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(layer_factory: impl Fn(usize) -&gt; (R, Box&lt;dyn Fn()&gt;, Box&lt;dyn Fn()&gt;) + &#39;static) -&gt; Self</code></pre>

Create a piet renderer with a chassis-provided layer factory.
