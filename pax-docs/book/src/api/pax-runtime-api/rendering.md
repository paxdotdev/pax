# rendering
<!-- summary: Rendering backend contracts and helpers for drawing Pax scene content. -->
<!-- tags: api, pax-runtime-api -->

Rendering backend contracts and helpers for drawing Pax scene content.

## Structs
### `AlphaMaskPaint`
One vector paint contributing alpha to a mask, in local path coordinates.
Color channels do not affect coverage; fill alpha and opacity are multiplied.

#### Properties
##### `path`
Type: `BezPath`

##### `transform`
Type: `Affine`

##### `fill`
Type: [`Fill`](../../api/pax-runtime-api/drawing.md#fill)

##### `opacity`
Type: `f64`

---

### `ReplayCanvasLayerUpdate`
Replay invalidation for one logical canvas layer.

#### Properties
##### `layer`
Type: `usize`

##### `node_ids`
Type: `Option`<`Vec`<`u32`>>

`None` means the layer should fall back to region/full-layer dirtification.

## Enums
### `Layer`
Render layer selected for a primitive or native surface.

#### Variants
##### `Native`
Platform-native element layer.

##### `NativeNonOccluding`
Platform-native layer that does not participate in occlusion.

##### `Canvas`
GPU/canvas-rendered layer.

##### `DontCare`
Runtime can choose the appropriate layer.

## Traits
### `RenderContext`
The Pax render trait, used as a layer of indirection and contract for backend-agnostic rendering.

## Functions
### `bez_path_to_svg_path_data`
<pre><code class="api-signature language-rust ignore">pub fn bez_path_to_svg_path_data(path: &amp;BezPath) -&gt; String</code></pre>

Convert a kurbo BezPath to a SVG-friendly drawing string
