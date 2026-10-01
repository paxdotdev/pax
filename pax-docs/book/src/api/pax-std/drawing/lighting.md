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

#### Properties
##### `color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Light color.

##### `intensity`
Type: `Property`<`f64`>

Light intensity multiplier.

##### `radius`
Type: `Property`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Radius for point-light attenuation.

##### `z`
Type: `Property`<[`Depth`](../../../api/pax-runtime-api/drawing.md#depth)>

Logical scene depth in pixels.

##### `shape`
Type: `Property`<[`LightShape`](../../../api/pax-runtime-api/drawing.md#lightshape)>

Positional or directional light shape.

##### `direction`
Type: `Property`<[`Vector3`](../../../api/pax-runtime-api/drawing.md#vector3)>

Direction for directional lights.

##### `enabled`
Type: `Property`<`bool`>

Whether this light contributes to the scene.
