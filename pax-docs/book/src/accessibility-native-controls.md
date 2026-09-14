<a id="input--native-controls"></a>

# Accessibility and Native Controls
<!-- summary: Editable forms, native control bindings, focus, photo selection, and the current accessibility foundation. -->
<!-- tags: accessibility, input, controls, forms, focus, photo-picker -->

Native controls bring familiar editing and selection behavior into a Pax
interface: entering a note, choosing an option, moving a slider, or opening a
photo library. They share the scene's layout with your text, drawings, and
images, while the browser or operating system supplies their underlying
controls.

This chapter builds on [Properties](state-properties.md) and
[Event Handling](event-handling-rust.md). It starts with a small form, then
covers keyboard interaction, accessibility, and photo selection. The
[current support](#current-support) section identifies differences to check
when shipping on web, macOS, iOS, or iPadOS.

<a id="native-controls"></a>

## A small editable form

Let's add a settings form to Field notes. A name, a location preference, a
visibility choice, and a detail level all belong to the component. Each
control edits one of those properties through `bind:`. A Save button calls
Rust when the reader is ready to apply their choices.

In `src/lib.rs`:

```rust
use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub name: Property<String>,
    pub include_location: Property<bool>,
    pub visibility: Property<u32>,
    pub detail: Property<f64>,
    pub message: Property<String>,
}

impl Example {
    pub fn save(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        let name = self.name.get();
        if name.trim().is_empty() {
            self.message.set("Give this notebook a name first.".into());
            return;
        }

        let visibility = match self.visibility.get() {
            1 => "shared",
            _ => "private",
        };
        let location = if self.include_location.get() { "on" } else { "off" };
        self.message.set(format!(
            "{}: {}, location {}, detail {:.1}.",
            name.trim(), visibility, location, self.detail.get()
        ));
    }
}
```

In `src/lib.pax`:

```pax
<Group x=24px y=24px width={100% - 48px} height=452px>
    <Text class="label" width=100% height=28px text="Notebook name" />
    <Textbox y=32px width=100% height=40px
        text=bind:self.name placeholder="e.g. Moss and rain" />

    <Checkbox y=92px width=24px height=24px
        checked=bind:self.include_location />
    <Text class="label" x=36px y=90px width={100% - 36px} height=28px
        text="Include location" />

    <Text class="label" y=140px width=100% height=28px text="Visibility" />
    <Dropdown y=172px width=100% height=40px
        options=["Private", "Shared"] selected_id=bind:self.visibility />

    <Text class="label" y=236px width=100% height=28px
        text={"Detail: " + self.detail} />
    <Slider y=268px width=100% height=32px
        min=0.0 max=1.0 step=0.1 value=bind:self.detail />

    <Button y=324px width=180px height=44px label="Save notebook"
        color=rgb(56, 100, 78) hover_color=rgb(43, 79, 60)
        @button_click=self.save />
    <Text class="label" y=388px width=100% height=64px text={self.message} />
</Group>

@settings {
    .label {
        style: {font: "Arial", font_size: 16px, fill: rgb(36, 54, 47)}
    }
}
```

Try saving before entering a name, then change the fields and save again.
The detail label updates while the slider moves. The message changes only
when Save runs. This sample reports the choices on screen; a real app would
also persist or submit them from Rust.

`bind:self.name` connects the Textbox's editable property to the component's
property. Typing updates both. A one-way expression such as `text={self.name}`
supplies a value to the control without making it an editor for the parent
property. Bind directly to state; put formatting and derived display values
in separate expressions. See [Two-way bindings](data-binding-expressions.md#reading-versus-writing)
for the underlying model.

The visible labels above help someone looking at the form. They are separate
Text elements: Pax does not currently turn that proximity into a semantic
label association for assistive technology. Keep the
[accessibility boundary](#accessibility-today) in mind when adapting the form.

<div class="docs-example-placeholder">
<p><strong>Interactive example planned:</strong> edit these settings and watch the bound properties change beside the form.</p>
<!-- Production brief:
- Use the article's exact form and source tabs; show live properties separately
  from the saved summary. Include empty-name validation and narrow/wide layouts.
- Three treatments: Field notes settings; a sound-check panel; a reading-list
  editor. Prefer Field notes to connect the content and drawing chapters.
- Add keyboard instructions and a static fallback. Explain the current label
  association gap; do not present the fixture as an accessibility certification. -->
</div>

<a id="other-controls"></a>

## Choose a control

These controls are available through `use pax_kit::*;`. The event column lists
control-specific bindings; ordinary pointer events are covered in
[Event Handling](event-handling-rust.md).

| Control | Editable property or content | Control event |
| --- | --- | --- |
| [Button](api/pax-std/forms/button.md) | `label` supplies its visible title | `@button_click`, with `Event<ButtonClick>` |
| [Textbox](api/pax-std/forms/textbox.md) | `text: String` | `@textbox_input` while editing; `@textbox_change` when the platform commits a change |
| [Checkbox](api/pax-std/forms/checkbox.md) | `checked: bool` | `@checkbox_change`, with `event.checked` |
| [Slider](api/pax-std/forms/slider.md) | `value: f64`, bounded by `min` and `max` | `@slider_change`, with `event.value` |
| [Dropdown](api/pax-std/forms/dropdown.md) | `options: Vec<String>` and `selected_id: u32` | Bind `selected_id`; there is no dedicated public dropdown-change event |
| [RadioList](api/pax-std/forms/radio_list.md) | `options: Vec<String>` and `selected_id: u32` | Bind `selected_id`; there is no dedicated public radio-list-change event |

`selected_id` is a zero-based **index into `options`**, despite its name. The
first option is index `0`. Keep the index valid when replacing or reordering
the options; map it to your application's own identifier in Rust when needed.

On both web and Apple chassis, native input updates the relevant control
property before calling its TextboxInput, CheckboxChange, or SliderChange
handler. A bound parent property therefore already contains the new value.
Use an event handler for validation or side effects, without writing the
same value back just to maintain the binding.

On web, `@textbox_input` follows the browser's `input` event, while
`@textbox_change` follows `change`. Apple text controls report input during
editing and change when editing ends. Use input for immediate feedback and
an explicit Button for a submission action; do not assume a change event
means Enter was pressed. Likewise, a slider-change handler can run repeatedly
during a drag, so keep expensive work out of that immediate path.

### Multiline text and styling

For a longer note, add a `Property<String>` named `notes` and use
`multiline=true`:

```pax
<Textbox x=24px y=24px width={100% - 48px} height=140px
    multiline=true text=bind:self.notes
    background=rgb(249, 247, 239)
    stroke={color: rgb(155, 170, 153), width: 1px}
    corner_radius=8
    style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />
```

Give a multiline editor enough height for its contents. A placeholder is
available for single-line Textbox on web and Apple, and multiline Textbox on
web; the current Apple multiline views do not display one. Keep a persistent
visible label when the field needs an explanation.

Native controls expose their own styling properties. Button uses `color`,
`hover_color`, `outline`, `corner_radius`, and `style`; Textbox uses
`background`, `stroke`, `outline`, `corner_radius`, and `style`. Checkbox has
separate unchecked and checked backgrounds, while Slider exposes an `accent`.
Consult the control's API for its exact property names and defaults. The
platform still influences its appearance, especially focus and hover states.

An ordinary Button renders its own label and surface. For a custom
composition, build a component and handle its interactions deliberately;
adding a click handler to a drawing does not supply a button's keyboard and
assistive-technology behavior. PhotoPicker's slotted design below is a
specific API for a custom visible affordance.

[ComboBox](api/pax-std/forms/combo_box.md), [Tabs](api/pax-std/forms/tabs.md),
and [dialog components](api/pax-std/forms/dialogs.md) combine Pax elements and
controls into larger patterns. Their presence in the library does not imply
an operating-system dialog or a complete accessible widget contract.

## Focus and keyboard interaction

Native text fields own editing, selection, and keyboard input while focused.
To request focus when a Textbox appears, set `focus_on_mount=true`. This can
be useful for a deliberate editing action; avoid taking focus automatically
from someone already navigating the page. On a mobile device, focusing a
field can also bring up the software keyboard.

In the web chassis, native controls participate in browser keyboard
navigation. The current implementation derives their tab indices from
visual stacking order; Pax does not yet offer an independent public
tab-order property. Changes to element order, conditional content, or
compositing can therefore affect the path through the form. Walk that path
with a keyboard in the actual application.

While a native element has focus on web, the chassis leaves its keyboard
input with that element. A component's `@key_down` is not a text-entry hook
or a way to make a drawn widget focusable. See
[Keyboard events](event-handling-rust.md#keyboard-delivery) for the app-level
event boundary.

## Accessibility today

Pax's native text and controls provide a starting point for accessibility.
On web, a Button is a browser button, Textbox uses an input or textarea,
Dropdown uses a select, and Checkbox and Slider use input elements. The
Apple chassis uses UIKit and AppKit controls, with custom styling and some
composed implementations.

The broader accessibility model is still incomplete. Public authoring APIs
do not yet provide a general contract for accessible names, descriptions,
roles, label associations, or a reading order independent of visual layout.
Image and NativeImage currently expose no alternative-text property. A
custom-drawn control has no automatically generated semantic counterpart.
Native backing alone also does not establish that every state is announced
correctly on every platform.

For an application, check the actual tasks a person must complete:

- Navigate the whole interaction with a keyboard, including opening and
  dismissing transient UI. Check where focus goes afterward.
- Use the target's screen reader to inspect names, values, selected states,
  and reading order. Check whether validation messages are discoverable.
- Keep instructions and errors visible in text, and check that zoomed or
  larger text still fits the layout. Avoid communicating a state through
  color alone.

These checks can reveal a requirement that needs additional engine work.
If your product depends on a particular accessibility contract, verify it
early against the current implementation. This chapter makes no blanket
screen-reader or standards-conformance claim.

<div class="docs-media-placeholder">
<p><strong>Diagram planned:</strong> compare a form's visual arrangement, keyboard path, and accessible names.</p>
<!-- Production brief:
- Show three separate observations of the same form. Mark a visible label
  without a semantic association and a custom drawing without a native role.
- Three treatments: stacked annotated screenshots; a three-column comparison;
  a keyboard walkthrough with a screen-reader transcript. Prefer the comparison.
- Base the accessible view on a recorded audit, not inferred native semantics.
  Provide text equivalents for the final diagram. -->
</div>

## PhotoPicker

PhotoPicker lets someone select images for the application. Its children
draw the visible affordance; a transparent native control above them opens
the platform picker. Keep those children simple and give the whole picker
a usable size:

```pax
<PhotoPicker x=24px y=24px width=240px height=48px
    source=PhotoPickerSource::Library allow_multiple=true
    include_bytes=false max_bytes_per_photo=26214400
    @photo_picker_change=self.choose_photos>
    <Text width=100% height=100% text="Choose photos"
        style={
            font: "Arial", font_size: 18px, fill: WHITE
            align_horizontal: HorizontalAlign::Center
            align_vertical: VerticalAlign::Center
        } />
    <Rectangle width=100% height=100% corner_radius=8
        fill=rgb(56, 100, 78) />
</PhotoPicker>
```

The default `accept` filter is `"image/*"`. On web it is passed to the file
input; Apple's current pickers select images without applying a custom
`accept` filter. Use `allow_multiple=false` when only one existing photo is
needed. Camera capture produces one photo at a time.

### Handle selection and partial results

The completion event includes `status`, an optional `message`, `request_id`,
and a list of `photos`. Add `Property<String>` fields named `preview_url` and
`message` to the component for this handler:

```rust
pub fn choose_photos(&mut self, _ctx: &NodeContext, event: Event<PhotoPickerChange>) {
    if event.status == PhotoPickerStatus::Cancelled {
        return;
    }

    if event.status == PhotoPickerStatus::Selected {
        if let Some(handle) = event.photos.first().and_then(|photo| photo.handle.as_ref()) {
            self.preview_url.set(handle.clone());
        }
        self.message.set(event.message.clone().unwrap_or_else(|| {
            format!("Selected {} photo(s).", event.photos.len())
        }));
    } else {
        self.message.set(event.message.clone().unwrap_or_else(|| {
            "No photo was added. Try another image or source.".into()
        }));
    }
}
```

`Selected` can include a warning: if some files exceed the configured size
limit, the accepted files can still be returned. `SizeLimitExceeded` reports
a selection in which the size limit prevented any accepted photos. Other
statuses are `Cancelled`, `PermissionDenied`, `Unavailable`, and `Failed`.
Keep the person's existing selection when a request is cancelled. Browser
dismissal does not always deliver a completion event in the current web
implementation, so do not depend on every close resetting an app-level
“picker open” flag.

Each photo includes a temporary identifier, MIME type, byte size, source
kind, and optional file name, dimensions, preview handle, and copied bytes.
Show a returned handle with NativeImage:

```pax
if self.preview_url != "" {
    <NativeImage x=24px y=96px width=240px height=160px
        url={self.preview_url} fit=ImageFit::Contain />
}
```

Handles are transient object URLs or temporary file URLs. Persist an
app-owned copy if the image must survive beyond the current session.
Selection does not upload a file; networking and storage belong to your
Rust application logic.

`include_bytes=true` asks for copied data in `photo.data`. Web supplies it
when the read succeeds; the current Apple implementation returns metadata
and file handles without copied bytes in the event. Treat `data` as optional.
`include_bytes=false` avoids that web byte copy when a preview is enough.
The default `max_bytes_per_photo` is 25 MiB (26,214,400 bytes); a positive
limit is checked even when `include_bytes` is false. It is a per-photo
limit, so also consider how many selections your app retains.

### Library, camera, and permissions

| Target | `PhotoPickerSource::Library` | `PhotoPickerSource::Camera` |
| --- | --- | --- |
| Web | Browser file picker with image filtering and optional multiple selection | Requests the file input's camera-capture hint; the browser/device decides what UI is available |
| macOS | Image-file selection through NSOpenPanel | Returns `Unavailable` |
| iOS and iPadOS | System photo picker with optional multiple selection | System camera UI on a capable device, subject to permission |

On iOS and iPadOS, library selection uses the system's scoped photo picker.
The camera path needs a purpose string in the project's `Cargo.toml`:

```toml
[package.metadata.pax.ios.info_plist]
NSCameraUsageDescription = "Add a photo to your field notes."
```

iPadOS inherits the iOS metadata unless
`[package.metadata.pax.ipados.info_plist]` overrides it. Check permission
denial and unavailable hardware as well as a successful capture; a simulator
cannot stand in for every device capability.

Incrementing the picker's `trigger` property requests a programmatic open,
and the value is returned as `request_id`. On web, prefer direct activation
of the picker itself: browsers restrict opening file dialogs without a
user gesture, so a delayed property update is not a reliable substitute.

The canonical `examples/src/photo-picker` project shows library and camera
selection, metadata, size limits, and thumbnail previews. From the repository
root, run it with:

```sh
pax-cli run --path examples/src/photo-picker --target web
```

<div class="docs-example-placeholder">
<p><strong>Interactive example planned:</strong> the canonical PhotoPicker example, with source and an explanation of the current platform's picker.</p>
<!-- Production brief:
- Reuse examples/src/photo-picker without duplicating its source. Keep the
  chooser user-initiated. Show optional bytes, partial results, and cancellation.
- Three treatments: the existing metadata inspector; a Field notes photo inset;
  a contact-sheet composer. Prefer the existing example for current coverage.
- Include target-labeled screenshots and a static fallback; never imply the
  embedded web picker verifies camera permission or accessibility on Apple. -->
</div>

## Current support

Button, Textbox (single-line and multiline), Checkbox, Slider, Dropdown, and
RadioList have implementations for web, macOS, iOS, and iPadOS. Their shared
Pax API does not make every platform detail identical:

- Web Slider applies `step`; the current Apple sliders are continuous and
  do not apply it. If discrete values are essential to your interaction,
  this requires a platform-specific check before adopting the control.
- Native styling and interaction states vary. For example, Button's
  `hover_color` is applied on web but not by the current Apple button views.
- Multiline placeholders, photo bytes, and camera availability have the
  limits described above.
- Accessibility remains a partial foundation. Test required keyboard and
  assistive-technology behavior on each shipping target.

## Read more

Continue to [Animation and Motion](animation-motion.md) for transitions and
animated feedback. [Compositing](compositing-effects.md) explains how native
controls and rendered content share clipping and stacking; [Scrolling](scrolling-viewports.md)
covers longer forms and content regions.

For more detail, see the [forms API](api/pax-std/forms.md),
[PhotoPicker API](api/pax-std/forms/photo_picker.md),
[event payloads](api/pax-runtime-api/events.md), and
[image sources](text-fonts-images.md#image-sources).
