# engine::occlusion
<!-- summary: API docs for pax-runtime::engine::occlusion. -->
<!-- tags: api, pax-runtime -->

## Structs
### `OcclusionBox`
Axis-aligned bounds used by the occlusion and native-mask pass.

## Functions
### `update_node_occlusion`
<pre><code class="api-signature language-rust ignore">pub fn update_node_occlusion(root_node: &amp;Rc&lt;ExpandedNode&gt;, ctx: &amp;<a href="/api/internal/pax-runtime/properties.md#runtimecontext">RuntimeContext</a>)</code></pre>

Recompute z-order, native masks, and logical render-layer assignments for the tree.
