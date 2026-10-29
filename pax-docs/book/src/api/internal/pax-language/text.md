# text
<!-- summary: Scalar element-body text: layout whitespace and character references. -->
<!-- tags: api, pax-language -->

Scalar element-body text: layout whitespace and character references.

## Structs
### `BodyTextError`
Invalid character reference, with a byte offset in the original body.

#### Properties
##### `offset`
Type: `usize`

##### `message`
Type: &'`static` `str`

## Functions
### `decode_body_text`
<pre><code class="api-signature language-rust ignore">pub fn decode_body_text(source: &amp;str) -&gt; Result&lt;Option&lt;String&gt;, <a href="../../../api/internal/pax-language/text.md#bodytexterror">BodyTextError</a>&gt;</code></pre>

Decode ordinary body text. Layout-only whitespace means no assignment.
References are decoded once, after whitespace normalization, and never
reinterpreted as template syntax. Raw string expressions bypass this path.

---

### `normalize_body_whitespace`
<pre><code class="api-signature language-rust ignore">pub fn normalize_body_whitespace(source: &amp;str) -&gt; String</code></pre>

Remove block framing and common indentation without interpreting references.
Inline bodies retain their spaces. Line endings become LF; blank lines,
relative indentation and trailing spaces remain significant.
