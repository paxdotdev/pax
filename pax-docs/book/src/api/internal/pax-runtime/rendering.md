# rendering
<!-- summary: API docs for pax-runtime::rendering. -->
<!-- tags: api, pax-runtime -->

## Structs
### `BaseInstance`
Shared storage carried by every concrete `InstanceNode`.

#### Properties
##### `handler_registry`
Type: `Option`<`Rc`<`RefCell`<[`HandlerRegistry`](/api/internal/pax-runtime/engine.md#handlerregistry)>>>

##### `instance_prototypical_properties_factory`
Type: `Box`<`dyn` `Fn`(`Rc`<[`RuntimePropertiesStackFrame`](/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe)>, `Option`<`Rc`<`ExpandedNode`>>) -> `Option`<`Rc`<`RefCell`<[`PaxAny`](/api/pax-runtime-api/pax_value.md#paxany)>>>>

##### `instance_prototypical_common_properties_factory`
Type: `Box`<`dyn` `Fn`(`Rc`<[`RuntimePropertiesStackFrame`](/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe)>, `Option`<`Rc`<`ExpandedNode`>>) -> `Option`<`Rc`<`RefCell`<[`CommonProperties`](/api/pax-runtime-api/layout.md#commonproperties)>>>>

##### `template_node_identifier`
Type: `Option`<[`UniqueTemplateNodeIdentifier`](/api/internal/pax-manifest/index.md#uniquetemplatenodeidentifier)>

##### `properties_scope_factory`
Type: `Option`<`Box`<`dyn` `Fn`(`Rc`<`RefCell`<[`PaxAny`](/api/pax-runtime-api/pax_value.md#paxany)>>) -> `HashMap`<`String`, [`Variable`](/api/pax-runtime-api/variables.md#variable)>>>

#### Implementations
##### `flags`
<pre><code class="api-signature language-rust ignore">pub fn flags(&amp;self) -&gt; &amp;<a href="/api/internal/pax-runtime/rendering.md#instanceflags">InstanceFlags</a></code></pre>

Static behavior flags for this instance.

##### `get_handler_registry`
<pre><code class="api-signature language-rust ignore">pub fn get_handler_registry(&amp;self) -&gt; Option&lt;Rc&lt;RefCell&lt;<a href="/api/internal/pax-runtime/engine.md#handlerregistry">HandlerRegistry</a>&gt;&gt;&gt;</code></pre>

Returns a handle to a node-managed HandlerRegistry, a mapping between event types and handlers.
Each node that can handle events is responsible for implementing this; Component instances generate
the necessary code to wire up userland events like `<SomeNode @click=self.handler>`. Primitives must handle
this explicitly, see e.g. `[pax_std::drawing::rectangle::RectangleInstance#get_handler_registry]`.

##### `get_instance_children`
<pre><code class="api-signature language-rust ignore">pub fn get_instance_children(&amp;self) -&gt; &amp;<a href="/api/internal/pax-runtime/rendering.md#instancenodeptrlist">InstanceNodePtrList</a></code></pre>

Return the list of instance nodes that are children of this one.  Intuitively, this will return
instance nodes mapping exactly to the template node definitions.
For `Component`s, `get_instance_children` returns the root(s) of its template, not its `slot_children`.
(see `get_slot_children` for the way to retrieve the latter.)

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(args: <a href="/api/internal/pax-runtime/rendering.md#instantiationargs">InstantiationArgs</a>, flags: <a href="/api/internal/pax-runtime/rendering.md#instanceflags">InstanceFlags</a>) -&gt; Self</code></pre>

Build shared instance state from compiler-generated instantiation args.

---

### `InstanceFlags`
Static traversal/rendering flags for a concrete instance node.

#### Properties
##### `invisible_to_slot`
Type: `bool`

Used for exotic tree traversals for `Slot`, e.g. for `Stacker` > `Repeat` > `Rectangle`
where the repeated `Rectangle`s need to be be considered direct children of `Stacker`.
`Repeat` and `Conditional` override `is_invisible_to_slot` to return true

##### `invisible_to_raycasting`
Type: `bool`

Certain elements, such as Groups and Components, are invisible to ray-casting.
Since these container elements are on top of the elements they contain,
this is needed otherwise the containers would intercept rays that should hit their contents.

##### `layer`
Type: [`Layer`](/api/pax-runtime-api/rendering.md#layer)

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
##### `prototypical_common_properties_factory`
Type: `Box`<`dyn` `Fn`(`Rc`<[`RuntimePropertiesStackFrame`](/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe)>, `Option`<`Rc`<`ExpandedNode`>>) -> `Option`<`Rc`<`RefCell`<[`CommonProperties`](/api/pax-runtime-api/layout.md#commonproperties)>>>>

##### `prototypical_properties_factory`
Type: `Box`<`dyn` `Fn`(`Rc`<[`RuntimePropertiesStackFrame`](/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe)>, `Option`<`Rc`<`ExpandedNode`>>) -> `Option`<`Rc`<`RefCell`<[`PaxAny`](/api/pax-runtime-api/pax_value.md#paxany)>>>>

##### `handler_registry`
Type: `Option`<`Rc`<`RefCell`<[`HandlerRegistry`](/api/internal/pax-runtime/engine.md#handlerregistry)>>>

##### `children`
Type: `Option`<[`InstanceNodePtrList`](/api/internal/pax-runtime/rendering.md#instancenodeptrlist)>

##### `component_template`
Type: `Option`<[`InstanceNodePtrList`](/api/internal/pax-runtime/rendering.md#instancenodeptrlist)>

##### `template_node_identifier`
Type: `Option`<[`UniqueTemplateNodeIdentifier`](/api/internal/pax-manifest/index.md#uniquetemplatenodeidentifier)>

##### `properties_scope_factory`
Type: `Option`<`Box`<`dyn` `Fn`(`Rc`<`RefCell`<[`PaxAny`](/api/pax-runtime-api/pax_value.md#paxany)>>) -> `HashMap`<`String`, [`Variable`](/api/pax-runtime-api/variables.md#variable)>>>

---

### `ReusableInstanceNodeArgs`
Lightweight clone of reusable base-node data for helper constructors.

#### Properties
##### `handler_registry`
Type: `Option`<`Rc`<`RefCell`<[`HandlerRegistry`](/api/internal/pax-runtime/engine.md#handlerregistry)>>>

##### `children`
Type: [`InstanceNodePtrList`](/api/internal/pax-runtime/rendering.md#instancenodeptrlist)

##### `template_node_identifier`
Type: `Option`<[`UniqueTemplateNodeIdentifier`](/api/internal/pax-manifest/index.md#uniquetemplatenodeidentifier)>

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(base: &amp;<a href="/api/internal/pax-runtime/rendering.md#baseinstance">BaseInstance</a>) -&gt; Self</code></pre>

Capture the reusable portions of a `BaseInstance`.

---

### `StrokeInstance`
Resolved stroke style used by canvas drawing primitives.

#### Properties
##### `color`
Type: [`Color`](/api/pax-runtime-api/color.md#color)

##### `width`
Type: `f64`

##### `style`
Type: `StrokeStyle`

## Enums
### `NodeType`
Coarse runtime category for an instance node.

#### Variants
##### `Component`
##### `Primitive`
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
