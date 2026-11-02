# scene_geometry
<!-- summary: Runtime-owned geometry preparation and spatial lookup, independent of canvas residency. -->
<!-- tags: api, pax-runtime -->

Runtime-owned geometry preparation and spatial lookup, independent of canvas residency.

## Structs
### `CanvasGeometry`
Primitive-local geometry prepared before drawing or spatial queries.

An absent bound is conservatively unbounded. Primitives that draw outside their
layout rectangle must supply their paint bounds. Paths can retain their prepared
command geometry here so selection and drawing do not construct it twice.

#### Properties
##### `local_bounds`
Type: `Option`<`Rect`>

Conservative local paint bounds; `None` requires inclusion in every region query.

##### `path`
Type: `Option`<`Rc`<`BezPath`>>

Optional prepared path reused by the primitive during drawing.

#### Implementations
##### `for_layout`
<pre><code class="api-signature language-rust ignore">pub fn for_layout(bounds: (f64, f64)) -&gt; Self</code></pre>

Use the layout rectangle for a bounded primitive.

---

### `InputQueryStats`
Cumulative input-query work; exact predicates remain distinct from broad-phase selection.

#### Properties
##### `topology_rebuilds`
Type: `u64`

Topology reconciliations, including a change of queried subtree root.

##### `bounds_prepared`
Type: `u64`

Conservative input footprints prepared after invalidation.

##### `indexed_queries`
Type: `u64`

Queries visiting only candidates and their necessary ancestors.

##### `pending_queries`
Type: `u64`

Uncached preparation while reactive or compositing work was pending.

##### `startup_queries`
Type: `u64`

Uncached preparation during incremental startup.

##### `candidates`
Type: `u64`

Broad-phase candidates, including unknown conservative bounds.

##### `exact_tests`
Type: `u64`

Nodes reaching exact hit evaluation, including clipping ancestors.

##### `unknown_tests`
Type: `u64`

Exact tests for shapes without a finite conservative screen bound.

---

### `PreparedCanvasGeometry`
Geometry shared by render selection, primitive drawing, and viewport observation.
Native surfaces use a separate bounds lookup in the same geometry service.
Observed layout-only targets and their ancestors share these records without
entering either rendering index.

Coverage is in the owning canvas's content coordinates. Affine native scrolling
changes only the queried region. Perspective scrolling also updates projected
records because surfaces at different depths move by different screen distances.

#### Properties
##### `world_transform`
Type: `Affine`

Camera-projected transform before native scroll offsets, shared with viewport observation.

##### `spatial_geometry`
Type: [`SpatialGeometry`](../../../api/internal/pax-runtime/spatial.md#spatialgeometry)

Camera-projected geometry used to inspect depth and geometric coplanarity.
Kept separate from canvas ownership and logical opacity scopes.

##### `solid`
Type: `Option`<`Rc`<[`PosedConvexMesh`](../../../api/pax-runtime-api/mesh3.md#posedconvexmesh)>>

Retained convex volume, in unscrolled world space.

##### `projected_solid`
Type: `Option`<`Rc`<[`ProjectedConvexMesh`](../../../api/pax-runtime-api/mesh3/projection.md#projectedconvexmesh)>>

Camera-clipped volume in window coordinates. Orthographic scrolling is
applied by consumers; perspective scrolling is already included.

##### `solid_projection`
Type: `Option`<[`MeshProjection`](../../../api/pax-runtime-api/mesh3/projection.md#meshprojection)>

Mesh-local projection into the owning canvas, preserving camera clip planes.

##### `layer`
Type: `usize`

Owning logical canvas layer.

##### `bounds`
Type: (`f64`, `f64`)

Layout dimensions, kept separate from overflowing paint coverage.

##### `surface_transform`
Type: `Affine`

Transform from primitive-local space to canvas content space.

##### `projection`
Type: `Option`<[`ProjectedPlane`](../../../api/pax-runtime-api/math/projection.md#projectedplane)>

Projective local-to-surface mapping, including camera clipping.

##### `world_projection`
Type: `Option`<[`ProjectedPlane`](../../../api/pax-runtime-api/math/projection.md#projectedplane)>

Projective mapping into window coordinates, including ancestor scrolling.

##### `coverage_bounds`
Type: `Option`<`Rect`>

Transformed paint coverage including the renderer's conservative culling pad.

##### `local`
Type: [`CanvasGeometry`](../../../api/internal/pax-runtime/scene_geometry.md#canvasgeometry)

Primitive-local geometry backing this prepared record.

---

### `SceneGeometryStats`
Cumulative work counters for verifying geometry reuse and spatial-query scaling.

#### Properties
##### `preparations`
Type: `u64`

Primitive geometry preparations, excluding cache hits.

##### `index_updates`
Type: `u64`

Insertions or changes to indexed coverage/layer ownership.

##### `queries`
Type: `u64`

Region-query requests.

##### `candidate_tests`
Type: `u64`

Distinct candidates reaching final rectangle tests, accumulated across queries.
