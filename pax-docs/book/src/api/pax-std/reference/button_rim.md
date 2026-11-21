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
