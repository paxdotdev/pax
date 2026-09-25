# media::image
<!-- summary: API docs for pax-std::media::image. -->
<!-- tags: api, pax-std -->

## Structs
### `Image`
A GPU/canvas-rendered image decoded by the active chassis.

`Image` draws into the node bounds and participates in the same canvas
rendering path as vectors. Use `NativeImage` when a platform-native image
element is preferable.
On WGPU and browser Piet, common opacity fades the composed canvas subtree;
source pixel alpha remains part of the content. Native surfaces fade separately.

#### Properties
##### `source`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`ImageSource`](../../../api/pax-std/media/image.md#imagesource)>

Image source: empty, URL, or raw RGBA data.

##### `fit`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`ImageFit`](../../../api/pax-std/media/image.md#imagefit)>

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

In Pax templates, a string in an `ImageSource` context is shorthand for
[`ImageSource::Url`]:

```pax
<Image source="assets/spaceship.png" />
<Image source={avatar_url} />
```

[`ImageSource::Url`] and [`ImageSource::Data`] remain available as explicit
constructor forms. Unlike the list shorthands used by compound Pax types,
the URL shorthand has no positional ("magic index") fields: the entire
string is the URL or chassis-relative asset path.

#### Variants
##### `Empty`
No image.

##### `Url`(`String`)
Image loaded from a URL/path understood by the chassis.

##### `Data`(`usize`, `usize`, `Vec`<`u8`>)
Raw RGBA image data: width, height, and bytes where `len = width * height * 4`.
