# formatting
<!-- summary: API docs for pax-language::formatting. -->
<!-- tags: api, pax-language -->

## Structs
### `FormatSummary`
Result of formatting a file or directory tree.

#### Properties
##### `files_checked`
Type: `usize`

Pax-bearing source files inspected.

##### `changed_files`
Type: `Vec`<`PathBuf`>

Files whose canonical representation differs from disk.

## Functions
### `format_file`
<pre><code class="api-signature language-rust ignore">pub fn format_file(file_path: &amp;str) -&gt; Result&lt;(), Report&gt;</code></pre>

Format either a `.pax` file or an inlined Pax template inside a Rust file.

---

### `format_path`
<pre><code class="api-signature language-rust ignore">pub fn format_path(path: &amp;<a href="/api/pax-std/drawing/path.md#path">Path</a>, check: bool) -&gt; Result&lt;<a href="/api/internal/pax-language/formatting.md#formatsummary">FormatSummary</a>, Report&gt;</code></pre>

Format a `.pax`/`.rs` file or every Pax-bearing source file below a directory.

Directory traversal skips generated and dependency directories (`.git`, `.pax`,
`target`, and `node_modules`). When `check` is true, files are inspected but not
written, and changed paths are returned in [`FormatSummary::changed_files`].

---

### `format_pax_template`
<pre><code class="api-signature language-rust ignore">pub fn format_pax_template(code: String) -&gt; Result&lt;String, Report&gt;</code></pre>

Format one Pax template string.
