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

### `PreparedCanvasGeometry`
Geometry shared by render selection, primitive drawing, and viewport observation.
Native surfaces use a separate bounds lookup in the same geometry service.
Observed layout-only targets and their ancestors share these records without
entering either rendering index.

Coverage is in the owning canvas's content coordinates, so native scrolling
changes the queried region without invalidating every descendant record.

#### Properties
##### `world_transform`
Type: `Affine`

Layout transform in unscrolled scene coordinates, shared with viewport observation.

##### `layer`
Type: `usize`

Owning logical canvas layer.

##### `bounds`
Type: (`f64`, `f64`)

Layout dimensions, kept separate from overflowing paint coverage.

##### `surface_transform`
Type: `Affine`

Transform from primitive-local space to canvas content space.

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
