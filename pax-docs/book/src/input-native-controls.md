# Input & Native Controls
<!-- summary: Native UI controls, input events, and interaction patterns. -->
<!-- tags: input, controls -->

Pax templates can mix vector-rendered content with platform-native controls.
Native controls participate in the same layout and occlusion coordinate space as
the rest of the scene, but each control owns the platform affordance needed for
its input model.

## PhotoPicker

`PhotoPicker` opens the platform photo library or camera when its slotted
content is activated. The slotted subtree is the visible affordance; `PhotoPicker`
adds the transparent native hit target needed by the web and Apple chassis.

```pax
<PhotoPicker
    x=50%
    y=148px
    width=86%
    height=54px
    anchor_x=50%
    source=PhotoPickerSource::Library
    allow_multiple=true
    accept="image/*"
    include_bytes=true
    max_bytes_per_photo=26214400
    @photo_picker_change=self.handle_picker_change
>
    <Text class="button_label" width=100% height=100% text="Choose Photos"/>
    <Rectangle class="primary_button" width=100% height=100%/>
</PhotoPicker>
```

Use `PhotoPickerSource::Library` for existing images and
`PhotoPickerSource::Camera` for camera capture. Camera support depends on the
current chassis and device; simulators may report the camera as unavailable.

The change event is delivered to the `PhotoPicker` node:

```rust
pub fn handle_picker_change(
    &mut self,
    _ctx: &NodeContext,
    event: Event<PhotoPickerChange>,
) {
    if event.status == PhotoPickerStatus::Selected {
        for photo in event.photos.iter() {
            let preview_url = photo.handle.clone();
            let bytes = photo.data.as_ref();
        }
    }
}
```

`PhotoPickerChange` includes:

- `status`: `Selected`, `Cancelled`, `PermissionDenied`, `Unavailable`,
  `SizeLimitExceeded`, or `Failed`.
- `request_id`: copied from the picker's `trigger` value for programmatic
  request tracking.
- `photos`: selected image metadata including file name, MIME type, byte size,
  dimensions when available, source kind, optional preview/read handle, and
  optional copied bytes.

Use `NativeImage` with a returned `photo.handle` when you need to preview a
selected image in the scene. Handles are transient platform references such as
object URLs or file URLs; copy `photo.data` or persist your own app-local file if
the selection must survive beyond the current session.

Set `include_bytes=false` when previews/handles are enough. If
`include_bytes=true`, `max_bytes_per_photo` bounds the amount of data the chassis
will copy into the event; assets over the limit are skipped and reported with a
size-limit status.

Apple camera builds need a camera usage string in Cargo metadata:

```toml
[package.metadata.pax.ios.info_plist]
NSCameraUsageDescription = "Capture photos when the camera source is selected."
```

iPadOS inherits iOS metadata by default. Use
`[package.metadata.pax.ipados.info_plist]` only when the iPad string should
differ.

The `examples/src/photo-picker` project demonstrates library selection, camera
capture, byte limits, metadata display, and thumbnail previews.

## Other Controls

Buttons, textboxes, dropdowns, sliders, links, and other controls expose their
own event handlers and styling properties. As with `PhotoPicker`, prefer making
the visible affordance in Pax markup and letting the control own only the native
input behavior it needs.
