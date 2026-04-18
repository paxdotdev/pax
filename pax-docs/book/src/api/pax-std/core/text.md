# core::text
<!-- summary: API docs for pax-std::core::text. -->
<!-- tags: api, pax-std -->

## Structs
### `Text`
Renders and styles text on-screen using platform-specific native elements (for example, a `<div>` with text content on the web, or a `UILabel` on iOS).
Text supports robust text layout features like automatic line breaking and selection, as well as platform-specific
accessibility tools like screen readers.

#### Properties
##### `editable`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether the text can be edited by the user.

##### `selectable`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether the text can be selected by the user.

##### `clip`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether text overflow is clipped to the node bounds.

##### `text`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Text content to display.

##### `style`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`TextStyle`](/api/pax-std/core/text.md#textstyle)>

Text styling.

##### `markdown`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether `text` should be interpreted as Markdown.

---

### `TextStyle`
Struct describing platform-agnostic text display properties.

#### Properties
##### `font`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Font`](/api/pax-std/core/text.md#font)>

Font family/source/style/weight configuration.

##### `font_size`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Size`](/api/pax-runtime-api/layout.md#size)>

Font size, in pixels.

##### `fill`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Fill`](/api/pax-runtime-api/drawing.md#fill)>

Text fill.

##### `underline`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether text should be underlined.

##### `align_multiline`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`TextAlignHorizontal`](/api/pax-std/core/text.md#textalignhorizontal)>

Alignment for multiline text layout.

##### `align_vertical`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`TextAlignVertical`](/api/pax-std/core/text.md#textalignvertical)>

Vertical text alignment within its bounds.

##### `align_horizontal`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`TextAlignHorizontal`](/api/pax-std/core/text.md#textalignhorizontal)>

Horizontal text alignment within its bounds.

## Enums
### `Font`
Describes a font available to native text renderers.

#### Variants
##### `Web`(`String`, `String`, [`FontStyle`](/api/pax-std/core/text.md#fontstyle), [`FontWeight`](/api/pax-std/core/text.md#fontweight))
Web font described by family name, stylesheet URL, style, and weight.

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
<pre><code class="api-signature language-rust ignore">pub fn decrease(weight: <a href="/api/pax-std/core/text.md#fontweight">FontWeight</a>) -&gt; <a href="/api/pax-std/core/text.md#fontweight">FontWeight</a></code></pre>

Returns the next lighter named font weight.

##### `increase`
<pre><code class="api-signature language-rust ignore">pub fn increase(weight: <a href="/api/pax-std/core/text.md#fontweight">FontWeight</a>) -&gt; <a href="/api/pax-std/core/text.md#fontweight">FontWeight</a></code></pre>

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
