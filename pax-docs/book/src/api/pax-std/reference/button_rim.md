# reference::button_rim
<!-- summary: API docs for pax-std::reference::button_rim. -->
<!-- tags: api, pax-std -->

## Structs
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

Some(density) draws cloud glass with a 2 px rim when no pool density is set.
With both densities unset the face uses the 1 px contour.
For cloud glass, project an opaque silhouette and supply the CPU fallback as its paint.

##### `pool_density`
Type: `Property`<`Option`<`f64`>>

Some(density) applies pool waves to the projected paint with a 2 px rim.
Takes precedence over cloud glass. The shared surface phase controls wave time.

##### `pool_amplitude`
Type: `Property`<`Option`<`f64`>>

Pool wave strength, from 0 (base paint) to 1 (default). Does not attenuate the rim.

---

### `ButtonSurfacePhase`
Shared phase in seconds for animated button faces. Visible buttons follow the
host's clock. Holding the phase freezes the pattern while scene lights respond.

#### Properties
##### `0`
Type: `LocalProperty`<`f64`>
