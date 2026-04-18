# media::native_image
<!-- summary: API docs for pax-std::media::native_image. -->
<!-- tags: api, pax-std -->

## Structs
### `NativeImage`
A platform-native image.  This will be managed by Pax's compositor as a platform-specific
host for an image, for example a DOM `<img>` element on the web, or a UIImageView on iOS.
`NativeImage` is an alternative to using `pax_std::media::Image`, with various trade-offs,
where the latter is managed as a GPU texture and drawn on a canvas surface.

#### Properties
##### `url`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Image URL/path understood by the chassis.

##### `fit`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`ImageFit`](/api/pax-std/media/image.md#imagefit)>

How the image should fit into this node's bounds.
