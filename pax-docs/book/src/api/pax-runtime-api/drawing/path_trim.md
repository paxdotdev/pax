# drawing::path_trim
<!-- summary: API docs for pax-runtime-api::drawing::path_trim. -->
<!-- tags: api, pax-runtime-api -->

## Functions
### `trim_bez_path`
<pre><code class="api-signature language-rust ignore">pub fn trim_bez_path(path: &amp;BezPath, draw_start: f64, draw_end: f64) -&gt; BezPath</code></pre>

Returns the sub-path visible between normalized `draw_start` and `draw_end`.
