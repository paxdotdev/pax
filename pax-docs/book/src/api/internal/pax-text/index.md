# pax-text
<!-- summary: Experimental single-line Latin LTR shaping and unhinted font outlines. -->
<!-- tags: api, pax-text -->

Experimental single-line Latin LTR shaping and unhinted font outlines.

This renderer-independent helper preserves the website prototype for future
library integration. It is not a `TextGPU` primitive or a replacement for
native `Text`. The crate is not published or re-exported by `pax-kit`.

## Structs
### `OutlineCluster`
An indivisible animation unit that may contain a ligature or combining marks.

#### Properties
##### `byte_start`
Type: `usize`

Inclusive UTF-8 byte offset in the source string.

##### `byte_end`
Type: `usize`

Exclusive UTF-8 byte offset in the source string.

##### `ordinal`
Type: `usize`

Ordinal within this run, useful for staggered effects.

##### `glyph_ids`
Type: `Vec`<`u32`>

Font glyph IDs, for inspection; repeated IDs are not occurrence keys.

##### `x`
Type: `f64`

Horizontal position of the cluster origin in the run, in em units.

##### `advance`
Type: `f64`

Horizontal advance, in em units; may exceed the visible outline.

##### `outline`
Type: `Vec`<[`OutlineCommand`](../../../api/internal/pax-text/index.md#outlinecommand)>

Cluster-local contours, baseline y=0, positive y downward, in em units.

---

### `OutlineFont`
A font instance with an owned outline cache, independent of any Pax component.

Font bytes are owned or static, and shared between shaping and outlining.
The fixed face, weight and unhinted outline mode define the cache identity. Drop the helper
after prewarming fixed runs, or retain it when shaping changing text.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(bytes: impl Into&lt;Blob&gt;, weight: f32) -&gt; Result&lt;Self, String&gt;</code></pre>

Opens face zero at the requested `wght` coordinate.

Accepts owned `Vec<u8>` or static `&'static [u8]` without copying font bytes.
Requires a variable TrueType/OpenType font with a weight axis and cap-height
metrics. Rejects invalid bytes, non-finite weights, and weights outside the axis.

##### `shape_line`
<pre><code class="api-signature language-rust ignore">pub fn shape_line(&amp;mut self, text: &amp;str) -&gt; Result&lt;<a href="../../../api/internal/pax-text/index.md#outlinerun">OutlineRun</a>, String&gt;</code></pre>

Shapes one Latin LTR line, preserving kerning, ligatures and cluster ranges.

Coordinates are in em units with a baseline at y=0 and positive y downward.
Rejects unsupported characters or missing glyphs; no wrapping, bidi, fallback,
font discovery, native selection, or rendering is performed. Spaces retain
their advance with an empty outline. Cached outlines live with this font.

---

### `OutlineRun`
One shaped line. Position clusters using their advances, not their ink bounds.

#### Properties
##### `clusters`
Type: `Vec`<[`OutlineCluster`](../../../api/internal/pax-text/index.md#outlinecluster)>

Logical clusters in this line's left-to-right order.

##### `advance`
Type: `f64`

Total horizontal advance, in em units.

##### `ascent`
Type: `f64`

Font ascent above the baseline, in em units.

##### `descent`
Type: `f64`

Font descent, normally negative, in em units.

##### `cap_height`
Type: `f64`

Font cap height above the baseline, in em units.

## Enums
### `OutlineCommand`
A Bézier contour command with exact, unhinted em-space coordinates.

#### Variants
##### `Move`(`f64`, `f64`)
Start a contour at x, y.

##### `Line`(`f64`, `f64`)
Draw a line to x, y.

##### `Quad`(`f64`, `f64`, `f64`, `f64`)
Draw a quadratic curve through a control point to an endpoint.

##### `Cubic`(`f64`, `f64`, `f64`, `f64`, `f64`, `f64`)
Draw a cubic curve through two control points to an endpoint.

##### `Close`
Close the current contour. Preserve winding when filling it.
