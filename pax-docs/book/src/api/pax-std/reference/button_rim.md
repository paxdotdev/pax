# reference::button_rim
<!-- summary: API docs for pax-std::reference::button_rim. -->
<!-- tags: api, pax-std -->

## Structs
### `ButtonCloudPhase`
Shared drift phase in seconds for cloud buttons. Provide this store on an
ancestor to animate visible faces from one clock. Without it, the gas is still.

#### Properties
##### `0`
Type: `LocalProperty`<`f64`>

---

### `ButtonRim`
Draw a thin scene-lit CMY contour on the alpha silhouette of a button face.
Project vector artwork here and keep native controls and labels as siblings.
Requires the shaders feature, a GPU renderer, and a scoped direct light.

#### Properties
##### `enabled`
Type: `Property`<`bool`>

Enable the contour. Defaults to false, which draws the face directly.

##### `cloud_density`
Type: `Property`<`Option`<`f64`>>

Some(density) draws cloud glass with a 2 px rim. None draws the 1 px contour.
For cloud glass, project an opaque silhouette and supply the CPU fallback as its paint.
