# formatting
<!-- summary: API docs for pax-lang::formatting. -->
<!-- tags: api, pax-lang -->

## Functions
### `format_file`
<pre><code class="api-signature language-rust ignore">pub fn format_file(file_path: &amp;str) -&gt; Result&lt;(), Report&gt;</code></pre>

Format either a `.pax` file or an inlined Pax template inside a Rust file.

---

### `format_pax_template`
<pre><code class="api-signature language-rust ignore">pub fn format_pax_template(code: String) -&gt; Result&lt;String, Report&gt;</code></pre>

Format one Pax template string.
