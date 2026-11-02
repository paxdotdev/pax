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

##### `structural_reconciliations`
Type: `u64`

Structural edits reconciled without replacing stable coverage records.

##### `records_retained`
Type: `u64`

Records reused by those structural reconciliations.

##### `topology_nodes_visited`
Type: `u64`

Expanded nodes inspected while reconciling structural edits.

##### `topology_nodes_reused`
Type: `u64`

Nodes copied from unchanged retained subtrees without inspecting components.

##### `orders_updated`
Type: `u64`

Numeric paint-order changes sent without invalidating geometry.

##### `records_updated`
Type: `u64`

Dirty/new records refreshed by localized coverage reconciliation.

##### `masks_evaluated`
Type: `u64`

Native masks evaluated after structural or incremental reconciliation.

##### `mask_candidates`
Type: `u64`

Canvas candidates examined for those masks.

##### `mask_plan_reuses`
Type: `u64`

Unchanged dependency plans reused before path mapping or serialization.

##### `mask_path_mappings`
Type: `u64`

Mask contributions mapped after a geometry/chart dependency changed.

##### `mask_path_serializations`
Type: `u64`

SVG paths serialized, excluding wire-message cloning.

##### `scroll_lineage_preparations`
Type: `u64`

Scroller lineage compositions after a dependency change.

##### `clip_geometry_preparations`
Type: `u64`

Local/source clip shapes prepared after invalidation.

##### `projection_preparations`
Type: `u64`

Camera projections rebuilt after pose/view/scroll inputs changed.

##### `solid_relation_evaluations`
Type: `u64`

Exact solid/support front-coverage evaluations (zero without `3d`).

##### `solid_relation_reuses`
Type: `u64`

Shared or unchanged solid/support coverage reused.

##### `visibility_cache_entries`
Type: `usize`

Resident auxiliary relation/plan entries across all presentation domains.

##### `visibility_cache_bytes`
Type: `usize`

Conservatively charged bytes of retained derived coverage and mask plans.

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
