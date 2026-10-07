# engine::surface_metrics
<!-- summary: Canonical logical-to-physical backing contract for both planning and allocation. -->
<!-- tags: api, pax-runtime -->

Canonical logical-to-physical backing contract for both planning and allocation.

## Structs
### `RasterDensityTracker`
Grow immediately at display-quality density, but shrink only after 500 ms
at a stable scale. Logarithmic upward quantization bounds extra density to
about 4.5% and makes rotation/floating-point noise allocation-inert.

## Functions
### `installed_surface_metrics`
<pre><code class="api-signature language-rust ignore">pub fn installed_surface_metrics(width: f64, height: f64, backing: [u32; 2], density_clamped: bool) -&gt; SurfaceMetrics</code></pre>

Allocation consumes the already planned physical size; it must not apply
display scale a second time or reconstruct it from a different snapshot.

---

### `projected_surface_metrics`
<pre><code class="api-signature language-rust ignore">pub fn projected_surface_metrics(width: f64, height: f64, density: [f64; 2], limit: u32) -&gt; SurfaceMetrics</code></pre>

Size a backing at the projected display density, including densities below
one for minified content. Only a backend hard limit may reduce that density.
