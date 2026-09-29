# appearance
<!-- summary: Static validation of appearance values at their expected property boundary. -->
<!-- tags: api, pax-manifest -->

Static validation of appearance values at their expected property boundary.

## Functions
### `validate_appearance`
<pre><code class="api-signature language-rust ignore">pub fn validate_appearance(value: &amp;<a href="../../../api/internal/pax-manifest/index.md#valuedefinition">ValueDefinition</a>, stroke: bool) -&gt; Result&lt;(), String&gt;</code></pre>

Checks the statically known parts of a fill/stroke stack. Dynamic bindings
are checked by the same layer coercions when their values become available.
