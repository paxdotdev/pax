# drawing::path_smoothing
<!-- summary: API docs for pax-runtime-api::drawing::path_smoothing. -->
<!-- tags: api, pax-runtime-api -->

## Functions
### `smooth_bez_path`
<pre><code class="api-signature language-rust ignore">pub fn smooth_bez_path(path: &amp;BezPath, smoothing: <a href="/api/pax-runtime-api/drawing.md#pathsmoothing">PathSmoothing</a>) -&gt; BezPath</code></pre>

Converts long polyline runs in `path` to cubic Bezier segments.

Existing quadratic and cubic segments are preserved. Sharp corners split a
line run so smoothing does not round deliberate angular forms.
