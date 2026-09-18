# drawing::stroke_utils
<!-- summary: API docs for pax-runtime-api::drawing::stroke_utils. -->
<!-- tags: api, pax-runtime-api -->

## Functions
### `stroke_width_pixels`
<pre><code class="api-signature language-rust ignore">pub fn stroke_width_pixels(stroke: &amp;<a href="../../../api/pax-runtime-api/drawing.md#stroke">Stroke</a>) -&gt; f64</code></pre>

Resolves a stroke width as pixels.

---

### `stroked_outline_path`
<pre><code class="api-signature language-rust ignore">pub fn stroked_outline_path(centerline: &amp;BezPath, stroke: &amp;<a href="../../../api/pax-runtime-api/drawing.md#stroke">Stroke</a>) -&gt; Option&lt;BezPath&gt;</code></pre>

Builds the filled outline path corresponding to a stroked centerline path.

This is useful for hit testing, masking, and occlusion, where stroke coverage
needs to be treated as fill geometry.
