# helpers
<!-- summary: API docs for pax-language::helpers. -->
<!-- tags: api, pax-language -->

## Structs
### `InlinedTemplate`
Source span and contents for a Pax template embedded in an `#[inlined(...)]` attribute.

#### Properties
##### `struct_name`
Type: `String`

##### `start`
Type: (`usize`, `usize`)

##### `end`
Type: (`usize`, `usize`)

##### `template`
Type: `String`

---

### `InlinedTemplateFinder`
AST visitor that extracts `#[inlined(...)]` templates from `#[pax]` structs.

#### Properties
##### `file_contents`
Type: `String`

##### `templates`
Type: `Vec`<[`InlinedTemplate`](../../../api/internal/pax-language/helpers.md#inlinedtemplate)>

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(file_contents: String) -&gt; Self</code></pre>

Prepare a finder for one Rust source file.

## Functions
### `clear_inlined_template`
<pre><code class="api-signature language-rust ignore">pub fn clear_inlined_template(file_path: &amp;str, pascal_identifier: &amp;str)</code></pre>

Replace a matching `#[inlined(...)]` template with an empty template body.

---

### `get_substring_by_line_column`
<pre><code class="api-signature language-rust ignore">pub fn get_substring_by_line_column(input: &amp;str, start: (usize, usize), end: (usize, usize)) -&gt; Option&lt;String&gt;</code></pre>

Extract a source substring addressed by one-indexed line/column coordinates.

---

### `replace_by_line_column`
<pre><code class="api-signature language-rust ignore">pub fn replace_by_line_column(input: &amp;str, start: (usize, usize), end: (usize, usize), replacement: String) -&gt; Option&lt;String&gt;</code></pre>

Replace a source span addressed by one-indexed line/column coordinates.
