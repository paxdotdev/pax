# engine::node_interface
<!-- summary: API docs for pax-runtime::engine::node_interface. -->
<!-- tags: api, pax-runtime -->

## Structs
### `NodeInterface`
Designer/runtime inspection handle for an expanded node.

#### Implementations
##### `auto_size`
<pre><code class="api-signature language-rust ignore">pub fn auto_size(&amp;self) -&gt; Option&lt;(f64, f64)&gt;</code></pre>

Auto-sized bounds reported by a native or text-backed node.

##### `children`
<pre><code class="api-signature language-rust ignore">pub fn children(&amp;self) -&gt; Vec&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodeinterface">NodeInterface</a>&gt;</code></pre>

Mounted child nodes.

##### `containing_component`
<pre><code class="api-signature language-rust ignore">pub fn containing_component(&amp;self) -&gt; Option&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodeinterface">NodeInterface</a>&gt;</code></pre>

Containing component for template scoping and slot ownership.

##### `engine_id`
<pre><code class="api-signature language-rust ignore">pub fn engine_id(&amp;self) -&gt; <a href="/api/internal/pax-runtime/properties.md#expandednodeidentifier">ExpandedNodeIdentifier</a></code></pre>

Runtime-expanded id for this concrete node.

##### `flattened_slot_children_count`
<pre><code class="api-signature language-rust ignore">pub fn flattened_slot_children_count(&amp;self) -&gt; <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;usize&gt;</code></pre>

Reactive count of slot children after repeat/conditional flattening.

##### `global_id`
<pre><code class="api-signature language-rust ignore">pub fn global_id(&amp;self) -&gt; Option&lt;<a href="/api/internal/pax-manifest/index.md#uniquetemplatenodeidentifier">UniqueTemplateNodeIdentifier</a>&gt;</code></pre>

Compiler-global template id for this node, if it originated from a template node.

##### `has_id`
<pre><code class="api-signature language-rust ignore">pub fn has_id(&amp;self, id: &amp;str) -&gt; bool</code></pre>

Test the template `id` common property.

##### `instance_flags`
<pre><code class="api-signature language-rust ignore">pub fn instance_flags(&amp;self) -&gt; <a href="/api/internal/pax-runtime/rendering.md#instanceflags">InstanceFlags</a></code></pre>

Static flags from the node's instance.

##### `is_descendant_of`
<pre><code class="api-signature language-rust ignore">pub fn is_descendant_of(&amp;self, node: &amp;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodeinterface">NodeInterface</a>) -&gt; bool</code></pre>

True if this node is below `node` in the template-parent chain.

##### `is_of_type`
<pre><code class="api-signature language-rust ignore">pub fn is_of_type&lt;T: <a href="/api/pax-runtime-api/pax_value.md#tofrompaxany">ToFromPaxAny</a>&gt;(&amp;self) -&gt; bool</code></pre>

Check whether the node's property object is of type `T`.

##### `layout_properties`
<pre><code class="api-signature language-rust ignore">pub fn layout_properties(&amp;self) -&gt; <a href="/api/internal/pax-runtime/layout.md#layoutproperties">LayoutProperties</a></code></pre>

Current layout properties after common-property collection.

##### `render_parent`
<pre><code class="api-signature language-rust ignore">pub fn render_parent(&amp;self) -&gt; Option&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodeinterface">NodeInterface</a>&gt;</code></pre>

Parent in render traversal order.

##### `template_parent`
<pre><code class="api-signature language-rust ignore">pub fn template_parent(&amp;self) -&gt; Option&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodeinterface">NodeInterface</a>&gt;</code></pre>

Parent in template ownership order.

##### `transform_and_bounds`
<pre><code class="api-signature language-rust ignore">pub fn transform_and_bounds(&amp;self) -&gt; <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;<a href="/api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;&gt;</code></pre>

Reactive transform-and-bounds property for this node.

##### `with_properties`
<pre><code class="api-signature language-rust ignore">pub fn with_properties&lt;V, T: <a href="/api/pax-runtime-api/pax_value.md#tofrompaxany">ToFromPaxAny</a>&gt;(&amp;self, f: impl FnOnce(&amp;mut T) -&gt; V) -&gt; Option&lt;V&gt;</code></pre>

Borrow the node's typed property object if it has the requested type.

---

### `NodeLocal`
Marker coordinate space for a node's local layout space.
