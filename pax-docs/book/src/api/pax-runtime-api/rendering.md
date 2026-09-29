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
Type: [`Paint`](../../api/pax-runtime-api/drawing.md#paint)

##### `opacity`
Type: `f64`

##### `paint_bounds`
Type: `Rect`

Complete local geometry bounds before stroke expansion or trimming.

##### `composition`
Type: `Option`<(`u32`, `f32`)>

Primitive boundary: compose its paints before applying source-relative opacity.

---

### `OpacityScope`
One authored opacity boundary, ordered from the outermost ancestor to the painted node.
Native surfaces can present independently while a canvas backend composes each scope.

#### Properties
##### `node_id`
Type: `u32`

##### `opacity`
Type: `f32`

---

### `ReplayCanvasLayerUpdate`
Replay invalidation for one logical canvas layer.

#### Properties
##### `layer`
Type: `usize`

##### `regions`
Type: `Option`<`Vec`<`Rect`>>

Canvas-content regions requiring replay. The runtime selects nodes from its
shared scene geometry; `None` requests full-layer dirtification.

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
The drawing contract shared by Pax runtime backends.

## Functions
### `bez_path_to_svg_path_data`
<pre><code class="api-signature language-rust ignore">pub fn bez_path_to_svg_path_data(path: &amp;BezPath) -&gt; String</code></pre>

Convert a kurbo BezPath to a SVG-friendly drawing string
