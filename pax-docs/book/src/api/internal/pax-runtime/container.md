# container
<!-- summary: Container-facing child ontology and geometry seams. -->
<!-- tags: api, pax-runtime -->

Container-facing child ontology and geometry seams.
Container-facing child ontology and geometry seams.

The runtime carries several child concepts, but container consumers should
reason about them on two axes:

1. Semantic role
   - `received_children`: the payload a node received from its caller
   - encapsulated implementation children: the node's own private template or
     primitive-assembled structure
2. Lifecycle slice of the received payload
   - active received children: present in `NodeContext::received_children`
   - retained exiting received children: present in
     `NodeContext::retained_received_children`

Engine details such as projection still exist, but they are transport
mechanisms rather than the primary semantic abstraction.

## Structs
### `ContainerFrame`
Parent-local frame assigned by a container to one of its received children.

This behaves like a virtual wrapper node inside the parent: the frame's
transform is composed onto the parent transform and its bounds become the
child container bounds.

#### Properties
##### `transform`
Type: `Transform2`<[`NodeLocal`](/api/internal/pax-runtime/engine/node_interface.md#nodelocal), [`NodeLocal`](/api/internal/pax-runtime/engine/node_interface.md#nodelocal)>

##### `bounds`
Type: (`f64`, `f64`)

## Enums
### `ReceivedChildrenSource`
Engine-internal selector for which child family should be normalized into
`NodeContext::received_children`.

This is a provenance selector, not the semantic API surface.

`Owned` means the node's received payload already lives in its active child
tree.

`Projected` means the node receives payload from its caller and the runtime
threads that payload through projection so it can be consumed by `slot(...)`
within the node's encapsulated implementation.

#### Variants
##### `Owned`
##### `Projected`
## Traits
### `Container`
Trait for nodes that semantically interpret child content.

Containers should treat `NodeContext::received_children` as their canonical
payload set and `NodeContext::retained_received_children` as the set of
exit-retained payload nodes that may still need placement or transition
handling.

Encapsulated implementation children remain a private detail of the node's
own template or primitive assembly and should generally not drive container
layout logic.

Containers can call this from their existing mount logic to install reactive
behavior on top of those normalized views.

## Functions
### `bind_content_measurement_effect`
<pre><code class="api-signature language-rust ignore">pub fn bind_content_measurement_effect&lt;F&gt;(expanded_node: &amp;Rc&lt;ExpandedNode&gt;, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, listener_name: &amp;&#39;static str, extra_deps: &amp;[UntypedProperty], effect: F) where F: Fn(&amp;Rc&lt;ExpandedNode&gt;, &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>) + Clone + &#39;static</code></pre>

Bind a reactive content-measurement effect to this node.

The effect is re-evaluated after the tree update pass, before occlusion and
layer-plan generation, and its dependency list is rebound whenever the
normalized received-child list changes.

---

### `measure_content_children_forward_extents`
<pre><code class="api-signature language-rust ignore">pub fn measure_content_children_forward_extents(ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>) -&gt; (Option&lt;f64&gt;, Option&lt;f64&gt;)</code></pre>

Measure forward autosize extents from received content.

---

### `measure_content_children_layout_hull`
<pre><code class="api-signature language-rust ignore">pub fn measure_content_children_layout_hull(ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>) -&gt; <a href="/api/internal/pax-runtime/layout.md#layouthull">LayoutHull</a></code></pre>

Measure the aggregate layout hull contributed by received content in the
container's local coordinate space.

Empty content is treated as a zero-sized valid hull so autosized containers
can collapse to `0x0` when they have no children.

---

### `resolve_axis_autosize`
<pre><code class="api-signature language-rust ignore">pub fn resolve_axis_autosize(autosize: bool, axis_override: Option&lt;bool&gt;, default_when_enabled: bool) -&gt; bool</code></pre>

Resolve one axis of autosize given the public `autosize` toggle plus an optional override.

---

### `resolve_content_autosize_measurement`
<pre><code class="api-signature language-rust ignore">pub fn resolve_content_autosize_measurement(ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, width_explicit: bool, height_explicit: bool) -&gt; Option&lt;(f64, f64)&gt;</code></pre>

Resolve a node's measured size from its received content.

Explicit axes keep their current container bounds; implicit axes use the
measured forward extents when they are valid. If an implicit axis cannot be
measured safely, this returns `None` so the caller can fall back.

---

### `resolve_content_autosize_measurement_with_axes`
<pre><code class="api-signature language-rust ignore">pub fn resolve_content_autosize_measurement_with_axes(ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, width_explicit: bool, height_explicit: bool, autosize_width: bool, autosize_height: bool) -&gt; Option&lt;(f64, f64)&gt;</code></pre>

Resolve a node's measured size from received content with explicit per-axis autosize control.

---

### `sync_content_autosize`
<pre><code class="api-signature language-rust ignore">pub fn sync_content_autosize(expanded_node: &amp;Rc&lt;ExpandedNode&gt;, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, enabled: bool)</code></pre>

Update `measured_size` from received content when autosize is enabled.

---

### `sync_content_autosize_with_axes`
<pre><code class="api-signature language-rust ignore">pub fn sync_content_autosize_with_axes(expanded_node: &amp;Rc&lt;ExpandedNode&gt;, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, autosize_width: bool, autosize_height: bool)</code></pre>

Update `measured_size` from received content with explicit per-axis autosize control.
