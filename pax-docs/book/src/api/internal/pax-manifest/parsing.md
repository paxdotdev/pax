# parsing
<!-- summary: API docs for pax-manifest::parsing. -->
<!-- tags: api, pax-manifest -->

## Structs
### `ParsingContext`
Accumulator for manifest parsing across components and reflected types.

#### Properties
##### `visited_type_ids`
Type: `HashSet`<[`TypeId`](/api/internal/pax-manifest/index.md#typeid)>

Used to track which files/sources have been visited during parsing,
to prevent duplicate parsing

##### `main_component_type_id`
Type: [`TypeId`](/api/internal/pax-manifest/index.md#typeid)

##### `component_definitions`
Type: `BTreeMap`<[`TypeId`](/api/internal/pax-manifest/index.md#typeid), [`ComponentDefinition`](/api/internal/pax-manifest/index.md#componentdefinition)>

##### `template_map`
Type: `HashMap`<`String`, [`TypeId`](/api/internal/pax-manifest/index.md#typeid)>

##### `template_node_definitions`
Type: [`ComponentTemplate`](/api/internal/pax-manifest/index.md#componenttemplate)

##### `type_table`
Type: [`TypeTable`](/api/internal/pax-manifest/index.md#typetable)

##### `assets_dirs`
Type: `Vec`<`String`>

---

### `ParsingError`
Source-mapped parsing error payload used by designer/editor integrations.

#### Properties
##### `error_name`
Type: `String`

##### `error_message`
Type: `String`

##### `matched_string`
Type: `String`

##### `start`
Type: (`usize`, `usize`)

##### `end`
Type: (`usize`, `usize`)

---

### `TemplateNodeParseContext`
Mutable state used while parsing template nodes for one component.

#### Properties
##### `template`
Type: [`ComponentTemplate`](/api/internal/pax-manifest/index.md#componenttemplate)

##### `pascal_identifier_to_type_id_map`
Type: `HashMap`<`String`, [`TypeId`](/api/internal/pax-manifest/index.md#typeid)>

## Traits
### `Reflectable`
This trait is used only to extend primitives like u64
with the parser-time method `parse_to_manifest`.  This
allows the parser binary to codegen calls to `::parse_to_manifest()` even
on primitive types

## Functions
### `assemble_component_definition`
<pre><code class="api-signature language-rust ignore">pub fn assemble_component_definition(ctx: <a href="/api/internal/pax-manifest/parsing.md#parsingcontext">ParsingContext</a>, pax: &amp;str, is_main_component: bool, template_map: HashMap&lt;String, <a href="/api/internal/pax-manifest/index.md#typeid">TypeId</a>&gt;, module_path: &amp;str, self_type_id: <a href="/api/internal/pax-manifest/index.md#typeid">TypeId</a>, component_source_file_path: &amp;str) -&gt; (<a href="/api/internal/pax-manifest/parsing.md#parsingcontext">ParsingContext</a>, <a href="/api/internal/pax-manifest/index.md#componentdefinition">ComponentDefinition</a>)</code></pre>

From a raw string of Pax representing a single component, parse a complete ComponentDefinition

---

### `assemble_primitive_definition`
<pre><code class="api-signature language-rust ignore">pub fn assemble_primitive_definition(module_path: &amp;str, primitive_instance_import_path: String, self_type_id: <a href="/api/internal/pax-manifest/index.md#typeid">TypeId</a>) -&gt; <a href="/api/internal/pax-manifest/index.md#componentdefinition">ComponentDefinition</a></code></pre>

Build a component definition for a built-in primitive.

---

### `assemble_struct_only_component_definition`
<pre><code class="api-signature language-rust ignore">pub fn assemble_struct_only_component_definition(ctx: <a href="/api/internal/pax-manifest/parsing.md#parsingcontext">ParsingContext</a>, module_path: &amp;str, self_type_id: <a href="/api/internal/pax-manifest/index.md#typeid">TypeId</a>) -&gt; (<a href="/api/internal/pax-manifest/parsing.md#parsingcontext">ParsingContext</a>, <a href="/api/internal/pax-manifest/index.md#componentdefinition">ComponentDefinition</a>)</code></pre>

Build a component definition for a `#[pax]` data struct with no template.

---

### `assemble_type_definition`
<pre><code class="api-signature language-rust ignore">pub fn assemble_type_definition(ctx: <a href="/api/internal/pax-manifest/parsing.md#parsingcontext">ParsingContext</a>, property_definitions: Vec&lt;<a href="/api/internal/pax-manifest/index.md#propertydefinition">PropertyDefinition</a>&gt;, inner_iterable_type_id: Option&lt;<a href="/api/internal/pax-manifest/index.md#typeid">TypeId</a>&gt;, self_type_id: <a href="/api/internal/pax-manifest/index.md#typeid">TypeId</a>) -&gt; (<a href="/api/internal/pax-manifest/parsing.md#parsingcontext">ParsingContext</a>, <a href="/api/internal/pax-manifest/index.md#typedefinition">TypeDefinition</a>)</code></pre>

Insert and return a reflected type definition.

---

### `clean_module_path`
<pre><code class="api-signature language-rust ignore">pub fn clean_module_path(module_path: &amp;str) -&gt; String</code></pre>

Convert macro-parser module roots into crate-relative paths.

---

### `parse_template_from_component_definition_string`
<pre><code class="api-signature language-rust ignore">pub fn parse_template_from_component_definition_string(ctx: &amp;mut <a href="/api/internal/pax-manifest/parsing.md#templatenodeparsecontext">TemplateNodeParseContext</a>, pax: &amp;str, pax_component_definition: Pair&lt;&#39;_, <a href="/api/internal/pax-lang/index.md#rule">Rule</a>&gt;)</code></pre>

Parse template nodes out of a component-definition AST into a mutable template context.
