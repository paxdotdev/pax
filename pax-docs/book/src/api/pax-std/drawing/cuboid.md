# drawing::cuboid
<!-- summary: API docs for pax-std::drawing::cuboid. -->
<!-- tags: api, pax-std -->

## Structs
### `Cuboid`
A rectangular solid whose six surfaces participate in spatial composition.
Requires the default-off `3d` feature (`pax-kit/3d`).

`width` and `height` use ordinary local layout; `depth` is an absolute length
centered around local Z=0. The default depth is 100 logical pixels. All faces
receive `fill`; add [`CuboidFace`] children to place live content on individual
faces. Face content keeps its mounted identity when the solid rotates.

Uses the experimental GPU spatial compositor, including its native-layer
restrictions. The current view is orthographic; materials retain the current
renderer's material model with the position and oriented normal of each face.

#### Properties
##### `depth`
Type: `Property`<[`Depth`](../../../api/pax-runtime-api/drawing.md#depth)>

Solid extent along local Z, in logical pixels. Negative/non-finite values
collapse to zero. Percentages are not accepted.

##### `fill`
Type: `Property`<`Vec`<[`Fill`](../../../api/pax-runtime-api/drawing.md#fill)>>

Paint shared by all six faces. Empty paints leave only authored face content.

---

### `CuboidFace`
Places an ordinary live subtree on one face of the nearest owning [`Cuboid`].

Front/back content is width × height, left/right is depth × height, and
top/bottom is width × depth. Content is upright when viewed from outside its
face. A projected rectangular aperture clips it to that face. Normal layout,
events, native controls, and further spatial transforms work inside the face.
Changing `side` moves the same subtree; it does not recreate its children.
Several face containers may contribute to one side in ordinary paint order.

#### Properties
##### `side`
Type: `Property`<[`CuboidSide`](../../../api/pax-std/drawing/cuboid.md#cuboidside)>

The face receiving this subtree; defaults to Front.

## Enums
### `CuboidSide`
Outward-facing sides of a [`Cuboid`], before its own transforms are applied.

#### Variants
##### `Front`
Positive Z; faces the default viewer.

##### `Back`
Negative Z.

##### `Right`
Positive X.

##### `Left`
Negative X.

##### `Top`
Negative Y.

##### `Bottom`
Positive Y.
