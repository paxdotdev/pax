# mesh3::projection
<!-- summary: Camera projection and clipped surface coverage for opaque convex meshes. -->
<!-- tags: api, pax-runtime-api -->

Camera projection and clipped surface coverage for opaque convex meshes.

## Structs
### `MeshProjection`
Homogeneous solid projection. Camera clip equations stay in mesh coordinates
when a projected scroller rebases x/y/w into its own canvas chart.

#### Implementations
##### `camera_clip_rows`
<pre><code class="api-signature language-rust ignore">pub fn camera_clip_rows(self) -&gt; [[f64; 4]; 2]</code></pre>

Original camera z and w equations in mesh-local coordinates.

##### `clips_depth`
<pre><code class="api-signature language-rust ignore">pub fn clips_depth(self) -&gt; bool</code></pre>

Whether the camera's near/far interval clips this surface.

##### `matrix`
<pre><code class="api-signature language-rust ignore">pub fn matrix(self) -&gt; [f64; 16]</code></pre>

Mesh-local to destination-chart coordinates, before division by w.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(matrix: [f64; 16], clip_depth: bool) -&gt; Option&lt;Self&gt;</code></pre>

Retains a finite column-major pixel-homogeneous matrix. Larger z/w is
nearer; `clip_depth` limits the original camera depth to 0..=w.

##### `post_mapping`
<pre><code class="api-signature language-rust ignore">pub fn post_mapping(self, mapping: <a href="../../../api/pax-runtime-api/math/projection.md#planarprojection">PlanarProjection</a>) -&gt; Self</code></pre>

Changes the destination chart without changing the camera's clip interval.

##### `pre_transform`
<pre><code class="api-signature language-rust ignore">pub fn pre_transform(self, transform: [f64; 16]) -&gt; Self</code></pre>

Composes mesh-local geometry before both projection and camera clipping.

---

### `ProjectedConvexMesh`
Camera-clipped surface triangles used by culling, picking and occlusion.
GPU topology remains shared and immutable; these are CPU coverage records.

#### Properties
##### `mesh`
Type: `Rc`<[`ConvexMesh`](../../../api/pax-runtime-api/mesh3.md#convexmesh)>

##### `projection`
Type: [`MeshProjection`](../../../api/pax-runtime-api/mesh3/projection.md#meshprojection)

#### Implementations
##### `coverage_bounds`
<pre><code class="api-signature language-rust ignore">pub fn coverage_bounds(&amp;self) -&gt; Option&lt;Rect&gt;</code></pre>

`None` is conservatively unbounded; an empty clipped mesh has zero coverage.

##### `depth_at`
<pre><code class="api-signature language-rust ignore">pub fn depth_at(&amp;self, x: f64, y: f64) -&gt; Option&lt;f64&gt;</code></pre>

Closest rendered triangle depth; camera-clipped caps are not invented.

##### `front_contours`
<pre><code class="api-signature language-rust ignore">pub fn front_contours(&amp;self, plane: <a href="../../../api/pax-runtime-api/rendering.md#depthplane">DepthPlane</a>) -&gt; Vec&lt;Vec&lt;[f64; 2]&gt;&gt;</code></pre>

Positive-winding contours whose nonzero union covers rendered surfaces
closer than a plane. Clipped solids can have holes; never convexify them.

##### `silhouette`
<pre><code class="api-signature language-rust ignore">pub fn silhouette(&amp;self) -&gt; Vec&lt;[f64; 2]&gt;</code></pre>

Conservative convex envelope. Use `front_contours` for exact occlusion
because a clipped surface can leave openings within this envelope.

##### `triangles`
<pre><code class="api-signature language-rust ignore">pub fn triangles(&amp;self) -&gt; impl Iterator + &#39;_</code></pre>

Actual surviving surface triangles in destination coordinates.
