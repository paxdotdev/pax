# forms::photo_picker
<!-- summary: API docs for pax-std::forms::photo_picker. -->
<!-- tags: api, pax-std -->

## Structs
### `PhotoPicker`
Opens the platform photo picker when its slotted content is activated.

`PhotoPicker` renders its children as the visible affordance and layers a
transparent native hit target over them. Increment `trigger` to request a
programmatic open; on the web, direct user activation of the picker element is
the most reliable way to satisfy browser file-picker requirements.

#### Properties
##### `trigger`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`u64`>

Incrementing request value. Returned as `request_id` in `photo_picker_change`.

##### `source`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`PhotoPickerSource`](/api/pax-std/forms/photo_picker.md#photopickersource)>

Requested platform source.

##### `allow_multiple`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether multiple images may be selected.

##### `accept`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Accepted MIME/file filter. Web uses this as the input `accept` value.

##### `include_bytes`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether the chassis should copy bytes into the event when practical.

##### `max_bytes_per_photo`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`u64`>

Maximum copied/read bytes per selected photo. Larger photos report a size-limit status.

## Enums
### `PhotoPickerSource`
Source requested when opening a `PhotoPicker`.

#### Variants
##### `Library`
Let the user choose existing images from the platform photo/file picker.

##### `Camera`
Request camera capture where the current platform supports it.
