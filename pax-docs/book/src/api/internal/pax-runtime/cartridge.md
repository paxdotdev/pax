# cartridge
<!-- summary: API docs for pax-runtime::cartridge. -->
<!-- tags: api, pax-runtime -->

## Structs
### `ComponentPropertyDescriptor`
Runtime descriptor for applying all resolved layers of a generated component
property.

#### Properties
##### `name`
Type: &'`static` `str`

Property name as it appears in Pax templates and settings.

##### `apply_entries`
Type: `fn`()

Generated applicator for all resolved layers of this property.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub const fn new(name: &amp;&#39;static str, apply_entries: fn()) -&gt; Self</code></pre>

Creates a descriptor for a generated component property.

## Functions
### `apply_component_property`
<pre><code class="api-signature language-rust ignore">pub fn apply_component_property&lt;T&gt;(property: &amp;mut <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;T&gt;, name: &amp;str, value_definition: &amp;<a href="/api/internal/pax-manifest/index.md#valuedefinition">ValueDefinition</a>, stack: &amp;Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;, timeline_stack: Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;, build_block: fn()) where T: CoercionRules + <a href="/api/pax-runtime-api/properties.md#propertyvalue">PropertyValue</a> + <a href="/api/pax-runtime-api/pax_value.md#topaxvalue">ToPaxValue</a></code></pre>

Replaces a typed component property with the value produced from a single
value definition.

---

### `build_component_property`
<pre><code class="api-signature language-rust ignore">pub fn build_component_property&lt;T&gt;(name: &amp;str, value_definition: &amp;<a href="/api/internal/pax-manifest/index.md#valuedefinition">ValueDefinition</a>, stack: &amp;Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;, timeline_stack: Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;, build_block: fn()) -&gt; <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;T&gt; where T: CoercionRules + <a href="/api/pax-runtime-api/properties.md#propertyvalue">PropertyValue</a> + <a href="/api/pax-runtime-api/pax_value.md#topaxvalue">ToPaxValue</a></code></pre>

Builds a typed component property from a single value definition. Callers
that apply layered settings should bind `$base` before calling this helper.

---

### `create_new_common_properties`
<pre><code class="api-signature language-rust ignore">pub fn create_new_common_properties(defined_properties: &amp;BTreeMap&lt;String, <a href="/api/internal/pax-manifest/index.md#valuedefinition">ValueDefinition</a>&gt;, stack_frame: &amp;Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;) -&gt; Rc&lt;RefCell&lt;<a href="/api/pax-runtime-api/layout.md#commonproperties">CommonProperties</a>&gt;&gt;</code></pre>

Creates common properties from a flattened property map.

---

### `create_new_common_properties_from_columns`
<pre><code class="api-signature language-rust ignore">pub fn create_new_common_properties_from_columns(property_columns: &amp;RuntimeResolvedPropertyColumns, stack_frame: &amp;Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;) -&gt; Rc&lt;RefCell&lt;<a href="/api/pax-runtime-api/layout.md#commonproperties">CommonProperties</a>&gt;&gt;</code></pre>

Creates common properties from resolved columns, preserving layer order so
`$base` can reference each prior layer.

---

### `property_columns_from_defined_properties`
<pre><code class="api-signature language-rust ignore">pub fn property_columns_from_defined_properties(defined_properties: &amp;BTreeMap&lt;String, <a href="/api/internal/pax-manifest/index.md#valuedefinition">ValueDefinition</a>&gt;) -&gt; RuntimeResolvedPropertyColumns</code></pre>

Converts a flattened property map into one-entry columns for legacy callers
that do not participate in selector/import precedence layering.

---

### `stack_with_base`
<pre><code class="api-signature language-rust ignore">pub fn stack_with_base&lt;T&gt;(stack: &amp;Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;, base_property: <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;T&gt;) -&gt; Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt; where T: <a href="/api/pax-runtime-api/properties.md#propertyvalue">PropertyValue</a> + <a href="/api/pax-runtime-api/pax_value.md#topaxvalue">ToPaxValue</a></code></pre>

Returns a stack frame with `$base` bound to the supplied previous-layer
property value.

---

### `stack_with_optional_base`
<pre><code class="api-signature language-rust ignore">pub fn stack_with_optional_base&lt;T&gt;(stack: &amp;Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;, base_property: <a href="/api/pax-runtime-api/properties.md#property">Property</a>&lt;Option&lt;T&gt;&gt;) -&gt; Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt; where T: <a href="/api/pax-runtime-api/properties.md#propertyvalue">PropertyValue</a> + <a href="/api/pax-runtime-api/pax_value.md#topaxvalue">ToPaxValue</a></code></pre>

Returns a stack frame with `$base` bound to an optional previous common
property value, exposing `T::default()` when the previous layer is `None`.

---

### `update_existing_common_properties`
<pre><code class="api-signature language-rust ignore">pub fn update_existing_common_properties(expanded_node: &amp;Rc&lt;ExpandedNode&gt;, defined_properties: &amp;BTreeMap&lt;String, <a href="/api/internal/pax-manifest/index.md#valuedefinition">ValueDefinition</a>&gt;, stack_frame: &amp;Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;)</code></pre>

Applies a flattened common-property map to an existing expanded node.

---

### `update_existing_common_properties_from_columns`
<pre><code class="api-signature language-rust ignore">pub fn update_existing_common_properties_from_columns(expanded_node: &amp;Rc&lt;ExpandedNode&gt;, property_columns: &amp;RuntimeResolvedPropertyColumns, stack_frame: &amp;Rc&lt;<a href="/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe">RuntimePropertiesStackFrame</a>&gt;)</code></pre>

Applies resolved common-property columns to an existing expanded node,
preserving layer order so `$base` can reference each prior layer.

## Constants
### `BASE_SYMBOL`
PAXEL symbol bound while resolving a property layer to the value from the
preceding layer in that same property's precedence stack.
