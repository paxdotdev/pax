# engine::surface_visibility
<!-- summary: Conservative GPU surface admission for desktop scroller layers. -->
<!-- tags: api, pax-runtime -->

Conservative GPU surface admission for desktop scroller layers.

## Structs
### `ScrollerSurfaceVisibility`
A scroller's content-to-window mapping and intersected ancestor warm region.

This only controls backing-surface residency. Nodes and native controls remain
mounted. Recompute it when planning surfaces, including after presentation
scrolling, so a previously cold tile can acquire a fresh backing surface.

#### Implementations
##### `contains_tile`
<pre><code class="api-signature language-rust ignore">pub fn contains_tile(&amp;self, bounds: Rect) -&gt; bool</code></pre>

Whether a content-space tile can contribute inside the warmed clip region.
Tile bounds must include any coverage already reserved for render effects.
