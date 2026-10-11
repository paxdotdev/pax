# engine::occlusion
<!-- summary: API docs for pax-runtime::engine::occlusion. -->
<!-- tags: api, pax-runtime -->

## Structs
### `OcclusionBox`
Axis-aligned bounds used by the occlusion and native-mask pass.

---

### `OcclusionStats`
Cumulative native-compositing work, independent of canvas replay counters.

#### Properties
##### `passes`
Type: `u64`

Reconciliation passes, including incremental passes.

##### `rebuilds`
Type: `u64`

Full structural/presentation reconciliations.

##### `records_updated`
Type: `u64`

Dirty records refreshed outside structural reconciliation.

##### `masks_evaluated`
Type: `u64`

Native masks evaluated by the incremental path.

##### `mask_candidates`
Type: `u64`

Canvas candidates examined for those masks.

##### `rebuild_reason`
Type: `Option`<&'`static` `str`>

Cause of the most recent full reconciliation, for opt-in profiling.

##### `rebuild_node`
Type: `Option`<`u32`>

Triggering expanded node, when the cause was not structural.

##### `rebuild_origins`
Type: [`Option`<`OcclusionInvalidation`>; 8]

Bounded origins of structural invalidation consumed by the last rebuild.
Populated only when diagnostic attribution is enabled.

## Functions
### `update_node_occlusion`
<pre><code class="api-signature language-rust ignore">pub fn update_node_occlusion(root_node: &amp;Rc&lt;ExpandedNode&gt;, ctx: &amp;<a href="../../../../api/internal/pax-runtime/properties.md#runtimecontext">RuntimeContext</a>)</code></pre>

Reconcile structural changes, otherwise update only dirty geometry and overlapping masks.
