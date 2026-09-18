# pax-manifest
<!-- summary: API reference for pax-manifest. -->
<!-- tags: api, pax-manifest -->

## Submodules
- [program_ir](program_ir.md)
- [selectors](selectors.md)
- [cartridge_generation](cartridge_generation.md)

## Structs
### `ComponentDefinition`
Container for an entire component definition — includes template, settings,
event bindings, property definitions, and compiler + reflection metadata

#### Properties
##### `type_id`
Type: [`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)

##### `is_main_component`
Type: `bool`

##### `is_primitive`
Type: `bool`

##### `is_struct_only_component`
Type: `bool`

Flag describing whether this component definition is a "struct-only component", a
struct decorated with `#[pax]` for use as the `T` in `Property<T>`.

##### `module_path`
Type: `String`

##### `primitive_instance_import_path`
Type: `Option`<`String`>

For primitives like Rectangle or Group, a separate import
path is required for the Instance (render context) struct
and the Definition struct.  For primitives, then, we need
to store an additional import path to use when instantiating.

##### `template`
Type: `Option`<[`ComponentTemplate`](../../../api/internal/pax-manifest/index.md#componenttemplate)>

##### `settings`
Type: `Option`<`Vec`<[`SettingsBlockElement`](../../../api/internal/pax-manifest/index.md#settingsblockelement)>>

##### `timelines`
Type: `Vec`<[`TimelineDefinition`](../../../api/internal/pax-manifest/index.md#timelinedefinition)>

##### `route_branch`
Type: `Option`<[`RouteBranchDescriptor`](../../../api/internal/pax-manifest/index.md#routebranchdescriptor)>

---

### `ComponentTemplate`
#### Implementations
##### `from_parts`
<pre><code class="api-signature language-rust ignore">pub fn from_parts(containing_component: <a href="../../../api/internal/pax-manifest/index.md#typeid">TypeId</a>, root: VecDeque&lt;<a href="../../../api/internal/pax-manifest/index.md#templatenodeid">TemplateNodeId</a>&gt;, children: HashMap&lt;<a href="../../../api/internal/pax-manifest/index.md#templatenodeid">TemplateNodeId</a>, VecDeque&lt;<a href="../../../api/internal/pax-manifest/index.md#templatenodeid">TemplateNodeId</a>&gt;&gt;, nodes: HashMap&lt;<a href="../../../api/internal/pax-manifest/index.md#templatenodeid">TemplateNodeId</a>, <a href="../../../api/internal/pax-manifest/index.md#templatenodedefinition">TemplateNodeDefinition</a>&gt;, next_id: usize, template_source_file_path: Option&lt;String&gt;) -&gt; Self</code></pre>

Construct a component template from already-materialized storage.

---

### `ControlFlowRouteBranchDefinition`
#### Properties
##### `metadata`
Type: `Option`<`RouteMetadataDefinition`>

Compiler-only document metadata attached to this declarative route.

The web compiler consumes this before cartridge generation. Runtime
routing and baked program representations intentionally omit it.

---

### `ExpressionCompilationInfo`
#### Properties
##### `dependencies`
Type: `Vec`<`String`>

symbols used in the expression

---

### `GradientDefinition`
Compile-time representation of an inline `@gradient` value.

#### Properties
##### `shape`
Type: [`GradientShapeDefinition`](../../../api/internal/pax-manifest/index.md#gradientshapedefinition)

##### `elements`
Type: `Vec`<[`GradientElement`](../../../api/internal/pax-manifest/index.md#gradientelement)>

#### Implementations
##### `stops`
<pre><code class="api-signature language-rust ignore">pub fn stops(&amp;self) -&gt; impl Iterator</code></pre>

Iterate only stop entries, skipping comments.

---

### `GradientStopDefinition`
A single color stop in a gradient ramp.

#### Properties
##### `position`
Type: [`Size`](../../../api/pax-runtime-api/layout.md#size)

##### `color`
Type: [`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)

---

### `HostCrateInfo`
Pulled from host Cargo.toml

#### Properties
##### `name`
Type: `String`

for example: `pax-example`

##### `identifier`
Type: `String`

for example: `pax_example`

##### `import_prefix`
Type: `String`

for example: `some_crate::pax_reexports`,

---

### `LiteralBlockDefinition`
Container for a parsed Literal object

#### Properties
##### `explicit_type_pascal_identifier`
Type: `Option`<[`Token`](../../../api/internal/pax-manifest/index.md#token)>

##### `elements`
Type: `Vec`<[`SettingElement`](../../../api/internal/pax-manifest/index.md#settingelement)>

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(elements: Vec&lt;<a href="../../../api/internal/pax-manifest/index.md#settingelement">SettingElement</a>&gt;) -&gt; Self</code></pre>

Construct a literal object block from setting elements.

##### `get_all_settings`
<pre><code class="api-signature language-rust ignore">pub fn get_all_settings&lt;&#39;a&gt;(&amp;&#39;a self) -&gt; Vec&lt;(&amp;&#39;a <a href="../../../api/internal/pax-manifest/index.md#token">Token</a>, &amp;&#39;a <a href="../../../api/internal/pax-manifest/index.md#valuedefinition">ValueDefinition</a>)&gt;</code></pre>

Return only actual setting entries, omitting comments.

---

### `LocationInfo`
Container for holding metadata about original Location in Pax Template
Used for source-mapping

#### Properties
##### `start_line_col`
Type: (`usize`, `usize`)

##### `end_line_col`
Type: (`usize`, `usize`)

---

### `NodeLocation`
Full editable location metadata for a template node.

#### Properties
##### `type_id`
Type: [`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)

##### `tree_location`
Type: [`TreeLocation`](../../../api/internal/pax-manifest/index.md#treelocation)

##### `index`
Type: [`TreeIndexPosition`](../../../api/internal/pax-manifest/index.md#treeindexposition)

#### Implementations
##### `get_tree_location`
<pre><code class="api-signature language-rust ignore">pub fn get_tree_location(&amp;self) -&gt; &amp;<a href="../../../api/internal/pax-manifest/index.md#treelocation">TreeLocation</a></code></pre>

Parent-location component of this node location.

##### `get_type_id`
<pre><code class="api-signature language-rust ignore">pub fn get_type_id(&amp;self) -&gt; &amp;<a href="../../../api/internal/pax-manifest/index.md#typeid">TypeId</a></code></pre>

Type id of the node at this location.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(type_id: <a href="../../../api/internal/pax-manifest/index.md#typeid">TypeId</a>, location: <a href="../../../api/internal/pax-manifest/index.md#treelocation">TreeLocation</a>, index: <a href="../../../api/internal/pax-manifest/index.md#treeindexposition">TreeIndexPosition</a>) -&gt; Self</code></pre>

Construct a node location.

---

### `PaxManifest`
Definition container for an entire Pax cartridge

#### Properties
##### `components`
Type: `BTreeMap`<[`TypeId`](../../../api/internal/pax-manifest/index.md#typeid), [`ComponentDefinition`](../../../api/internal/pax-manifest/index.md#componentdefinition)>

##### `main_component_type_id`
Type: [`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)

##### `type_table`
Type: `TypeTable`

##### `assets_dirs`
Type: `Vec`<`String`>

Compiler metadata: list of fully qualified asset directories, gathered during compiletime,
from which assets will be copied for bundling into executable binaries

##### `engine_import_path`
Type: `String`

Compiler metadata: the import prefix for the engine module, `pax_kit::pax_engine` by default,
but parameterizable for crates such as `pax-std` that integrate with `pax-engine` directly.

---

### `PropertyDefinition`
#### Properties
##### `name`
Type: `String`

String representation of the symbolic identifier of a declared Property

##### `flags`
Type: [`PropertyDefinitionFlags`](../../../api/internal/pax-manifest/index.md#propertydefinitionflags)

Flags, used ultimately by ExpressionSpecInvocations, to denote
e.g. whether a property is the `i` or `elem` of a `Repeat`, which allows
for special-handling the RIL that invokes these values

##### `type_id`
Type: [`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)

Statically known type_id for this Property's associated TypeDefinition

#### Implementations
Describes static metadata surrounding a property, for example
the string representation of the property's name and a `TypeInfo`
entry for the property's statically discovered type

##### `primitive_with_name`
<pre><code class="api-signature language-rust ignore">pub fn primitive_with_name(type_name: &amp;str, symbol_name: &amp;str) -&gt; Self</code></pre>

Shorthand factory / constructor

---

### `PropertyDefinitionFlags`
These flags describe the aspects of properties that affect RIL codegen.
Properties are divided into modal axes (exactly one value should be true per axis per struct instance)
Codegen considers each element of the cartesian product of these axes

#### Properties
##### `is_binding_repeat_i`
Type: `bool`

Does this property represent the index `i` in `for (elem, i)` ?

##### `is_binding_repeat_elem`
Type: `bool`

Does this property represent `elem` in `for (elem, i)` OR `for elem in 0..5` ?

##### `is_repeat_source_range`
Type: `bool`

Is the source being iterated over a Range?

##### `is_repeat_source_iterable`
Type: `bool`

Is the source being iterated over an iterable, like `Vec<T>`?

##### `is_property_wrapped`
Type: `bool`

Describes whether this property is a `Property`-wrapped `T` in `Property<T>`
This distinction affects our ability to dirty-watch a particular property, and
has implications on codegen

##### `is_enum`
Type: `bool`

Describes whether this property is an enum variant property

---

### `RouteBranchDescriptor`
Compile-time contract allowing a component or primitive to act as a direct
route branch child of `Router`.

#### Properties
##### `path_property`
Type: `String`

##### `default_property`
Type: `String`

##### `modal`
Type: `bool`

---

### `SettingsConditionalBlock`
Top-level conditional content inside a settings block.

#### Properties
##### `branches`
Type: `Vec`<[`SettingsConditionalBranch`](../../../api/internal/pax-manifest/index.md#settingsconditionalbranch)>

---

### `SettingsConditionalBranch`
One branch inside a settings conditional. `None` represents `else`.

#### Properties
##### `condition_expression`
Type: `Option`<`ExpressionInfo`>

##### `elements`
Type: `Vec`<[`SettingsBlockElement`](../../../api/internal/pax-manifest/index.md#settingsblockelement)>

---

### `TemplateNodeDefinition`
Represents an entry within a component template, e.g. a `<Rectangle>` declaration inside a template
Each node in a template is represented by exactly one `TemplateNodeDefinition`, and this is a compile-time
concern.  Note the difference between compile-time `definitions` and runtime `instances`.
A compile-time `TemplateNodeDefinition` corresponds to a single runtime `RenderNode` instance.

#### Properties
##### `type_id`
Type: [`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)

Reference to the unique string ID for a component, e.g. `primitive::Frame` or `component::Stacker`

##### `control_flow_settings`
Type: `Option`<`ControlFlowSettingsDefinition`>

Iff this TND is a control-flow node: parsed control flow attributes (slot/if/for)

##### `settings`
Type: `Option`<`Vec`<[`SettingElement`](../../../api/internal/pax-manifest/index.md#settingelement)>>

IFF this TND is NOT a control-flow node: parsed key-value store of attribute definitions (like `some_key="some_value"`)

##### `selector_info`
Type: [`TemplateNodeSelectorInfo`](../../../api/internal/pax-manifest/selectors.md#templatenodeselectorinfo)

Normalized selector metadata preserved for runtime/designtime matching.

##### `raw_comment_string`
Type: `Option`<`String`>

IFF this TND is a comment node: raw comment string

---

### `TemplateNodeId`
Stable id for a template node inside a single component.

#### Implementations
##### `as_usize`
<pre><code class="api-signature language-rust ignore">pub fn as_usize(&amp;self) -&gt; usize</code></pre>

Numeric index backing this id.

##### `build`
<pre><code class="api-signature language-rust ignore">pub fn build(id: usize) -&gt; Self</code></pre>

Construct a template node id from its numeric index.

---

### `TimelineDefinition`
Compile-time representation of a declared timeline.

#### Properties
##### `name`
Type: `Option`<[`Token`](../../../api/internal/pax-manifest/index.md#token)>

##### `playhead`
Type: `Option`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>

##### `duration`
Type: `Option`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>

##### `repeat`
Type: `bool`

##### `interruption`
Type: [`InOutInterruption`](../../../api/internal/pax-manifest/index.md#inoutinterruption)

##### `elements`
Type: `Vec`<[`TimelineBlockElement`](../../../api/internal/pax-manifest/index.md#timelineblockelement)>

---

### `TimelineKeyframe`
A single timeline value at a frame, duration, or percent marker.

#### Properties
##### `marker`
Type: [`TimelineMarker`](../../../api/internal/pax-manifest/index.md#timelinemarker)

##### `value`
Type: [`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)

##### `easing`
Type: `Option`<[`Token`](../../../api/internal/pax-manifest/index.md#token)>

---

### `TimelineSelectorBlockDefinition`
Selector body inside a timeline block.

#### Properties
##### `elements`
Type: `Vec`<[`TimelineSelectorElement`](../../../api/internal/pax-manifest/index.md#timelineselectorelement)>

---

### `TimelineTrackDefinition`
Track-level timeline data for one animated property.

#### Properties
##### `elements`
Type: `Vec`<[`TimelineTrackElement`](../../../api/internal/pax-manifest/index.md#timelinetrackelement)>

##### `playhead`
Type: `Option`<`Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>>

##### `duration`
Type: `Option`<`Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>>

##### `repeat`
Type: `Option`<`bool`>

##### `starting_value`
Type: `Option`<`Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>>

##### `interruption`
Type: [`InOutInterruption`](../../../api/internal/pax-manifest/index.md#inoutinterruption)

##### `use_local_property_scope`
Type: `bool`

#### Implementations
##### `keyframes`
<pre><code class="api-signature language-rust ignore">pub fn keyframes(&amp;self) -&gt; impl Iterator</code></pre>

Iterate only keyframe entries, skipping comments.

---

### `Token`
Container for parsed values with optional location information
Location is optional in case this token was generated dynamically

#### Properties
##### `token_value`
Type: `String`

##### `token_location`
Type: `Option`<[`LocationInfo`](../../../api/internal/pax-manifest/index.md#locationinfo)>

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(token_value: String, token_location: <a href="../../../api/internal/pax-manifest/index.md#locationinfo">LocationInfo</a>) -&gt; Self</code></pre>

Construct a token with source location information.

##### `new_without_location`
<pre><code class="api-signature language-rust ignore">pub fn new_without_location(token_value: String) -&gt; Self</code></pre>

Construct a token synthesized without a source location.

---

### `TransitionDefinition`
Pair of timeline tracks bound to a node's enter/exit lifecycle.

#### Properties
##### `enter`
Type: `Option`<[`TimelineTrackDefinition`](../../../api/internal/pax-manifest/index.md#timelinetrackdefinition)>

##### `exit`
Type: `Option`<[`TimelineTrackDefinition`](../../../api/internal/pax-manifest/index.md#timelinetrackdefinition)>

##### `starting_value`
Type: `Option`<`Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>>

---

### `TypeDefinition`
Describes metadata surrounding a property's type, gathered from a combination of static & dynamic analysis

#### Properties
##### `type_id`
Type: [`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)

Program-unique ID for this type

##### `inner_iterable_type_id`
Type: `Option`<[`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)>

Statically known type_id for this Property's iterable TypeDefinition, that is,
T for some `Property<Vec<T>>`

##### `property_definitions`
Type: `Vec`<[`PropertyDefinition`](../../../api/internal/pax-manifest/index.md#propertydefinition)>

A vec of PropertyType, describing known addressable (sub-)properties of this PropertyType

#### Implementations
##### `builtin_vec_rc_ref_cell_any_properties`
<pre><code class="api-signature language-rust ignore">pub fn builtin_vec_rc_ref_cell_any_properties(inner_iterable_type_id: <a href="../../../api/internal/pax-manifest/index.md#typeid">TypeId</a>) -&gt; Self</code></pre>

Used by Repeat for source expressions, e.g. the `self.some_vec` in `for elem in self.some_vec`

---

### `TypeId`
#### Implementations
##### `build_blank_component`
<pre><code class="api-signature language-rust ignore">pub fn build_blank_component(pascal_identifier: &amp;str) -&gt; Self</code></pre>

Build a typeid for a transient component

##### `build_map`
<pre><code class="api-signature language-rust ignore">pub fn build_map(key_identifier: &amp;str, value_identifier: &amp;str) -&gt; Self</code></pre>

Build a TypeId for map types like `std::collections::HashMap<String><Color>`

##### `build_option`
<pre><code class="api-signature language-rust ignore">pub fn build_option(identifier: &amp;str) -&gt; Self</code></pre>

Build a TypeId for option types like `std::option::Option<Color>`

##### `build_primitive`
<pre><code class="api-signature language-rust ignore">pub fn build_primitive(identifier: &amp;str) -&gt; Self</code></pre>

Build a TypeId for rust primitives like `u8` or `String`

##### `build_range`
<pre><code class="api-signature language-rust ignore">pub fn build_range(identifier: &amp;str) -&gt; Self</code></pre>

Build a TypeId for range types like `std::ops::Range<Color>`

##### `build_singleton`
<pre><code class="api-signature language-rust ignore">pub fn build_singleton(import_path: &amp;str, pascal_identifier: Option&lt;&amp;str&gt;) -&gt; Self</code></pre>

Build a TypeId for a most types, like `Stacker` or `SpecialComponent`

##### `build_vector`
<pre><code class="api-signature language-rust ignore">pub fn build_vector(elem_identifier: &amp;str) -&gt; Self</code></pre>

Build a TypeId for vector types like `Vec<Color>`

---

### `UniqueTemplateNodeIdentifier`
Globally unique identity for a template node: component type plus local template-node id.

#### Implementations
##### `build`
<pre><code class="api-signature language-rust ignore">pub fn build(component: <a href="../../../api/internal/pax-manifest/index.md#typeid">TypeId</a>, template_node_id: <a href="../../../api/internal/pax-manifest/index.md#templatenodeid">TemplateNodeId</a>) -&gt; Self</code></pre>

Construct a globally unique template-node id.

##### `get_containing_component_type_id`
<pre><code class="api-signature language-rust ignore">pub fn get_containing_component_type_id(&amp;self) -&gt; <a href="../../../api/internal/pax-manifest/index.md#typeid">TypeId</a></code></pre>

Component that owns this template node.

##### `get_template_node_id`
<pre><code class="api-signature language-rust ignore">pub fn get_template_node_id(&amp;self) -&gt; <a href="../../../api/internal/pax-manifest/index.md#templatenodeid">TemplateNodeId</a></code></pre>

Node id within the containing component template.

## Enums
### `ControlFlowConditionalBranchKind`
Container for storing parsed control flow information, for
example the string (PAXEL) representations of condition / slot / repeat
expressions and the related vtable ids (for "punching" during expression compilation)

#### Variants
##### `If`
##### `ElseIf`
##### `Else`
---

### `ControlFlowRepeatPredicateDefinition`
Container for holding parsed data describing a Repeat (`for`)
predicate, for example the `(elem, i)` in `for (elem, i) in foo` or
the `elem` in `for elem in foo`

#### Variants
##### `ElemId`(`String`)
##### `ElemIdIndexId`(`String`, `String`)
---

### `GradientElement`
One entry inside a gradient block.

#### Variants
##### `Stop`([`GradientStopDefinition`](../../../api/internal/pax-manifest/index.md#gradientstopdefinition))
##### `Comment`(`String`)
---

### `GradientShapeDefinition`
Shape-specific parameters for a gradient. V1 maps directly to runtime `Fill` variants.

#### Variants
##### `Linear` { `start`: `Option`<`Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>>, `end`: `Option`<`Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>> }
##### `Radial` { `start`: `Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>, `end`: `Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)>, `radius`: `Box`<[`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition)> }
---

### `InOutInterruption`
Controls how an `@in` or `@out` timeline begins when it directly reverses
the other lifecycle transition on the same mounted instance.

#### Variants
##### `Takeover`
Continue from the property's currently sampled value.

##### `Restart`
Begin from the destination timeline's authored starting value.

---

### `Number`
Parsed numeric literal before final type coercion.

#### Variants
##### `Float`(`f64`)
##### `Int`(`isize`)
---

### `PaxType`
Manifest-level type identity category.

#### Variants
##### `If`
##### `Router`
##### `Slot`
##### `Repeat`
##### `Comment`
##### `BlankComponent` { `pascal_identifier`: `String` }
##### `Primitive` { `pascal_identifier`: `String` }
##### `Singleton` { `pascal_identifier`: `String` }
##### `Range` { `identifier`: `String` }
##### `Option` { `identifier`: `String` }
##### `Vector` { `elem_identifier`: `String` }
##### `Map` { `key_identifier`: `String`, `value_identifier`: `String` }
##### `Unknown`
---

### `SettingElement`
One key/value or comment entry inside a literal block.

#### Variants
##### `Setting`([`Token`](../../../api/internal/pax-manifest/index.md#token), [`ValueDefinition`](../../../api/internal/pax-manifest/index.md#valuedefinition))
##### `Comment`(`String`)
---

### `SettingsBlockElement`
One entry inside a settings block.

#### Variants
##### `SelectorBlock`([`Token`](../../../api/internal/pax-manifest/index.md#token), [`LiteralBlockDefinition`](../../../api/internal/pax-manifest/index.md#literalblockdefinition))
##### `Handler`([`Token`](../../../api/internal/pax-manifest/index.md#token), `Vec`<[`Token`](../../../api/internal/pax-manifest/index.md#token)>)
##### `Transition`([`Token`](../../../api/internal/pax-manifest/index.md#token), [`Token`](../../../api/internal/pax-manifest/index.md#token))
##### `Conditional`([`SettingsConditionalBlock`](../../../api/internal/pax-manifest/index.md#settingsconditionalblock))
##### `Comment`(`String`)
---

### `TimelineBlockElement`
One entry inside a timeline block.

#### Variants
##### `SelectorBlock`([`Token`](../../../api/internal/pax-manifest/index.md#token), [`TimelineSelectorBlockDefinition`](../../../api/internal/pax-manifest/index.md#timelineselectorblockdefinition))
##### `Comment`(`String`)
---

### `TimelineMarker`
Timeline position expressed as an absolute frame, absolute duration, or normalized percentage.

#### Variants
##### `Frame`(`u64`)
##### `Duration`([`Duration`](../../../api/pax-runtime-api/animation.md#duration))
##### `Percent`(`f64`)
---

### `TimelineSelectorElement`
One selector-scoped element inside a timeline block.

#### Variants
##### `Track`([`Token`](../../../api/internal/pax-manifest/index.md#token), [`TimelineTrackDefinition`](../../../api/internal/pax-manifest/index.md#timelinetrackdefinition))
##### `Comment`(`String`)
---

### `TimelineTrackElement`
One entry in a timeline track.

#### Variants
##### `Keyframe`([`TimelineKeyframe`](../../../api/internal/pax-manifest/index.md#timelinekeyframe))
##### `Comment`(`String`)
---

### `TreeIndexPosition`
Desired insertion position among siblings.

#### Variants
##### `Top`
##### `Bottom`
##### `At`(`usize`)
#### Implementations
##### `get_index`
<pre><code class="api-signature language-rust ignore">pub fn get_index(&amp;self, len: usize) -&gt; usize</code></pre>

Resolve this symbolic position against a sibling-list length.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(index: usize) -&gt; Self</code></pre>

Construct an explicit index position.

---

### `TreeLocation`
Parent relationship for a node inside a component template tree.

#### Variants
##### `Root`
##### `Parent`([`TemplateNodeId`](../../../api/internal/pax-manifest/index.md#templatenodeid))
---

### `Unit`
Parsed Pax unit suffix.

#### Variants
##### `Pixels`
##### `Percent`
---

### `ValueDefinition`
Container for settings values, storing all possible
variants, populated at parse-time and used at compile-time

#### Variants
##### `Undefined`
##### `LiteralValue`([`PaxValue`](../../../api/pax-runtime-api/pax_value.md#paxvalue))
##### `Block`([`LiteralBlockDefinition`](../../../api/internal/pax-manifest/index.md#literalblockdefinition))
##### `Timeline`([`TimelineTrackDefinition`](../../../api/internal/pax-manifest/index.md#timelinetrackdefinition))
##### `Gradient`([`GradientDefinition`](../../../api/internal/pax-manifest/index.md#gradientdefinition))
##### `Transition`([`TransitionDefinition`](../../../api/internal/pax-manifest/index.md#transitiondefinition))
##### `Expression`(`ExpressionInfo`)
(Expression contents, vtable id binding)

##### `Identifier`([`PaxIdentifier`](../../../api/internal/pax-language/interpreter.md#paxidentifier))
(Expression contents, vtable id binding)

##### `DoubleBinding`([`PaxIdentifier`](../../../api/internal/pax-language/interpreter.md#paxidentifier))
(Expression contents, vtable id binding)

##### `EventBindingTarget`([`PaxIdentifier`](../../../api/internal/pax-language/interpreter.md#paxidentifier))
## Functions
### `escape_identifier`
<pre><code class="api-signature language-rust ignore">pub fn escape_identifier(input: String) -&gt; String</code></pre>

Mangle an identifier into a token-safe representation for generated symbols.

---

### `get_common_properties_as_property_definitions`
<pre><code class="api-signature language-rust ignore">pub fn get_common_properties_as_property_definitions() -&gt; Vec&lt;<a href="../../../api/internal/pax-manifest/index.md#propertydefinition">PropertyDefinition</a>&gt;</code></pre>

Common properties represented as manifest property definitions.

---

### `get_common_properties_type_ids`
<pre><code class="api-signature language-rust ignore">pub fn get_common_properties_type_ids() -&gt; Vec&lt;<a href="../../../api/internal/pax-manifest/index.md#typeid">TypeId</a>&gt;</code></pre>

Type ids for the built-in common properties attached to every template node.

## Constants
### `SUPPORTED_NONNUMERIC_PRIMITIVES`
Primitive nonnumeric Rust types supported directly by manifest reflection.

---

### `SUPPORTED_NUMERIC_PRIMITIVES`
Primitive numeric Rust types supported directly by manifest reflection.
