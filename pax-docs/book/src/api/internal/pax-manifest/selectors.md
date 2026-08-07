# selectors
<!-- summary: API docs for pax-manifest::selectors. -->
<!-- tags: api, pax-manifest -->

## Structs
### `TemplateNodeSelectorInfo`
Selector identity persisted for one authored template node.

`class_binding` is the sole class representation: literal strings and lists
as well as PAXEL expressions all cross the manifest/runtime boundary here.

#### Properties
##### `source_location`
Type: `Option`<[`LocationInfo`](/api/internal/pax-manifest/index.md#locationinfo)>

Source span of the authored node, when available.

##### `id`
Type: `Option`<[`Token`](/api/internal/pax-manifest/index.md#token)>

The node's authored id selector.

##### `class_binding`
Type: `Option`<[`ValueDefinition`](/api/internal/pax-manifest/index.md#valuedefinition)>

The complete `class` attribute value. Literal and expression-backed classes share this
representation so runtime selector resolution has a single source of truth.

#### Implementations
##### `literal_classes`
<pre><code class="api-signature language-rust ignore">pub fn literal_classes(&amp;self) -&gt; Vec&lt;String&gt;</code></pre>

Return class names that can be read without evaluating an expression.
