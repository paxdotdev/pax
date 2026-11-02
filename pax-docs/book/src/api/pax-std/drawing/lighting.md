# drawing::lighting
<!-- summary: API docs for pax-std::drawing::lighting. -->
<!-- tags: api, pax-std -->

## Structs
### `AmbientLight`
A non-rendering singleton ambient-light override for its scene.

#### Properties
##### `color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Ambient color.

##### `intensity`
Type: `Property`<`f64`>

Ambient intensity multiplier.

##### `enabled`
Type: `Property`<`bool`>

Whether this ambient override participates in topmost-wins selection.

---

### `LightFrame`
A non-rendering container that keeps descendant lights from escaping its subtree.

Lights outside the frame may still illuminate its descendants. Each expanded
frame instance has its own lexical lighting identity.

---

### `LightSource`
A non-rendering light resource that affects light-reactive vector materials in its scene.

The emitter is the transformed center of this node, using common `x`, `y`,
and `z` placement (or `position`). Depth inherits ancestor transforms and
defaults to zero. Set `z=200px` to retain the former light-specific default.

#### Properties
##### `color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Light color.

##### `intensity`
Type: `Property`<`f64`>

Light intensity multiplier.

##### `radius`
Type: `Property`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Point/strip attenuation radius in world pixels; ancestor scale does not resize it.

##### `shape`
Type: `Property`<[`LightShape`](../../../api/pax-runtime-api/drawing.md#lightshape)>

Light shape. A strip spans the node's local width through its center;
ancestor transforms orient/resize it. Height is not emitter thickness.

##### `direction`
Type: `Property`<`Vector3`>

Local direction for directional lights, transformed with the light's ancestors.

##### `enabled`
Type: `Property`<`bool`>

Whether this light contributes to the scene.

---

### `StripLight`
A finite line light. Its width sets length, its center sets position, and
`rotate` sets its angle. The emitter is non-rendering and uses one light slot.

Uses the GPU renderer's continuous nearest-point line-light model, not a
sampled row of point lights or physically integrated area illumination.
Rougher materials soften its band; glossy materials give a tighter reflection.
Height is only a layout box, not emitter thickness. Zero width behaves as a point.
Like LightSource, it respects LightFrame scope; CPU/native-text lighting is unsupported.
Position and depth use common node transforms, with zero depth by default.

#### Properties
##### `color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Emitted light color.

##### `intensity`
Type: `Property`<`f64`>

Brightness multiplier; independent of strip length.

##### `radius`
Type: `Property`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Attenuation distance from the closest point on the strip, in pixels.

##### `enabled`
Type: `Property`<`bool`>

Whether this emitter contributes, without unmounting it.
