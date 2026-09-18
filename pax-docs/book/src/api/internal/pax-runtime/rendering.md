# rendering
<!-- summary: API docs for pax-runtime::rendering. -->
<!-- tags: api, pax-runtime -->

## Structs
### `BaseInstance`
Shared storage carried by every concrete `InstanceNode`.

#### Properties
##### `handler_registry`
Type: `Option`<`Rc`<`RefCell`<[`HandlerRegistry`](../../../api/internal/pax-runtime/engine.md#handlerregistry)>>>

##### `instance_prototypical_properties`
Type: [`PropertiesInit`](../../../api/internal/pax-runtime/rendering.md#propertiesinit)

##### `instance_prototypical_common_properties`
Type: [`CommonPropertiesInit`](../../../api/internal/pax-runtime/rendering.md#commonpropertiesinit)

##### `component_settings`
Type: `Option`<`Vec`<[`SettingsBlockElement`](../../../api/internal/pax-manifest/index.md#settingsblockelement)>>

##### `template_node_identifier`
Type: `Option`<[`UniqueTemplateNodeIdentifier`](../../../api/internal/pax-manifest/index.md#uniquetemplatenodeidentifier)>

##### `template_node_type_id`
Type: `Option`<[`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)>

##### `template_node_selector_info`
Type: `Option`<[`TemplateNodeSelectorInfo`](../../../api/internal/pax-manifest/selectors.md#templatenodeselectorinfo)>

##### `transition_config`
Type: `ComponentTransitionConfig`

##### `properties_scope`
Type: [`PropertiesScopeInit`](../../../api/internal/pax-runtime/rendering.md#propertiesscopeinit)

#### Implementations
##### `flags`
<pre><code class="api-signature language-rust ignore">pub fn flags(&amp;self) -&gt; &amp;<a href="../../../api/internal/pax-runtime/rendering.md#instanceflags">InstanceFlags</a></code></pre>

Static behavior flags for this instance.

##### `get_handler_registry`
<pre><code class="api-signature language-rust ignore">pub fn get_handler_registry(&amp;self) -&gt; Option&lt;Rc&lt;RefCell&lt;<a href="../../../api/internal/pax-runtime/engine.md#handlerregistry">HandlerRegistry</a>&gt;&gt;&gt;</code></pre>

Returns a handle to a node-managed HandlerRegistry, a mapping between event types and handlers.
Each node that can handle events is responsible for implementing this; Component instances generate
the necessary code to wire up userland events like `<SomeNode @click=self.handler>`. Primitives must handle
this explicitly, see e.g. `[pax_std::drawing::rectangle::RectangleInstance#get_handler_registry]`.

##### `get_instance_children`
<pre><code class="api-signature language-rust ignore">pub fn get_instance_children(&amp;self) -&gt; &amp;InstanceNodePtrList</code></pre>

Return the list of instance nodes that are children of this one. Intuitively, this returns
the nodes owned directly by this instance's definition.

For `Component`s, this returns the root(s) of the component template, not the
projected children supplied by the containing component.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(args: <a href="../../../api/internal/pax-runtime/rendering.md#instantiationargs">InstantiationArgs</a>, flags: <a href="../../../api/internal/pax-runtime/rendering.md#instanceflags">InstanceFlags</a>) -&gt; Self</code></pre>

Build shared instance state from compiler-generated instantiation args.

---

### `InstanceFlags`
Static traversal/rendering flags for a concrete instance node.

#### Properties
##### `invisible_to_slot`
Type: `bool`

Used for exotic tree traversals for `Slot`, e.g. for `Stacker` > `Repeat` > `Rectangle`
where the repeated `Rectangle`s need to be be considered direct children of `Stacker`.
`Repeat` and `Conditional` set this true so their active children can be
considered direct projected children by slot-driven containers.

##### `invisible_to_raycasting`
Type: `bool`

Certain elements, such as Groups and Components, are invisible to ray-casting.
Since these container elements are on top of the elements they contain,
this is needed otherwise the containers would intercept rays that should hit their contents.

##### `layer`
Type: [`Layer`](../../../api/pax-runtime-api/rendering.md#layer)

The layer type (`Layer::Native`, `Layer::NativeNonOccluding`, or `Layer::Canvas`)
for this RenderNode.
Default is `Layer::Canvas`, and must be overwritten for `InstanceNode`s that manage native
content.

##### `is_component`
Type: `bool`

Only true for ComponentInstance

##### `is_slot`
Type: `bool`

Is this node a `Slot`?

---

### `InstantiationArgs`
Construction payload used when compiler-generated code instantiates an `InstanceNode`.

#### Properties
##### `prototypical_common_properties`
Type: [`CommonPropertiesInit`](../../../api/internal/pax-runtime/rendering.md#commonpropertiesinit)

##### `prototypical_properties`
Type: [`PropertiesInit`](../../../api/internal/pax-runtime/rendering.md#propertiesinit)

##### `handler_registry`
Type: `Option`<`Rc`<`RefCell`<[`HandlerRegistry`](../../../api/internal/pax-runtime/engine.md#handlerregistry)>>>

##### `children`
Type: `Option`<`InstanceNodePtrList`>

##### `component_template`
Type: `Option`<`InstanceNodePtrList`>

##### `component_settings`
Type: `Option`<`Vec`<[`SettingsBlockElement`](../../../api/internal/pax-manifest/index.md#settingsblockelement)>>

##### `template_node_identifier`
Type: `Option`<[`UniqueTemplateNodeIdentifier`](../../../api/internal/pax-manifest/index.md#uniquetemplatenodeidentifier)>

##### `template_node_type_id`
Type: `Option`<[`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)>

##### `template_node_selector_info`
Type: `Option`<[`TemplateNodeSelectorInfo`](../../../api/internal/pax-manifest/selectors.md#templatenodeselectorinfo)>

##### `transition_config`
Type: `ComponentTransitionConfig`

##### `properties_scope`
Type: [`PropertiesScopeInit`](../../../api/internal/pax-runtime/rendering.md#propertiesscopeinit)

---

### `ReusableInstanceNodeArgs`
Lightweight clone of reusable base-node data for helper constructors.

#### Properties
##### `handler_registry`
Type: `Option`<`Rc`<`RefCell`<[`HandlerRegistry`](../../../api/internal/pax-runtime/engine.md#handlerregistry)>>>

##### `children`
Type: `InstanceNodePtrList`

##### `template_node_identifier`
Type: `Option`<[`UniqueTemplateNodeIdentifier`](../../../api/internal/pax-manifest/index.md#uniquetemplatenodeidentifier)>

##### `template_node_type_id`
Type: `Option`<[`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)>

##### `template_node_selector_info`
Type: `Option`<[`TemplateNodeSelectorInfo`](../../../api/internal/pax-manifest/selectors.md#templatenodeselectorinfo)>

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(base: &amp;<a href="../../../api/internal/pax-runtime/rendering.md#baseinstance">BaseInstance</a>) -&gt; Self</code></pre>

Capture the reusable portions of a `BaseInstance`.

---

### `StrokeInstance`
Resolved stroke style used by canvas drawing primitives.

#### Properties
##### `color`
Type: [`Color`](../../../api/pax-runtime-api/color.md#color)

##### `width`
Type: `f64`

##### `style`
Type: `StrokeStyle`

## Enums
### `CommonPropertiesInit`
Structured initialization for node-local common properties.

#### Variants
##### `Default`
##### `Inline` { `defined_properties`: `BTreeMap`<`String`, [`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)> }
##### `Template`(`Rc`<[`TemplatePropertyPlan`](../../../api/internal/pax-runtime/cartridge.md#templatepropertyplan)>)
##### `Factory`(`CommonPropertiesFactory`)
---

### `NodeType`
Coarse runtime category for an instance node.

#### Variants
##### `Component`
##### `Primitive`
---

### `PropertiesInit`
Structured initialization for node-local typed properties.

#### Variants
##### `DescriptorDefault`(&'`static` `ErasedComponentDescriptor`)
##### `DescriptorInline` { `descriptor`: &'`static` `ErasedComponentDescriptor`, `defined_properties`: `BTreeMap`<`String`, [`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)> }
##### `Template` { `descriptor`: &'`static` `ErasedComponentDescriptor`, `plan`: `Rc`<[`TemplatePropertyPlan`](../../../api/internal/pax-runtime/cartridge.md#templatepropertyplan)> }
##### `Factory`(`PropertiesFactory`)
---

### `PropertiesScopeInit`
How an expanded node should expose component-local symbols into scope.

#### Variants
##### `None`
##### `Descriptor`(&'`static` `ErasedComponentDescriptor`)
##### `Factory`(`PropertiesScopeFactory`)
## Traits
### `InstanceNode`
Central runtime representation of a properties-computable and renderable node.
`InstanceNode`s are conceptually stateless, and rely on [`ExpandedNode`]s for stateful representations.

An `InstanceNode` sits in between a `pax_compiler::TemplateNodeDefinition`, the
compile-time `definition` analogue to this `instance`, and [`ExpandedNode`].

There is a 1:1 relationship between `pax_compiler::TemplateNodeDefinition`s and `InstanceNode`s.
There is a one-to-many relationship between one `InstanceNode` and possibly many variant [`ExpandedNode`]s,
due to duplication via `for`.

`InstanceNode`s are architecturally "type-aware" — they can perform type-specific operations e.g. on the state stored in [`ExpandedNode`], while
[`ExpandedNode`]s are "type-blind".  The latter store polymorphic data but cannot operate on it without the type-aware assistance of their linked `InstanceNode`.

(See `RepeatInstance::expand_node` where we visit a singular `InstanceNode` several times, producing multiple [`ExpandedNode`]s.)

## Type Aliases
### `InstanceNodePtr`
Type aliases to make it easier to work with nested Rcs and
RefCells for instance nodes.
