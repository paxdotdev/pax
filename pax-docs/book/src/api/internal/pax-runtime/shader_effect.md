# shader_effect
<!-- summary: Runtime glue for experimental GPU-only subtree filters. -->
<!-- tags: api, pax-runtime -->

Runtime glue for experimental GPU-only subtree filters.
Available with the opt-in `shaders` Cargo feature.

## Functions
### `bind_shader_effect`
<pre><code class="api-signature language-rust ignore">pub fn bind_shader_effect(node: &amp;Rc&lt;ExpandedNode&gt;, context: &amp;Rc&lt;<a href="../../../api/internal/pax-runtime/properties.md#runtimecontext">RuntimeContext</a>&gt;)</code></pre>

Subscribe an effect primitive's typed inputs and presentation geometry to canvas updates.
The default `InstanceNode::handle_mount` installs this for primitives with a shader recipe.
A primitive overriding that hook must call this after expanding its children.
