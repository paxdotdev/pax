# core::text
<!-- summary: API docs for pax-std::core::text. -->
<!-- tags: api, pax-std -->

## Structs
### `Text`
Renders and styles text through the target's native text system.

A constrained width with an omitted height allows native measurement to
determine the height of wrapped content. Selection, editing, font loading,
and accessibility behavior depend on the target's native implementation;
test the intended interaction on each shipping target.

#### Properties
##### `editable`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether the text can be edited by the user.

##### `selectable`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Requests selectable text. On the current iOS/iPadOS path, non-editable
text uses the interactive selection view only when `clip` is enabled.

##### `clip`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether text overflow is clipped to the node bounds.

##### `text`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`String`>

Text content to display.

##### `style`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`TextStyle`](../../../api/pax-std/core/text.md#textstyle)>

Text styling.

##### `markdown`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether `text` should be interpreted as Markdown.

##### `wrap`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether long text lines should wrap inside the node bounds.

---

### `TextStyle`
Struct describing platform-agnostic text display properties.
Interpolation eases font size and fill; font selection, underline, and
alignment switch to the destination immediately. Glyphs do not morph.

#### Properties
##### `font`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Font`](../../../api/pax-std/core/text.md#font)>

Font family/source/style/weight configuration.

##### `font_size`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Font size, in pixels.

##### `fill`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Fill`](../../../api/pax-runtime-api/drawing.md#fill)>

Text color. Native text patches reduce gradient fills to their first
stop's color; use a solid fill for predictable text color.

##### `underline`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether text should be underlined.

##### `align_multiline`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`TextAlignHorizontal`](../../../api/pax-std/core/text.md#textalignhorizontal)>

Alignment for multiline text layout.

##### `align_vertical`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`TextAlignVertical`](../../../api/pax-std/core/text.md#textalignvertical)>

Vertical text alignment within its bounds.

##### `align_horizontal`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`TextAlignHorizontal`](../../../api/pax-std/core/text.md#textalignhorizontal)>

Horizontal text alignment within its bounds.

## Enums
### `Font`
Describes a font available to native text renderers.

Pax templates may use either the explicit [`Font::Web`] constructor or a
contextual shorthand. A string names a locally available family with an
empty source URL and normal style and weight:

```pax
font: "Times New Roman"
```

Named object fields describe font sources and optional modifiers. Omitted
fields retain their defaults. Supplying `family` without `url` selects a
locally available family, matching the string shorthand. `weight` accepts
either [`FontWeight`] or its CSS numeric equivalent from `100` through
`900`:

```pax
font: {
    family: "Inter"
    url: "https://example.com/Inter-Italic.ttf"
    style: FontStyle::Italic
    weight: 700
}
```

Nonempty URLs identify font files, except that Google Fonts URLs containing
`fonts.googleapis.com/css` receive special stylesheet handling. Relative
asset URLs work on web; the native Web-font loader does not resolve them
into application bundle resources. Native targets also require the actual
font family name, rather than a browser-only alias.

Font shorthand has no positional ("magic index") fields: named keys are
canonical, and positional list/tuple forms are not accepted. Use the
explicit [`Font::Web`] constructor as verbose longhand when needed.

#### Variants
##### `Web`(`String`, `String`, [`FontStyle`](../../../api/pax-std/core/text.md#fontstyle), [`FontWeight`](../../../api/pax-std/core/text.md#fontweight))
Font described by family name, source URL, style, and weight.

---

### `FontStyle`
Describes available font styles.

#### Variants
##### `Normal`
##### `Italic`
##### `Oblique`
---

### `FontWeight`
Describes available font weights.

#### Variants
##### `Thin`
##### `ExtraLight`
##### `Light`
##### `Normal`
##### `Medium`
##### `SemiBold`
##### `Bold`
##### `ExtraBold`
##### `Black`
#### Implementations
##### `decrease`
<pre><code class="api-signature language-rust ignore">pub fn decrease(weight: <a href="../../../api/pax-std/core/text.md#fontweight">FontWeight</a>) -&gt; <a href="../../../api/pax-std/core/text.md#fontweight">FontWeight</a></code></pre>

Returns the next lighter named font weight.

##### `increase`
<pre><code class="api-signature language-rust ignore">pub fn increase(weight: <a href="../../../api/pax-std/core/text.md#fontweight">FontWeight</a>) -&gt; <a href="../../../api/pax-std/core/text.md#fontweight">FontWeight</a></code></pre>

Returns the next heavier named font weight.

---

### `TextAlignHorizontal`
Describes available horizontal text alignments.

#### Variants
##### `Left`
##### `Center`
##### `Right`
---

### `TextAlignVertical`
Describes available vertical text alignments.

#### Variants
##### `Top`
##### `Center`
##### `Bottom`
