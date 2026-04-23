# pax-message
<!-- summary: API reference for pax-message. -->
<!-- tags: api, pax-message -->

## Structs
### `AddedLayerArgs`
Chassis acknowledgement that render layers were added.

#### Properties
##### `num_layers_added`
Type: `u32`

##### `layer_id`
Type: `Option`<`u32`>

---

### `AnyCreatePatch`
Common creation payload shared by native element families.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`u32`>

##### `occlusion_layer_id`
Type: `u32`

---

### `BrowserConfigInterruptArgs`
Browser capability flags discovered by the web chassis.

#### Properties
##### `allow_scroller_vector_layers`
Type: `bool`

##### `allow_nested_scroller_vector_layers`
Type: `bool`

---

### `ButtonPatch`
Create/update patch for a native button.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `hover_color`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `outline_stroke_color`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `outline_stroke_width`
Type: `Option`<`f64`>

##### `border_radius`
Type: `Option`<`f64`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `content`
Type: `Option`<`String`>

##### `color`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `style`
Type: `Option`<[`TextStyleMessage`](/api/internal/pax-message/index.md#textstylemessage)>

---

### `ChassisResizeRequestArgs`
Chassis response containing measured native control bounds.

#### Properties
##### `id`
Type: `u32`

##### `width`
Type: `f64`

##### `height`
Type: `f64`

---

### `CheckboxPatch`
Create/update patch for a native checkbox.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `background`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `background_checked`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `outline_color`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `outline_width`
Type: `Option`<`f64`>

##### `border_radius`
Type: `Option`<`f64`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `checked`
Type: `Option`<`bool`>

---

### `CheckboxStyleMessage`
Style payload shared by checkbox-like controls.

---

### `ClickOrTapInterruptArgs`
Pointer tap/click payload normalized to window coordinates.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

---

### `ClickInterruptArgs`
Click interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `button`
Type: [`MouseButtonMessage`](/api/internal/pax-message/index.md#mousebuttonmessage)

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `ContextMenuInterruptArgs`
Context-menu interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `button`
Type: [`MouseButtonMessage`](/api/internal/pax-message/index.md#mousebuttonmessage)

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `DoubleClickInterruptArgs`
Double-click interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `button`
Type: [`MouseButtonMessage`](/api/internal/pax-message/index.md#mousebuttonmessage)

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `DropFileArgs`
File-drop interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `name`
Type: `String`

##### `mime_type`
Type: `String`

##### `size`
Type: `u64`

---

### `DropdownPatch`
Create/update patch for a native dropdown/select element.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `selected_id`
Type: `Option`<`u32`>

##### `options`
Type: `Option`<`Vec`<`String`>>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `background`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `stroke_color`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `stroke_width`
Type: `Option`<`f64`>

##### `border_radius`
Type: `Option`<`f64`>

##### `style`
Type: `Option`<[`TextStyleMessage`](/api/internal/pax-message/index.md#textstylemessage)>

---

### `EventBlockerPatch`
Create/update patch for an invisible native hit-test blocker.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `opacity`
Type: `Option`<`f64`>

---

### `FocusInterruptArgs`
Focus interrupt marker payload.

---

### `FormButtonClickArgs`
Native button click payload.

#### Properties
##### `id`
Type: `u32`

---

### `FormCheckboxToggleArgs`
Checkbox state-change interrupt payload.

#### Properties
##### `state`
Type: `bool`

##### `id`
Type: `u32`

---

### `FormDropdownChangeArgs`
Dropdown selection-change payload.

#### Properties
##### `id`
Type: `u32`

##### `selected_id`
Type: `u32`

---

### `FormRadioListChangeArgs`
Radio-list selection-change payload.

#### Properties
##### `id`
Type: `u32`

##### `selected_id`
Type: `u32`

---

### `FormSliderChangeArgs`
Slider value-change payload.

#### Properties
##### `id`
Type: `u32`

##### `value`
Type: `f64`

---

### `FormTextboxChangeArgs`
Textbox committed-value change payload.

#### Properties
##### `text`
Type: `String`

##### `id`
Type: `u32`

---

### `FormTextboxInputArgs`
Textbox in-progress input payload.

#### Properties
##### `text`
Type: `String`

##### `id`
Type: `u32`

---

### `FramePatch`
Create/update patch for a native frame host.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `clip_content`
Type: `Option`<`bool`>

##### `border_radius`
Type: `Option`<`f64`>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `clip_path`
Type: `Option`<`String`>

##### `opacity`
Type: `Option`<`f64`>

##### `presented_bounds`
Type: `Option`<[`f64`; 4]>

##### `presented_clip_bounds`
Type: `Option`<[`f64`; 4]>

---

### `ImageDataArgs`
Image-load payload carrying image metadata without a byte pointer.

#### Properties
##### `id`
Type: `u32`

##### `path`
Type: `String`

##### `width`
Type: `usize`

##### `height`
Type: `usize`

---

### `ImagePatch`
Image-load request sent to the chassis.

#### Properties
##### `id`
Type: `u32`

##### `path`
Type: `Option`<`String`>

---

### `ImagePointerArgs`
Image-load payload carrying a pointer to chassis-owned image bytes.

#### Properties
##### `id`
Type: `u32`

##### `path`
Type: `String`

##### `image_data`
Type: `u64`

##### `image_data_length`
Type: `usize`

##### `width`
Type: `usize`

##### `height`
Type: `usize`

---

### `InterruptBuffer`
Raw FFI buffer containing serialized interrupts.

#### Properties
##### `data_ptr`
Type: *`const` `u8`

##### `length`
Type: `u64`

---

### `KeyDownInterruptArgs`
Key-down interrupt payload.

#### Properties
##### `key`
Type: `String`

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

##### `is_repeat`
Type: `bool`

---

### `KeyPressInterruptArgs`
Key-press interrupt payload.

#### Properties
##### `key`
Type: `String`

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

##### `is_repeat`
Type: `bool`

---

### `KeyUpInterruptArgs`
Key-up interrupt payload.

#### Properties
##### `key`
Type: `String`

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

##### `is_repeat`
Type: `bool`

---

### `LayerAddPatch`
Request to allocate additional native/canvas layers.

#### Properties
##### `num_layers_to_add`
Type: `usize`

---

### `LinkStyleMessage`
Serializable style payload for link text.

#### Properties
##### `font`
Type: `Option`<[`FontPatch`](/api/internal/pax-message/index.md#fontpatch)>

##### `fill`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `underline`
Type: `Option`<`bool`>

##### `size`
Type: `Option`<`f64`>

---

### `LocalFontMessage`
Local bundled font payload.

#### Properties
##### `family`
Type: `Option`<`String`>

##### `path`
Type: `Option`<`String`>

##### `style`
Type: `Option`<[`FontStyleMessage`](/api/internal/pax-message/index.md#fontstylemessage)>

##### `weight`
Type: `Option`<[`FontWeightMessage`](/api/internal/pax-message/index.md#fontweightmessage)>

---

### `MaskPathPatch`
One vector coverage path entry for a native occlusion mask.

#### Properties
##### `path`
Type: `String`

##### `clips`
Type: `Vec`<`String`>

##### `opacity`
Type: `Option`<`f64`>

---

### `MessageQueue`
Serializable batch of native messages emitted for one runtime tick.

#### Properties
##### `messages`
Type: `Vec`<[`NativeMessage`](/api/internal/pax-message/index.md#nativemessage)>

---

### `MouseDownInterruptArgs`
Mouse-down interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `button`
Type: [`MouseButtonMessage`](/api/internal/pax-message/index.md#mousebuttonmessage)

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `MouseMoveInterruptArgs`
Mouse-move interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `button`
Type: [`MouseButtonMessage`](/api/internal/pax-message/index.md#mousebuttonmessage)

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `MouseOutInterruptArgs`
Mouse-out interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `button`
Type: [`MouseButtonMessage`](/api/internal/pax-message/index.md#mousebuttonmessage)

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `MouseOverInterruptArgs`
Mouse-over interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `button`
Type: [`MouseButtonMessage`](/api/internal/pax-message/index.md#mousebuttonmessage)

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `MouseUpInterruptArgs`
Mouse-up interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `button`
Type: [`MouseButtonMessage`](/api/internal/pax-message/index.md#mousebuttonmessage)

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `NativeImagePatch`
Create/update patch for a native image element.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `url`
Type: `Option`<`String`>

##### `fit`
Type: `Option`<`String`>

---

### `NativeMaskPatch`
Create/update patch for a native occlusion mask surface.

#### Properties
##### `id`
Type: `u32`

##### `size_x`
Type: `f64`

##### `size_y`
Type: `f64`

##### `entries`
Type: `Vec`<[`MaskPathPatch`](/api/internal/pax-message/index.md#maskpathpatch)>

---

### `NativeMessageQueue`
Raw FFI buffer containing serialized native messages.

#### Properties
##### `data_ptr`
Type: *`mut` [`u8`]

##### `length`
Type: `u64`

---

### `NavigationPatch`
Request for the chassis to navigate to a URL.

#### Properties
##### `url`
Type: `String`

##### `target`
Type: `String`

---

### `RadioListPatch`
Create/update patch for a native radio list.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `selected_id`
Type: `Option`<`u32`>

##### `options`
Type: `Option`<`Vec`<`String`>>

##### `style`
Type: `Option`<[`TextStyleMessage`](/api/internal/pax-message/index.md#textstylemessage)>

##### `background_checked`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `outline_color`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `outline_width`
Type: `Option`<`f64`>

##### `background`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

---

### `ScreenshotData`
Completed screenshot bytes returned by the chassis.

#### Properties
##### `id`
Type: `u32`

##### `data`
Type: `Vec`<`u8`>

##### `width`
Type: `usize`

##### `height`
Type: `usize`

---

### `ScreenshotPatch`
Request to capture a screenshot from the chassis.

#### Properties
##### `id`
Type: `u32`

##### `scale`
Type: `Option`<`f64`>

---

### `ScrollInterruptArgs`
Wheel or gesture scroll delta payload.

#### Properties
##### `delta_x`
Type: `f64`

##### `delta_y`
Type: `f64`

---

### `ScrollerPatch`
Create/update patch for a native/browser-owned scroller.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `clip_content`
Type: `Option`<`bool`>

##### `border_radius`
Type: `Option`<`f64`>

##### `size_inner_pane_x`
Type: `Option`<`f64`>

##### `size_inner_pane_y`
Type: `Option`<`f64`>

##### `snap_points_x`
Type: `Option`<`Vec`<`f64`>>

##### `snap_points_y`
Type: `Option`<`Vec`<`f64`>>

##### `scroll_x`
Type: `Option`<`f64`>

##### `scroll_y`
Type: `Option`<`f64`>

##### `presentation_scroll_x`
Type: `Option`<`f64`>

##### `presentation_scroll_y`
Type: `Option`<`f64`>

##### `scroll_enabled_x`
Type: `Option`<`bool`>

##### `scroll_enabled_y`
Type: `Option`<`bool`>

##### `content_layer_id`
Type: `Option`<`u32`>

##### `presented_bounds`
Type: `Option`<[`f64`; 4]>

##### `presented_clip_bounds`
Type: `Option`<[`f64`; 4]>

##### `subtree_depth`
Type: `u32`

---

### `ScrollerPositionInterruptArgs`
Native scroller position payload, including optional presentation offsets.

#### Properties
##### `id`
Type: `u32`

##### `scroll_x`
Type: `f64`

##### `scroll_y`
Type: `f64`

##### `presentation_scroll_x`
Type: `Option`<`f64`>

##### `presentation_scroll_y`
Type: `Option`<`f64`>

---

### `SelectStartArgs`
Selection-start interrupt marker payload.

---

### `SetCursorPatch`
Request for the chassis to update cursor style.

#### Properties
##### `cursor`
Type: `String`

---

### `SliderPatch`
Create/update patch for a native slider.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `value`
Type: `Option`<`f64`>

##### `step`
Type: `Option`<`f64`>

##### `min`
Type: `Option`<`f64`>

##### `max`
Type: `Option`<`f64`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `accent`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `background`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `border_radius`
Type: `Option`<`f64`>

---

### `SystemFontMessage`
System font family payload.

#### Properties
##### `family`
Type: `Option`<`String`>

##### `style`
Type: `Option`<[`FontStyleMessage`](/api/internal/pax-message/index.md#fontstylemessage)>

##### `weight`
Type: `Option`<[`FontWeightMessage`](/api/internal/pax-message/index.md#fontweightmessage)>

---

### `TextInputArgs`
Raw text input payload delivered by a native text control.

#### Properties
##### `text`
Type: `String`

##### `id`
Type: `u32`

---

### `TextPatch`
Create/update patch for native or browser-managed text.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `content`
Type: `Option`<`String`>

##### `editable`
Type: `Option`<`bool`>

##### `selectable`
Type: `Option`<`bool`>

##### `clip`
Type: `Option`<`bool`>

##### `markdown`
Type: `Option`<`bool`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `style`
Type: `Option`<[`TextStyleMessage`](/api/internal/pax-message/index.md#textstylemessage)>

##### `style_link`
Type: `Option`<[`TextStyleMessage`](/api/internal/pax-message/index.md#textstylemessage)>

---

### `TextStyleMessage`
Serializable text style payload shared with chassis text renderers.

#### Properties
##### `font`
Type: `Option`<[`FontPatch`](/api/internal/pax-message/index.md#fontpatch)>

##### `font_size`
Type: `Option`<`f64`>

##### `fill`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `underline`
Type: `Option`<`bool`>

##### `align_multiline`
Type: `Option`<[`TextAlignHorizontalMessage`](/api/internal/pax-message/index.md#textalignhorizontalmessage)>

##### `align_vertical`
Type: `Option`<[`TextAlignVerticalMessage`](/api/internal/pax-message/index.md#textalignverticalmessage)>

##### `align_horizontal`
Type: `Option`<[`TextAlignHorizontalMessage`](/api/internal/pax-message/index.md#textalignhorizontalmessage)>

---

### `TextboxPatch`
Create/update patch for a native textbox.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `text`
Type: `Option`<`String`>

##### `background`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `stroke_color`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `stroke_width`
Type: `Option`<`f64`>

##### `border_radius`
Type: `Option`<`f64`>

##### `style`
Type: `Option`<[`TextStyleMessage`](/api/internal/pax-message/index.md#textstylemessage)>

##### `focus_on_mount`
Type: `Option`<`bool`>

##### `placeholder`
Type: `Option`<`String`>

##### `outline_color`
Type: `Option`<[`ColorMessage`](/api/internal/pax-message/index.md#colormessage)>

##### `outline_width`
Type: `Option`<`f64`>

##### `is_text_area`
Type: `Option`<`bool`>

---

### `TouchEndInterruptArgs`
Touch-end interrupt payload.

#### Properties
##### `touches`
Type: `Vec`<[`TouchMessage`](/api/internal/pax-message/index.md#touchmessage)>

---

### `TouchMessage`
One touch point in a multi-touch interrupt.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `identifier`
Type: `i64`

##### `delta_x`
Type: `f64`

##### `delta_y`
Type: `f64`

---

### `TouchMoveInterruptArgs`
Touch-move interrupt payload.

#### Properties
##### `touches`
Type: `Vec`<[`TouchMessage`](/api/internal/pax-message/index.md#touchmessage)>

---

### `TouchStartInterruptArgs`
Touch-start interrupt payload.

#### Properties
##### `touches`
Type: `Vec`<[`TouchMessage`](/api/internal/pax-message/index.md#touchmessage)>

---

### `VisualViewportUpdateArgs`
Browser visual viewport payload for page-scroll-backed root scrollers.

#### Properties
##### `width`
Type: `f64`

##### `height`
Type: `f64`

##### `offset_x`
Type: `f64`

##### `offset_y`
Type: `f64`

##### `page_scroll_x`
Type: `f64`

##### `page_scroll_y`
Type: `f64`

---

### `WebFontMessage`
Web font payload, including family and source URL.

#### Properties
##### `family`
Type: `Option`<`String`>

##### `url`
Type: `Option`<`String`>

##### `style`
Type: `Option`<[`FontStyleMessage`](/api/internal/pax-message/index.md#fontstylemessage)>

##### `weight`
Type: `Option`<[`FontWeightMessage`](/api/internal/pax-message/index.md#fontweightmessage)>

---

### `WheelInterruptArgs`
Wheel interrupt payload.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

##### `delta_x`
Type: `f64`

##### `delta_y`
Type: `f64`

##### `modifiers`
Type: `Vec`<[`ModifierKeyMessage`](/api/internal/pax-message/index.md#modifierkeymessage)>

---

### `YoutubeVideoPatch`
Create/update patch for an embedded YouTube video.

#### Properties
##### `id`
Type: `u32`

##### `parent_frame`
Type: `Option`<`Option`<`u32`>>

##### `z_index`
Type: `Option`<`i32`>

##### `transform`
Type: `Option`<`Vec`<`f64`>>

##### `size_x`
Type: `Option`<`f64`>

##### `size_y`
Type: `Option`<`f64`>

##### `opacity`
Type: `Option`<`f64`>

##### `url`
Type: `Option`<`String`>

## Enums
### `ColorMessage`
Serializable color payload for native/chassis messages.

#### Variants
##### `Rgba`([`f64`; 4])
##### `Rgb`([`f64`; 3])
---

### `FontPatch`
Serializable font selection payload.

#### Variants
##### `System`([`SystemFontMessage`](/api/internal/pax-message/index.md#systemfontmessage))
##### `Web`([`WebFontMessage`](/api/internal/pax-message/index.md#webfontmessage))
##### `Local`([`LocalFontMessage`](/api/internal/pax-message/index.md#localfontmessage))
---

### `FontStyleMessage`
Serializable font-style value.

#### Variants
##### `Normal`
##### `Italic`
##### `Oblique`
---

### `FontWeightMessage`
Serializable font-weight value.

#### Variants
##### `Thin`
##### `ExtraLight`
##### `Light`
##### `Normal`
##### `Medium`
##### `SemiBold`
##### `Bold`
##### `ExtraBold`
##### `Black`
---

### `ImageLoadInterruptArgs`
Image-load response, either by pointer or copied metadata.

#### Variants
##### `Reference`([`ImagePointerArgs`](/api/internal/pax-message/index.md#imagepointerargs))
##### `Data`([`ImageDataArgs`](/api/internal/pax-message/index.md#imagedataargs))
---

### `ModifierKeyMessage`
Normalized keyboard modifier identifier.

#### Variants
##### `Shift`
##### `Control`
##### `Alt`
##### `Command`
---

### `MouseButtonMessage`
Normalized mouse button identifier.

#### Variants
##### `Left`
##### `Right`
##### `Middle`
##### `Unknown`
---

### `NativeInterrupt`
Events and data packets sent from the chassis back into the Pax runtime.

#### Variants
##### `ChassisResizeRequestCollection`(`Vec`<[`ChassisResizeRequestArgs`](/api/internal/pax-message/index.md#chassisresizerequestargs)>)
##### `SelectStart`([`SelectStartArgs`](/api/internal/pax-message/index.md#selectstartargs))
##### `Focus`([`FocusInterruptArgs`](/api/internal/pax-message/index.md#focusinterruptargs))
##### `ClickOrTap`([`ClickOrTapInterruptArgs`](/api/internal/pax-message/index.md#clickortapinterruptargs))
##### `Scroll`([`ScrollInterruptArgs`](/api/internal/pax-message/index.md#scrollinterruptargs))
##### `TouchStart`([`TouchStartInterruptArgs`](/api/internal/pax-message/index.md#touchstartinterruptargs))
##### `TouchMove`([`TouchMoveInterruptArgs`](/api/internal/pax-message/index.md#touchmoveinterruptargs))
##### `TouchEnd`([`TouchEndInterruptArgs`](/api/internal/pax-message/index.md#touchendinterruptargs))
##### `KeyDown`([`KeyDownInterruptArgs`](/api/internal/pax-message/index.md#keydowninterruptargs))
##### `KeyUp`([`KeyUpInterruptArgs`](/api/internal/pax-message/index.md#keyupinterruptargs))
##### `KeyPress`([`KeyPressInterruptArgs`](/api/internal/pax-message/index.md#keypressinterruptargs))
##### `Click`([`ClickInterruptArgs`](/api/internal/pax-message/index.md#clickinterruptargs))
##### `DoubleClick`([`DoubleClickInterruptArgs`](/api/internal/pax-message/index.md#doubleclickinterruptargs))
##### `MouseMove`([`MouseMoveInterruptArgs`](/api/internal/pax-message/index.md#mousemoveinterruptargs))
##### `Wheel`([`WheelInterruptArgs`](/api/internal/pax-message/index.md#wheelinterruptargs))
##### `MouseDown`([`MouseDownInterruptArgs`](/api/internal/pax-message/index.md#mousedowninterruptargs))
##### `MouseUp`([`MouseUpInterruptArgs`](/api/internal/pax-message/index.md#mouseupinterruptargs))
##### `ContextMenu`([`ContextMenuInterruptArgs`](/api/internal/pax-message/index.md#contextmenuinterruptargs))
##### `Image`([`ImageLoadInterruptArgs`](/api/internal/pax-message/index.md#imageloadinterruptargs))
##### `AddedLayer`([`AddedLayerArgs`](/api/internal/pax-message/index.md#addedlayerargs))
##### `TextInput`([`TextInputArgs`](/api/internal/pax-message/index.md#textinputargs))
##### `FormCheckboxToggle`([`FormCheckboxToggleArgs`](/api/internal/pax-message/index.md#formcheckboxtoggleargs))
##### `FormDropdownChange`([`FormDropdownChangeArgs`](/api/internal/pax-message/index.md#formdropdownchangeargs))
##### `FormSliderChange`([`FormSliderChangeArgs`](/api/internal/pax-message/index.md#formsliderchangeargs))
##### `FormRadioListChange`([`FormRadioListChangeArgs`](/api/internal/pax-message/index.md#formradiolistchangeargs))
##### `FormTextboxChange`([`FormTextboxChangeArgs`](/api/internal/pax-message/index.md#formtextboxchangeargs))
##### `FormTextboxInput`([`FormTextboxInputArgs`](/api/internal/pax-message/index.md#formtextboxinputargs))
##### `FormButtonClick`([`FormButtonClickArgs`](/api/internal/pax-message/index.md#formbuttonclickargs))
##### `ScrollerPosition`([`ScrollerPositionInterruptArgs`](/api/internal/pax-message/index.md#scrollerpositioninterruptargs))
##### `BrowserConfig`([`BrowserConfigInterruptArgs`](/api/internal/pax-message/index.md#browserconfiginterruptargs))
##### `VisualViewportUpdate`([`VisualViewportUpdateArgs`](/api/internal/pax-message/index.md#visualviewportupdateargs))
##### `DropFile`([`DropFileArgs`](/api/internal/pax-message/index.md#dropfileargs))
##### `Screenshot`([`ImageLoadInterruptArgs`](/api/internal/pax-message/index.md#imageloadinterruptargs))
---

### `NativeMessage`
Messages emitted by the runtime to create, update, delete, or configure native/chassis resources.

#### Variants
##### `TextCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `TextUpdate`([`TextPatch`](/api/internal/pax-message/index.md#textpatch))
##### `TextDelete`(`u32`)
##### `FrameCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `FrameUpdate`([`FramePatch`](/api/internal/pax-message/index.md#framepatch))
##### `FrameDelete`(`u32`)
##### `EventBlockerCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `EventBlockerUpdate`([`EventBlockerPatch`](/api/internal/pax-message/index.md#eventblockerpatch))
##### `EventBlockerDelete`(`u32`)
##### `CheckboxCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `CheckboxUpdate`([`CheckboxPatch`](/api/internal/pax-message/index.md#checkboxpatch))
##### `CheckboxDelete`(`u32`)
##### `NativeImageCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `NativeImageUpdate`([`NativeImagePatch`](/api/internal/pax-message/index.md#nativeimagepatch))
##### `NativeImageDelete`(`u32`)
##### `YoutubeVideoCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `YoutubeVideoUpdate`([`YoutubeVideoPatch`](/api/internal/pax-message/index.md#youtubevideopatch))
##### `YoutubeVideoDelete`(`u32`)
##### `TextboxCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `TextboxUpdate`([`TextboxPatch`](/api/internal/pax-message/index.md#textboxpatch))
##### `TextboxDelete`(`u32`)
##### `SliderCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `SliderUpdate`([`SliderPatch`](/api/internal/pax-message/index.md#sliderpatch))
##### `SliderDelete`(`u32`)
##### `DropdownCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `DropdownUpdate`([`DropdownPatch`](/api/internal/pax-message/index.md#dropdownpatch))
##### `DropdownDelete`(`u32`)
##### `RadioListCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `RadioListUpdate`([`RadioListPatch`](/api/internal/pax-message/index.md#radiolistpatch))
##### `RadioListDelete`(`u32`)
##### `ButtonCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `ButtonUpdate`([`ButtonPatch`](/api/internal/pax-message/index.md#buttonpatch))
##### `ButtonDelete`(`u32`)
##### `ScrollerCreate`([`AnyCreatePatch`](/api/internal/pax-message/index.md#anycreatepatch))
##### `ScrollerUpdate`([`ScrollerPatch`](/api/internal/pax-message/index.md#scrollerpatch))
##### `ScrollerDelete`(`u32`)
##### `ImageLoad`([`ImagePatch`](/api/internal/pax-message/index.md#imagepatch))
##### `LayerAdd`([`LayerAddPatch`](/api/internal/pax-message/index.md#layeraddpatch))
##### `ShrinkLayersTo`(`u32`)
##### `NativeMaskUpdate`([`NativeMaskPatch`](/api/internal/pax-message/index.md#nativemaskpatch))
##### `Navigate`([`NavigationPatch`](/api/internal/pax-message/index.md#navigationpatch))
##### `SetCursor`([`SetCursorPatch`](/api/internal/pax-message/index.md#setcursorpatch))
##### `Screenshot`([`ScreenshotPatch`](/api/internal/pax-message/index.md#screenshotpatch))
---

### `TextAlignHorizontalMessage`
Serializable horizontal text alignment.

#### Variants
##### `Left`
##### `Center`
##### `Right`
---

### `TextAlignVerticalMessage`
Serializable vertical text alignment.

#### Variants
##### `Top`
##### `Center`
##### `Bottom`
