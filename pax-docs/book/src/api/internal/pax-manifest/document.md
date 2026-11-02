# document
<!-- summary: Source-only document contracts and documentation directives. -->
<!-- tags: api, pax-manifest -->

Source-only document contracts and documentation directives.

## Enums
### `DocumentDirective`
A reserved standalone comment affecting only generated document content.

#### Variants
##### `Ignore`
##### `Heading`(`u8`)
#### Implementations
##### `parse`
<pre><code class="api-signature language-rust ignore">pub fn parse(comment: &amp;str) -&gt; Result&lt;Option&lt;Self&gt;, String&gt;</code></pre>

Ordinary comments have no directive. The exact `///#` namespace is reserved.

---

### `DocumentKind`
Static extraction behavior owned by a component declaration.
This metadata does not participate in runtime mounting or binary programs.

#### Variants
##### `Text`
##### `Link`
##### `Children`
##### `Mask`
##### `Settings`
## Functions
### `validate_heading_targets`
<pre><code class="api-signature language-rust ignore">pub fn validate_heading_targets(manifest: &amp;<a href="../../../api/internal/pax-manifest/index.md#paxmanifest">PaxManifest</a>) -&gt; Result&lt;(), String&gt;</code></pre>

Validate heading receivers in every template, including unreachable routes.
