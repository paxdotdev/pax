# code_serialization
<!-- summary: API docs for pax-manifest::code_serialization. -->
<!-- tags: api, pax-manifest -->

## Structs
### `RustFileSerialization`
Template payload for generating a Rust file that hosts a Pax component.

#### Properties
##### `pax_path`
Type: `String`

##### `pascal_identifier`
Type: `String`

## Functions
### `diff`
<pre><code class="api-signature language-rust ignore">pub fn diff(old_content: &amp;str, new_content: &amp;str) -&gt; Option&lt;String&gt;</code></pre>

Colorized line diff for terminal output.

---

### `diff_html`
<pre><code class="api-signature language-rust ignore">pub fn diff_html(old_content: &amp;str, new_content: &amp;str) -&gt; Option&lt;String&gt;</code></pre>

HTML diff used by designer-facing source update previews.

---

### `press_code_serialization_template`
<pre><code class="api-signature language-rust ignore">pub fn press_code_serialization_template(args: <a href="/api/internal/pax-manifest/index.md#componentdefinition">ComponentDefinition</a>) -&gt; Result&lt;String, String&gt;</code></pre>

Serialize one component definition back into Pax source.

---

### `press_rust_file_serialization_template`
<pre><code class="api-signature language-rust ignore">pub fn press_rust_file_serialization_template(args: <a href="/api/internal/pax-manifest/code_serialization.md#rustfileserialization">RustFileSerialization</a>) -&gt; String</code></pre>

Render the Rust-file template for a new component.

---

### `serialize_component_to_file`
<pre><code class="api-signature language-rust ignore">pub fn serialize_component_to_file(component: &amp;<a href="/api/internal/pax-manifest/index.md#componentdefinition">ComponentDefinition</a>, file_path: String)</code></pre>

Serialize a component to a file
Replaces entire .pax file and replaces inlined attribute directly for .rs files

---

### `serialize_main_component`
<pre><code class="api-signature language-rust ignore">pub fn serialize_main_component(manifest: &amp;<a href="/api/internal/pax-manifest/index.md#paxmanifest">PaxManifest</a>, repo_root: &amp;str)</code></pre>

Serialize the manifest's main component back to its source file.

---

### `serialize_main_component_to_string`
<pre><code class="api-signature language-rust ignore">pub fn serialize_main_component_to_string(manifest: &amp;<a href="/api/internal/pax-manifest/index.md#paxmanifest">PaxManifest</a>) -&gt; String</code></pre>

Serialize the manifest's main component to a Pax source string.

---

### `serialize_new_component_rust_file`
<pre><code class="api-signature language-rust ignore">pub fn serialize_new_component_rust_file(comp_def: &amp;<a href="/api/internal/pax-manifest/index.md#componentdefinition">ComponentDefinition</a>, pax_file_path: String)</code></pre>

Create the companion Rust file for a newly created blank component.
