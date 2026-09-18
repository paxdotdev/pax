# properties
<!-- summary: API docs for pax-runtime::properties. -->
<!-- tags: api, pax-runtime -->

## Structs
### `ExpandedNodeIdentifier`
Stable runtime identifier assigned to an expanded node.

#### Properties
##### `0`
Type: `u32`

#### Implementations
##### `to_u32`
<pre><code class="api-signature language-rust ignore">pub fn to_u32(&amp;self) -&gt; u32</code></pre>

Convert to the integer id passed across chassis message boundaries.

---

### `ExpressionContext`
Data structure used for dynamic injection of values
into Expressions, maintaining a pointer e.g. to the current
stack frame to enable evaluation of properties & dependencies

#### Properties
##### `stack_frame`
Type: `Rc`<[`RuntimePropertiesStackFrame`](../../../api/internal/pax-runtime/properties.md#runtimepropertiesstackframe)>

---

### `RuntimeContext`
Shared context for properties pass recursion

#### Properties
##### `layer_count`
Type: `Cell`<`usize`>

##### `dirty_canvases`
Type: `Rc`<`RefCell`<`Vec`<`bool`>>>

#### Implementations
##### `add_to_cache`
<pre><code class="api-signature language-rust ignore">pub fn add_to_cache(&amp;self, node: &amp;Rc&lt;ExpandedNode&gt;)</code></pre>

Add a node to runtime lookup caches.

##### `canvas_node_light_mask`
<pre><code class="api-signature language-rust ignore">pub fn canvas_node_light_mask(&amp;self, id: <a href="../../../api/internal/pax-runtime/properties.md#expandednodeidentifier">ExpandedNodeIdentifier</a>) -&gt; u32</code></pre>

Return the direct-light membership mask resolved for a retained canvas node.

##### `capture_touch_target`
<pre><code class="api-signature language-rust ignore">pub fn capture_touch_target(&amp;self, identifier: i64, target: <a href="../../../api/internal/pax-runtime/properties.md#expandednodeidentifier">ExpandedNodeIdentifier</a>)</code></pre>

Route a touch sequence to the node hit at touch-down, even after the finger moves away.

##### `captured_touch_target`
<pre><code class="api-signature language-rust ignore">pub fn captured_touch_target(&amp;self, identifier: i64) -&gt; Option&lt;Rc&lt;ExpandedNode&gt;&gt;</code></pre>

Resolve the node captured for an active touch sequence.

##### `clear_all_dirty_canvases`
<pre><code class="api-signature language-rust ignore">pub fn clear_all_dirty_canvases(&amp;self)</code></pre>

Mark every canvas layer clean.

##### `clear_layer_scroller_owners`
<pre><code class="api-signature language-rust ignore">pub fn clear_layer_scroller_owners(&amp;self)</code></pre>

Clear render-layer-to-scroller ownership before recomputing occlusion.

##### `clear_root_expanded_node`
<pre><code class="api-signature language-rust ignore">pub fn clear_root_expanded_node(&amp;self)</code></pre>

Clear the registered root expanded node.

##### `clear_visual_viewport_state`
<pre><code class="api-signature language-rust ignore">pub fn clear_visual_viewport_state(&amp;self)</code></pre>

Clear cached visual viewport state.

##### `get_elements_beneath_ray`
<pre><code class="api-signature language-rust ignore">pub fn get_elements_beneath_ray(&amp;self, root: Option&lt;Rc&lt;ExpandedNode&gt;&gt;, ray: Point2&lt;<a href="../../../api/pax-runtime-api/platform.md#window">Window</a>&gt;, limit_one: bool, accum: Vec&lt;Rc&lt;ExpandedNode&gt;&gt;, hit_invisible: bool) -&gt; Vec&lt;Rc&lt;ExpandedNode&gt;&gt;</code></pre>

Simple 2D raycasting: the coordinates of the ray represent a
ray running orthogonally to the view plane, intersecting at
the specified point `ray`.  Areas outside of clipping bounds will
not register a `hit`, nor will elements that suppress input events.

##### `get_expanded_node_by_eid`
<pre><code class="api-signature language-rust ignore">pub fn get_expanded_node_by_eid(&amp;self, id: <a href="../../../api/internal/pax-runtime/properties.md#expandednodeidentifier">ExpandedNodeIdentifier</a>) -&gt; Option&lt;Rc&lt;ExpandedNode&gt;&gt;</code></pre>

Look up an expanded node by runtime id.

##### `get_expanded_nodes_by_global_ids`
<pre><code class="api-signature language-rust ignore">pub fn get_expanded_nodes_by_global_ids(&amp;self, uni: &amp;<a href="../../../api/internal/pax-manifest/index.md#uniquetemplatenodeidentifier">UniqueTemplateNodeIdentifier</a>) -&gt; Vec&lt;Rc&lt;ExpandedNode&gt;&gt;</code></pre>

Finds all ExpandedNodes with corresponding UniqueTemplateNodeIdentifier

##### `get_expanded_nodes_by_id`
<pre><code class="api-signature language-rust ignore">pub fn get_expanded_nodes_by_id(&amp;self, id: &amp;str) -&gt; Vec&lt;Rc&lt;ExpandedNode&gt;&gt;</code></pre>

Finds all ExpandedNodes with the CommonProperty#id matching the provided string

##### `get_layer_scroller_owner`
<pre><code class="api-signature language-rust ignore">pub fn get_layer_scroller_owner(&amp;self, layer_id: usize) -&gt; Option&lt;<a href="../../../api/internal/pax-runtime/properties.md#expandednodeidentifier">ExpandedNodeIdentifier</a>&gt;</code></pre>

Find the scroller that owns a render layer, when one exists.

##### `get_root_scroller_id`
<pre><code class="api-signature language-rust ignore">pub fn get_root_scroller_id(&amp;self) -&gt; Option&lt;u32&gt;</code></pre>

Current page-scroll-backed root scroller id.

##### `get_screenshot_map`
<pre><code class="api-signature language-rust ignore">pub fn get_screenshot_map(&amp;self) -&gt; Rc&lt;RefCell&lt;HashMap&lt;u32, <a href="../../../api/internal/pax-message/index.md#screenshotdata">ScreenshotData</a>&gt;&gt;&gt;</code></pre>

Shared screenshot capture map keyed by request id.

##### `get_scroller_surface_scroll`
<pre><code class="api-signature language-rust ignore">pub fn get_scroller_surface_scroll(&amp;self, id: u32) -&gt; Option&lt;(f64, f64)&gt;</code></pre>

Fetch the presentation scroll offset for a native scroller surface, falling back to the
authoritative scroll position when presentation scroll is unavailable.

##### `get_scroller_surface_state`
<pre><code class="api-signature language-rust ignore">pub fn get_scroller_surface_state(&amp;self, id: u32) -&gt; Option&lt;<a href="../../../api/internal/pax-runtime/properties.md#scrollersurfacestate">ScrollerSurfaceState</a>&gt;</code></pre>

Fetch cached scroller surface state by node id.

##### `get_topmost_element_beneath_ray`
<pre><code class="api-signature language-rust ignore">pub fn get_topmost_element_beneath_ray(self: &amp;Rc&lt;Self&gt;, ray: Point2&lt;<a href="../../../api/pax-runtime-api/platform.md#window">Window</a>&gt;) -&gt; Option&lt;Rc&lt;ExpandedNode&gt;&gt;</code></pre>

Alias for `get_elements_beneath_ray` with `limit_one = true`

##### `get_visual_viewport_state`
<pre><code class="api-signature language-rust ignore">pub fn get_visual_viewport_state(&amp;self) -&gt; Option&lt;<a href="../../../api/internal/pax-runtime/properties.md#visualviewportstate">VisualViewportState</a>&gt;</code></pre>

Return cached browser visual viewport state, if available.

##### `is_canvas_dirty`
<pre><code class="api-signature language-rust ignore">pub fn is_canvas_dirty(&amp;self, id: &amp;usize) -&gt; bool</code></pre>

Check whether a canvas layer needs redraw.

##### `layer_has_canvas_drawables`
<pre><code class="api-signature language-rust ignore">pub fn layer_has_canvas_drawables(&amp;self, layer: usize) -&gt; bool</code></pre>

Return whether a render layer currently has canvas work to paint.

##### `load_screenshot`
<pre><code class="api-signature language-rust ignore">pub fn load_screenshot(&amp;self, id: u32, data: <a href="../../../api/internal/pax-message/index.md#screenshotdata">ScreenshotData</a>) -&gt; bool</code></pre>

Store a screenshot payload delivered by the chassis.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(globals: <a href="../../../api/internal/pax-runtime/engine.md#globals">Globals</a>) -&gt; Self</code></pre>

Create a runtime context for normal app execution.

##### `register_layer_scroller_owner`
<pre><code class="api-signature language-rust ignore">pub fn register_layer_scroller_owner(&amp;self, layer_id: usize, scroller_id: <a href="../../../api/internal/pax-runtime/properties.md#expandednodeidentifier">ExpandedNodeIdentifier</a>)</code></pre>

Record that a render layer is owned by a particular scroller.

##### `register_root_expanded_node`
<pre><code class="api-signature language-rust ignore">pub fn register_root_expanded_node(&amp;self, root: &amp;Rc&lt;ExpandedNode&gt;)</code></pre>

Store the root expanded node after it has been initialized.

##### `release_touch_target`
<pre><code class="api-signature language-rust ignore">pub fn release_touch_target(&amp;self, identifier: i64) -&gt; Option&lt;Rc&lt;ExpandedNode&gt;&gt;</code></pre>

Release and resolve the node captured for a completed touch sequence.

##### `remove_from_cache`
<pre><code class="api-signature language-rust ignore">pub fn remove_from_cache(&amp;self, node: &amp;Rc&lt;ExpandedNode&gt;)</code></pre>

Remove a node from runtime lookup caches.

##### `remove_scroller_surface_state`
<pre><code class="api-signature language-rust ignore">pub fn remove_scroller_surface_state(&amp;self, id: u32)</code></pre>

Remove cached scroller surface state.

##### `resize_canvas_layers_to`
<pre><code class="api-signature language-rust ignore">pub fn resize_canvas_layers_to(&amp;self, id: usize)</code></pre>

Ensure the dirty-canvas table has entries up to the requested layer count.

##### `set_canvas_dirty`
<pre><code class="api-signature language-rust ignore">pub fn set_canvas_dirty(&amp;self, id: usize)</code></pre>

Mark a canvas layer dirty.

##### `set_canvas_drawable_layers`
<pre><code class="api-signature language-rust ignore">pub fn set_canvas_drawable_layers(&amp;self, layers: HashSet&lt;usize&gt;)</code></pre>

Replace the set of render layers that currently contain canvas drawables.

##### `set_root_scroller_id`
<pre><code class="api-signature language-rust ignore">pub fn set_root_scroller_id(&amp;self, id: Option&lt;u32&gt;)</code></pre>

Mark which node currently delegates root scrolling behavior to the page.

##### `set_scroller_surface_state`
<pre><code class="api-signature language-rust ignore">pub fn set_scroller_surface_state(&amp;self, id: u32, state: <a href="../../../api/internal/pax-runtime/properties.md#scrollersurfacestate">ScrollerSurfaceState</a>) -&gt; <a href="../../../api/internal/pax-runtime/properties.md#scrollersurfacestatechange">ScrollerSurfaceStateChange</a></code></pre>

Remember browser-owned scroller state for native compositing and scroll transforms.

##### `set_visual_viewport_state`
<pre><code class="api-signature language-rust ignore">pub fn set_visual_viewport_state(&amp;self, state: <a href="../../../api/internal/pax-runtime/properties.md#visualviewportstate">VisualViewportState</a>)</code></pre>

Cache the browser visual viewport state for root scroller math.

##### `update_scroller_surface_scroll`
<pre><code class="api-signature language-rust ignore">pub fn update_scroller_surface_scroll(&amp;self, id: u32, scroll_x: f64, scroll_y: f64, presentation_scroll_x: f64, presentation_scroll_y: f64) -&gt; <a href="../../../api/internal/pax-runtime/properties.md#scrollersurfacestatechange">ScrollerSurfaceStateChange</a></code></pre>

Update hot scroll offsets for an existing native scroller surface without touching
structural state.

---

### `RuntimePropertiesStackFrame`
Data structure for a single frame of our runtime stack, including
a reference to its parent frame and `properties` for
runtime evaluation, e.g. of Expressions.  `RuntimePropertiesStackFrame`s also track
timeline playhead position.

`Component`s push `RuntimePropertiesStackFrame`s before computing properties and pop them after computing, thus providing a
hierarchical store of node-relevant data that can be bound to symbols in expressions.

---

### `ScrollerSurfaceState`
Last-known scroll state for a native or browser-owned scroller surface.

#### Properties
##### `viewport_width`
Type: `f64`

##### `viewport_height`
Type: `f64`

##### `content_width`
Type: `f64`

##### `content_height`
Type: `f64`

##### `scroll_x`
Type: `f64`

##### `scroll_y`
Type: `f64`

##### `presentation_scroll_x`
Type: `f64`

##### `presentation_scroll_y`
Type: `f64`

##### `clip_content`
Type: `bool`

---

### `VisualViewportState`
Browser visual viewport state used when page scrolling participates in root scroller behavior.

#### Properties
##### `width`
Type: `f64`

##### `height`
Type: `f64`

##### `offset_x`
Type: `f64`

##### `offset_y`
Type: `f64`

##### `page_scroll_x`
Type: `f64`

##### `page_scroll_y`
Type: `f64`

## Enums
### `ScrollerSurfaceStateChange`
Coarse classification of changes to a native or browser-owned scroller surface.

#### Variants
##### `Unchanged`
##### `ScrollOnly`
##### `Structural`
