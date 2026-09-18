# drawing::lighting
<!-- summary: API docs for pax-std::drawing::lighting. -->
<!-- tags: api, pax-std -->

## Structs
### `AmbientLight`
A non-rendering singleton ambient-light override for its scene.

#### Properties
##### `color`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Ambient color.

##### `intensity`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Ambient intensity multiplier.

##### `enabled`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether this ambient override participates in topmost-wins selection.

---

### `LightFrame`
A non-rendering container that keeps descendant lights from escaping its subtree.

Lights outside the frame may still illuminate its descendants. Each expanded
frame instance has its own lexical lighting identity.

---

### `LightSource`
A non-rendering light resource that affects light-reactive vector materials in its scene.

#### Properties
##### `color`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Light color.

##### `intensity`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Light intensity multiplier.

##### `radius`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Radius for point-light attenuation.

##### `z`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Depth`](../../../api/pax-runtime-api/drawing.md#depth)>

Logical scene depth in pixels.

##### `shape`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`LightShape`](../../../api/pax-runtime-api/drawing.md#lightshape)>

Positional or directional light shape.

##### `direction`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Vector3`](../../../api/pax-runtime-api/drawing.md#vector3)>

Direction for directional lights.

##### `enabled`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether this light contributes to the scene.
