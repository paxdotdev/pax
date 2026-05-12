# layout
<!-- summary: API docs for pax-runtime::layout. -->
<!-- tags: api, pax-runtime -->

## Structs
### `LayoutHull`
Per-axis local extents contributed by a node subtree for container measurement.

Coordinates are expressed in the node's local layout space. Validity is tracked
per axis so parent-dependent axes can be ignored without discarding the entire
subtree.

#### Properties
##### `min_x`
Type: `f64`

##### `max_x`
Type: `f64`

##### `min_y`
Type: `f64`

##### `max_y`
Type: `f64`

##### `valid_x`
Type: `bool`

##### `valid_y`
Type: `bool`

---

### `LayoutProperties`
Unresolved layout inputs copied out of common properties before geometry calculation.

#### Properties
##### `x`
Type: `Option`<[`Size`](/api/pax-runtime-api/layout.md#size)>

##### `y`
Type: `Option`<[`Size`](/api/pax-runtime-api/layout.md#size)>

##### `width`
Type: `Option`<[`Size`](/api/pax-runtime-api/layout.md#size)>

##### `height`
Type: `Option`<[`Size`](/api/pax-runtime-api/layout.md#size)>

##### `rotate`
Type: `Option`<[`Rotation`](/api/pax-runtime-api/transform.md#rotation)>

##### `scale_x`
Type: `Option`<[`Percent`](/api/pax-runtime-api/color.md#percent)>

##### `scale_y`
Type: `Option`<[`Percent`](/api/pax-runtime-api/color.md#percent)>

##### `anchor_x`
Type: `Option`<[`Size`](/api/pax-runtime-api/layout.md#size)>

##### `anchor_y`
Type: `Option`<[`Size`](/api/pax-runtime-api/layout.md#size)>

##### `skew_x`
Type: `Option`<[`Rotation`](/api/pax-runtime-api/transform.md#rotation)>

##### `skew_y`
Type: `Option`<[`Rotation`](/api/pax-runtime-api/transform.md#rotation)>

#### Implementations
##### `fill`
<pre><code class="api-signature language-rust ignore">pub fn fill() -&gt; Self</code></pre>

Full-size defaults for nodes that should fill their containing bounds.

---

### `TransformAndBounds`
Pax's canonical representation of position, size, and transform, encoded
as a transform (translation, rotation, scale, skew) and a separate width/height (bounds) value.
Bounds are expressed as the (x1, y1) values of the axis-aligned pre-transform bounding box,
where (x0, y0) are the origin.

In this model, position is a derived property, calculated by applying the transform to the bounding box.

#### Properties
##### `transform`
Type: `Transform2`<`F`, `T`>

##### `bounds`
Type: (`f64`, `f64`)

#### Implementations
##### `as_pure_scale`
<pre><code class="api-signature language-rust ignore">pub fn as_pure_scale(self) -&gt; Self</code></pre>

Move bounds into the transform as scale, leaving unit bounds.

##### `as_pure_size`
<pre><code class="api-signature language-rust ignore">pub fn as_pure_size(self) -&gt; Self</code></pre>

Move scale from the transform into the bounds field.

##### `as_transform`
<pre><code class="api-signature language-rust ignore">pub fn as_transform(&amp;self) -&gt; Transform2&lt;F, T&gt;</code></pre>

Convert this split representation into a single affine transform.

##### `cast_spaces`
<pre><code class="api-signature language-rust ignore">pub fn cast_spaces&lt;A: <a href="/api/pax-runtime-api/math.md#space">Space</a>, B: <a href="/api/pax-runtime-api/math.md#space">Space</a>&gt;(self) -&gt; <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;A, B&gt;</code></pre>

Retype coordinate-space markers without changing numeric values.

##### `center`
<pre><code class="api-signature language-rust ignore">pub fn center(&amp;self) -&gt; Point2&lt;T&gt;</code></pre>

Center point of this transformed box.

##### `contains_point`
<pre><code class="api-signature language-rust ignore">pub fn contains_point(&amp;self, point: Point2&lt;T&gt;) -&gt; bool</code></pre>

Test whether a point falls inside this transformed box.

##### `corners`
<pre><code class="api-signature language-rust ignore">pub fn corners(&amp;self) -&gt; [Point2&lt;T&gt;; 4]</code></pre>

Corners of this transformed box, starting at origin and proceeding around the rectangle.

##### `intersects`
<pre><code class="api-signature language-rust ignore">pub fn intersects(&amp;self, other: &amp;Self) -&gt; bool</code></pre>

Test transformed-box intersection using the separating axis theorem.

##### `inverse`
<pre><code class="api-signature language-rust ignore">pub fn inverse(&amp;self) -&gt; <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;T, F&gt;</code></pre>

Invert the transform-and-bounds mapping.

## Functions
### `add_symmetric_padding_to_content_layout_hull`
<pre><code class="api-signature language-rust ignore">pub fn add_symmetric_padding_to_content_layout_hull(hull: <a href="/api/internal/pax-runtime/layout.md#layouthull">LayoutHull</a>, padding_x: Option&lt;<a href="/api/pax-runtime-api/layout.md#size">Size</a>&gt;, padding_y: Option&lt;<a href="/api/pax-runtime-api/layout.md#size">Size</a>&gt;) -&gt; <a href="/api/internal/pax-runtime/layout.md#layouthull">LayoutHull</a></code></pre>

Expand a content-space hull into the node's padded outer layout space.

Children are laid out inside the leading padding offset, but autosized
bounds must be solved from the content hull itself. Solving the outer size
first keeps percentage padding from feeding back through the node's current
measured bounds.

---

### `apply_container_frame`
<pre><code class="api-signature language-rust ignore">pub fn apply_container_frame(container_transform_and_bounds: <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;, container_frame: Option&lt;<a href="/api/internal/pax-runtime/container.md#containerframe">ContainerFrame</a>&gt;) -&gt; <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;</code></pre>

Apply a container-assigned child frame on top of the parent geometry.

This is the geometry seam where container-owned placement can cooperate
with descendant-authored layout and future bottom-up measurement.

---

### `apply_padding_frame`
<pre><code class="api-signature language-rust ignore">pub fn apply_padding_frame(container_transform_and_bounds: <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;, padding_x: Option&lt;<a href="/api/pax-runtime-api/layout.md#size">Size</a>&gt;, padding_y: Option&lt;<a href="/api/pax-runtime-api/layout.md#size">Size</a>&gt;) -&gt; <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;</code></pre>

Apply a node's padding to the container geometry seen by its children.

---

### `calculate_transform_and_bounds`
<pre><code class="api-signature language-rust ignore">pub fn calculate_transform_and_bounds(_: &amp;<a href="/api/internal/pax-runtime/layout.md#layoutproperties">LayoutProperties</a>, _: <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;) -&gt; <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;</code></pre>

Resolve one set of layout properties into concrete bounds and a window-space transform.

---

### `compute_tab`
<pre><code class="api-signature language-rust ignore">pub fn compute_tab(layout_properties: <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;<a href="/api/internal/pax-runtime/layout.md#layoutproperties">LayoutProperties</a>&gt;, extra_transform: <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;Option&lt;<a href="/api/pax-runtime-api/transform.md#transform2d">Transform2D</a>&gt;&gt;, container_transform_and_bounds: <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;<a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;&gt;) -&gt; <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;<a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;&gt;</code></pre>

Compute a reactive `TransformAndBounds` property from layout properties plus parent geometry.

---

### `project_child_layout_hull_to_parent_space`
<pre><code class="api-signature language-rust ignore">pub fn project_child_layout_hull_to_parent_space(parent: <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;, child: <a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;, child_hull: <a href="/api/internal/pax-runtime/layout.md#layouthull">LayoutHull</a>) -&gt; <a href="/api/internal/pax-runtime/layout.md#layouthull">LayoutHull</a></code></pre>

Project a child's local hull into its parent's local layout space.

---

### `project_layout_hull`
<pre><code class="api-signature language-rust ignore">pub fn project_layout_hull&lt;F: <a href="/api/pax-runtime-api/math.md#space">Space</a>, T: <a href="/api/pax-runtime-api/math.md#space">Space</a>&gt;(transform: Transform2&lt;F, T&gt;, hull: <a href="/api/internal/pax-runtime/layout.md#layouthull">LayoutHull</a>) -&gt; <a href="/api/internal/pax-runtime/layout.md#layouthull">LayoutHull</a></code></pre>

Project a local layout hull through the provided transform and return the
axis-aligned hull in the destination coordinate space.

---

### `resolve_padded_autosize_axis`
<pre><code class="api-signature language-rust ignore">pub fn resolve_padded_autosize_axis(content_extent: f64, padding: Option&lt;<a href="/api/pax-runtime-api/layout.md#size">Size</a>&gt;) -&gt; Option&lt;f64&gt;</code></pre>

Solve an autosized outer axis from measured content and symmetric padding.
