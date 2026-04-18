# color
<!-- summary: Color and opacity types used by Pax style properties and render backends. -->
<!-- tags: api, pax-runtime-api -->

Color and opacity types used by Pax style properties and render backends.

## Structs
### `Percent`
Raw Percent type, which we use for serialization and dynamic traversal.  At the time
of authoring, this type is not used directly at runtime, but is intended for `into` coercion
into downstream types, e.g. ColorChannel, Rotation, and Size.  This allows us to be "dumb"
about how we parse `%`, and allow the context in which it is used to pull forward a specific
type through `into` inference.

#### Properties
##### `0`
Type: [`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric)

## Enums
### `Color`
Entrypoint for specifying and representing colors in Pax.

#### Variants
##### `rgb`([`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel))
Models a color in the RGB space, with an alpha channel of 100%

##### `rgba`([`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel))
Models a color in the RGBA space

##### `hsl`([`Rotation`](/api/pax-runtime-api/transform.md#rotation), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel))
Models a color in the HSL space, with an alpha channel of 100%

##### `hsla`([`Rotation`](/api/pax-runtime-api/transform.md#rotation), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel), [`ColorChannel`](/api/pax-runtime-api/color.md#colorchannel))
Models a color in the HSLA space.

##### `SLATE`
Cool blue-gray, RGB (100, 116, 139).

##### `GRAY`
Balanced gray, RGB (107, 114, 128).

##### `ZINC`
Crisp cool gray, RGB (113, 113, 122).

##### `NEUTRAL`
Plain neutral gray, RGB (115, 115, 115).

##### `STONE`
Warm stone gray, RGB (120, 113, 108).

##### `RED`
Bright warm red, RGB (239, 68, 68).

##### `ORANGE`
Vivid citrus orange, RGB (249, 115, 22).

##### `AMBER`
Golden amber, RGB (245, 158, 11).

##### `YELLOW`
Sunny yellow, RGB (234, 179, 8).

##### `LIME`
Electric lime, RGB (132, 204, 22).

##### `GREEN`
Fresh green, RGB (34, 197, 94).

##### `EMERALD`
Jewel emerald, RGB (16, 185, 129).

##### `TEAL`
Deep aquatic teal, RGB (20, 184, 166).

##### `CYAN`
Bright clean cyan, RGB (6, 182, 212).

##### `SKY`
Open sky blue, RGB (14, 165, 233).

##### `BLUE`
Saturated primary blue, RGB (59, 130, 246).

##### `INDIGO`
Cool electric indigo, RGB (99, 102, 241).

##### `VIOLET`
Soft vivid violet, RGB (139, 92, 246).

##### `PURPLE`
Rich playful purple, RGB (168, 85, 247).

##### `FUCHSIA`
Brilliant magenta fuchsia, RGB (217, 70, 239).

##### `PINK`
Bright candy pink, RGB (236, 72, 153).

##### `ROSE`
Warm rosy red, RGB (244, 63, 94).

##### `BLACK`
Pure black, RGB (0, 0, 0).

##### `WHITE`
Pure white, RGB (255, 255, 255).

##### `TRANSPARENT`
Fully transparent white, RGB (255, 255, 255).

##### `NONE`
Non-rendering transparent, RGB (255, 255, 255).

#### Implementations
##### `alpha_0_1`
<pre><code class="api-signature language-rust ignore">pub fn alpha_0_1(&amp;self) -&gt; f64</code></pre>

Returns this color's alpha channel normalized to the `[0.0, 1.0]` range.

##### `from_hex`
<pre><code class="api-signature language-rust ignore">pub fn from_hex(hex: &amp;str) -&gt; Self</code></pre>

Constructs a color from a six- or eight-character RGB/RGBA hex string.

##### `from_rgba_0_1`
<pre><code class="api-signature language-rust ignore">pub fn from_rgba_0_1(rgba_0_1: [f64; 4]) -&gt; Self</code></pre>

Constructs a color from normalized RGBA channels in the `[0.0, 1.0]` range.

##### `hsl`
<pre><code class="api-signature language-rust ignore">pub fn hsl(h: <a href="/api/pax-runtime-api/transform.md#rotation">Rotation</a>, s: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>, l: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>) -&gt; Self</code></pre>

Constructs an HSL color with 100% alpha.

##### `hsla`
<pre><code class="api-signature language-rust ignore">pub fn hsla(h: <a href="/api/pax-runtime-api/transform.md#rotation">Rotation</a>, s: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>, l: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>, a: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>) -&gt; Self</code></pre>

Constructs an HSLA color.

##### `rgb`
<pre><code class="api-signature language-rust ignore">pub fn rgb(r: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>, g: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>, b: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>) -&gt; Self</code></pre>

Constructs an RGB color with 100% alpha.

##### `rgba`
<pre><code class="api-signature language-rust ignore">pub fn rgba(r: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>, g: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>, b: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>, a: <a href="/api/pax-runtime-api/color.md#colorchannel">ColorChannel</a>) -&gt; Self</code></pre>

Constructs an RGBA color.

##### `to_hsla_0_1`
<pre><code class="api-signature language-rust ignore">pub fn to_hsla_0_1(&amp;self) -&gt; [f64; 4]</code></pre>

Returns HSLA channels normalized to the `[0.0, 1.0]` range.

##### `to_rgba_0_1`
<pre><code class="api-signature language-rust ignore">pub fn to_rgba_0_1(&amp;self) -&gt; [f64; 4]</code></pre>

Returns RGBA channels normalized to the `[0.0, 1.0]` range.

##### `with_alpha_factor`
<pre><code class="api-signature language-rust ignore">pub fn with_alpha_factor(&amp;self, factor: f64) -&gt; Self</code></pre>

Multiplies this color's alpha channel by `factor`.

---

### `ColorChannel`
Describes a color channel in a unit appropriate to the surrounding color model.

#### Variants
##### `Rotation`([`Rotation`](/api/pax-runtime-api/transform.md#rotation))
Used, for example, to express hue in HSL or other rotational color models.

##### `Integer`(`u8`)
Integer color channel in the `[0, 255]` range.

##### `Percent`([`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric))
Percent color channel in the `[0.0, 100.0]` range.

#### Implementations
##### `to_float_0_1`
<pre><code class="api-signature language-rust ignore">pub fn to_float_0_1(&amp;self) -&gt; f64</code></pre>

Normalizes this color channel as a float in the `[0.0, 1.0]` range.

---

### `Opacity`
Describes an opacity value either as normalized alpha or as a percent.

#### Variants
##### `Alpha`([`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric))
Unitless alpha in the normalized [0.0, 1.0] range.

##### `Percent`([`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric))
Percent alpha in the [0.0, 100.0] range.

#### Implementations
##### `to_float_0_1`
<pre><code class="api-signature language-rust ignore">pub fn to_float_0_1(&amp;self) -&gt; f64</code></pre>

Normalizes this Opacity as a float in [0.0, 1.0].
