# media::image
<!-- summary: API docs for pax-std::media::image. -->
<!-- tags: api, pax-std -->

## Structs
### `Image`
A GPU/canvas-rendered image decoded by the active chassis.

`Image` draws into the node bounds and participates in the same canvas
rendering path as vectors. Use `NativeImage` when a platform-native image
element is preferable.

#### Properties
##### `source`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`ImageSource`](/api/pax-std/media/image.md#imagesource)>

Image source: empty, URL, or raw RGBA data.

##### `fit`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`ImageFit`](/api/pax-std/media/image.md#imagefit)>

How the image should fit into this node's bounds.

## Enums
### `ImageFit`
Image fit/layout options.

#### Variants
##### `Fill`
Scale the image to fill its bounds, possibly clipping part of the image.

##### `Fit`
Scale the image to fit within its bounds without clipping, possibly leaving empty space.

##### `Stretch`
Stretch the image to exactly match the container.

---

### `ImageSource`
Source data for an `Image`.

#### Variants
##### `Empty`
No image.

##### `Url`(`String`)
Image loaded from a URL/path understood by the chassis.

##### `Data`(`usize`, `usize`, `Vec`<`u8`>)
Raw RGBA image data: width, height, and bytes where `len = width * height * 4`.
