# mesh3
<!-- summary: Retained convex surface geometry for the opt-in spatial renderer. -->
<!-- tags: api, pax-runtime-api -->

Retained convex surface geometry for the opt-in spatial renderer.

Mesh topology is immutable. Poses retain projected coverage independently of
GPU resources; clipping a convex volume against a plane never splits its mesh.

## Submodules
- [mesh3::projection](mesh3/projection.md)

## Structs
### `ConvexMesh`
An immutable, closed convex mesh. This type's coverage algorithms require
convexity; arbitrary concave triangle soups must not use this contract.

#### Implementations
##### `projected`
<pre><code class="api-signature language-rust ignore">pub fn projected(self: &amp;Rc&lt;Self&gt;, projection: <a href="../../api/pax-runtime-api/mesh3/projection.md#meshprojection">MeshProjection</a>, viewport: Option&lt;Rect&gt;) -&gt; Rc&lt;<a href="../../api/pax-runtime-api/mesh3/projection.md#projectedconvexmesh">ProjectedConvexMesh</a>&gt;</code></pre>

Projects the actual mesh surface, clipping before division. Supplying a
viewport also bounds a canvas chart whose projective horizon crosses it.

##### `posed`
<pre><code class="api-signature language-rust ignore">pub fn posed(self: &amp;Rc&lt;Self&gt;, transform: [f64; 16]) -&gt; Option&lt;Rc&lt;<a href="../../api/pax-runtime-api/mesh3.md#posedconvexmesh">PosedConvexMesh</a>&gt;&gt;</code></pre>

Retains a finite, nonsingular affine pose. Matrices are column-major.

##### `sphere`
<pre><code class="api-signature language-rust ignore">pub fn sphere(longitudes: usize, latitudes: usize) -&gt; Rc&lt;Self&gt;</code></pre>

Unit sphere centered at the origin. Ellipsoids scale this shared topology.

---

### `MeshVertex`
One local-space mesh vertex. Normals are outward-facing; UVs span the surface.

#### Properties
##### `position`
Type: [`f32`; 3]

##### `normal`
Type: [`f32`; 3]

##### `uv`
Type: [`f32`; 2]

---

### `OpaqueMeshDraw`
Settled opaque mesh draw, retaining shared topology across replay and motion.

#### Properties
##### `mesh`
Type: `Rc`<[`ConvexMesh`](../../api/pax-runtime-api/mesh3.md#convexmesh)>

##### `world`
Type: [`f64`; 16]

Local mesh to unscrolled world, used for lighting.

##### `surface`
Type: [`MeshProjection`](../../api/pax-runtime-api/mesh3/projection.md#meshprojection)

Local mesh to homogeneous canvas-content coordinates, with camera clipping.

##### `color`
Type: [`Color`](../../api/pax-runtime-api/color.md#color)

##### `material`
Type: [`ResolvedMaterial`](../../api/pax-runtime-api/drawing.md#resolvedmaterial)

---

### `PosedConvexMesh`
One affine instance of a convex mesh, in a chosen world/presentation space.

#### Properties
##### `mesh`
Type: `Rc`<[`ConvexMesh`](../../api/pax-runtime-api/mesh3.md#convexmesh)>

##### `transform`
Type: [`f64`; 16]

#### Implementations
##### `projected`
<pre><code class="api-signature language-rust ignore">pub fn projected(&amp;self, projection: <a href="../../api/pax-runtime-api/mesh3/projection.md#meshprojection">MeshProjection</a>) -&gt; Rc&lt;<a href="../../api/pax-runtime-api/mesh3/projection.md#projectedconvexmesh">ProjectedConvexMesh</a>&gt;</code></pre>

Retains up to four camera/chart projections of this world pose. The
supplied projection includes the world transform and camera clip planes;
coverage is not additionally clipped to a viewport.

##### `coverage_in_front`
<pre><code class="api-signature language-rust ignore">pub fn coverage_in_front(&amp;self, plane: <a href="../../api/pax-runtime-api/rendering.md#depthplane">DepthPlane</a>) -&gt; Vec&lt;[f64; 2]&gt;</code></pre>

Projects the portion closer than a planar UI surface. Edge/plane crossings
are included, so a panel passing through the volume gets the intersection
contour, not merely a subset of the solid's existing silhouette vertices.

##### `depth_at`
<pre><code class="api-signature language-rust ignore">pub fn depth_at(&amp;self, x: f64, y: f64) -&gt; Option&lt;f64&gt;</code></pre>

Nearest surface depth under orthographic viewing, matching the rendered
triangles rather than a different analytic approximation of the solid.

##### `silhouette`
<pre><code class="api-signature language-rust ignore">pub fn silhouette(&amp;self) -&gt; Vec&lt;[f64; 2]&gt;</code></pre>

Projected silhouette of the tessellated solid in these x/y coordinates.

## Functions
### `convex_hull`
<pre><code class="api-signature language-rust ignore">pub fn convex_hull(points: Vec&lt;[f64; 2]&gt;) -&gt; Vec&lt;[f64; 2]&gt;</code></pre>

Counterclockwise projected hull. Collinear/duplicate vertices are removed.
