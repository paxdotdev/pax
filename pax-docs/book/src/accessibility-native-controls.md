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
when shipping on web, Apple, or the alpha Windows/Linux hosts.

<a id="native-controls"></a>

`Button` accepts its label as a [text body](template-language.md#text-bodies),
for example `<Button width=120px height=40px>Save</Button>`. This sets the existing
`label` property; attributes and bindings remain available. On web, Button labels
continue to use the existing Markdown renderer. Text bodies do not change a
control's native accessibility or editing behavior.

On experimental Android, Button labels use the authored box without Android's
default internal button padding. Choose a height that fits the selected font and
leaves an appropriate touch target; a compact desktop-style button does not
automatically receive a larger invisible touch area.

`Toast` accepts its `message` as a body, and `ConfirmationDialog` accepts its
prompt `text` the same way:

```pax
<Toast width=100% y=100%>Changes saved.</Toast>
<ConfirmationDialog width=100% height=100% open=true>Discard these changes?</ConfirmationDialog>
```

These forms only assign the existing text properties. Toast keeps its existing
message-driven show/hide behavior, and the dialog still uses `open` to control
visibility. Whole-body expressions work too, such as
`<Toast width=100% y=100%>{self.status}</Toast>`; use the property attribute
when a two-way binding is needed.
Both components display light text on their dark panels. The dialog prompt
wraps within the area above its response buttons.

## A small editable form

Let's add a settings form to Field notes. A name, a location preference, a
visibility choice, and a detail level all belong to the component. Each
control edits one of those properties through `bind:`. A Save button calls
Rust when the reader is ready to apply their choices.

In `src/lib.rs`:

```rust
use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
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
    <Text class="label" width=100% height=28px>Notebook name</Text>
    <Textbox y=32px width=100% height=40px
        text=bind:self.name placeholder="e.g. Moss and rain" />

    <Checkbox y=92px width=24px height=24px
        checked=bind:self.include_location />
    <Text class="label" x=36px y=90px width={100% - 36px} height=28px>Include location</Text>

    <Text class="label" y=140px width=100% height=28px>Visibility</Text>
    <Dropdown y=172px width=100% height=40px
        options=["Private", "Shared"] selected_id=bind:self.visibility />

    <Text class="label" y=236px width=100% height=28px>{"Detail: " + self.detail}</Text>
    <Slider y=268px width=100% height=32px
        min=0.0 max=1.0 step=0.1 value=bind:self.detail />

    <Button y=324px width=180px height=44px
        color=rgb(56, 100, 78) hover_color=rgb(43, 79, 60)
        @button_click=self.save>Save notebook</Button>
    <Text class="label" y=388px width=100% height=64px>{self.message}</Text>
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

<a id="other-controls"></a>

## Choose a control

These controls are available through `use pax_kit::*;`. The event column lists
control-specific bindings; ordinary pointer events are covered in
[Event Handling](event-handling-rust.md).

| Control | Editable property or content | Control event |
| --- | --- | --- |
| [Button](api/pax-std/forms/button.md) | `label` supplies its visible title | `@button_click`; also `@submit` when `submit=true` |
| [Textbox](api/pax-std/forms/textbox.md) | `text: String` | `@textbox_input` while editing; `@textbox_change` when the platform commits a change; `@submit` for single-line Return |
| [Checkbox](api/pax-std/forms/checkbox.md) | `checked: bool` | `@checkbox_change`, with `event.checked` |
| [Slider](api/pax-std/forms/slider.md) | `value: f64`, bounded by `min` and `max` | `@slider_change`, with `event.value` |
| [Dropdown](api/pax-std/forms/dropdown.md) | `options: Vec<String>` and `selected_id: u32` | Bind `selected_id`; there is no dedicated public dropdown-change event |
| [RadioList](api/pax-std/forms/radio_list.md) | `options: Vec<String>` and `selected_id: u32` | Bind `selected_id`; there is no dedicated public radio-list-change event |

`selected_id` is a zero-based **index into `options`**, despite its name. The
first option is index `0`. Keep the index valid when replacing or reordering
the options; map it to your application's own identifier in Rust when needed.
On web, replacing options preserves the requested index even when the index
itself has not changed. An empty list or an out-of-range index shows no selected
option; refilling the list restores that index if it becomes valid again.

Multiple Sliders can bind to the same property, including Sliders inside a
repeated template. Native input updates their peers; synchronizing those peers
does not emit additional `@slider_change` events. A handler can replace an
incoming value, and the originating native control receives that correction.

On both web and Apple chassis, native input updates the relevant control
property before calling its TextboxInput, CheckboxChange, or SliderChange
handler. A bound parent property therefore already contains the new value.
Use an event handler for validation or side effects, without writing the
same value back just to maintain the binding.

On web, `@textbox_input` follows the browser's `input` event, while
`@textbox_change` follows `change`. Apple text controls report input during
editing and change when editing ends. Use input for immediate feedback and
`@submit` for a submission action from single-line Return or a Button with
`submit=true`; do not assume a change event means Enter was pressed. The bound
Textbox value is updated before submission handlers run. See
[Submit an action](event-handling-rust.md#submit-an-action) for Group handlers,
nested controls, and propagation. Likewise, a slider-change handler can run repeatedly
during a drag, so keep expensive work out of that immediate path.

### Multiline text and styling

For a longer note, add a `Property<String>` named `notes` and use
`multiline=true`:

```pax
<Textbox x=24px y=24px width={100% - 48px} height=140px
    multiline=true text=bind:self.notes
    background=rgb(249, 247, 239)
    stroke={paint: rgb(155, 170, 153), width: 1px}
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

### Filtered selection with ComboBox

[ComboBox](api/pax-std/forms/combo_box.md) combines an editable Textbox with a
Pax list. Add these fields to your form component, initializing them in its
`Default` implementation if it has one:

```rust
pub query: Property<String>,
pub choice: Property<Option<usize>>,
```

Use `Property::new(String::new())` and `Property::new(None)` for an initially
empty control. Then place it in the template:

```pax
<ComboBox width=240px height=36px
    options=["Alpha", "Beta", "Gamma"]
    text=bind:self.query selected=bind:self.choice
    background=WHITE stroke={paint: rgb(155, 170, 153), width: 1px}
    style={font_size: 16px, fill: rgb(36, 54, 47)} />
```

Click or tap the input to open the list, then type to filter it. Matching is
case-sensitive and uses substrings; prefix matches appear first, with
alphabetical order within each group. Selecting a row commits its original
zero-based index in `options`, copies its label to `query`, and closes the
list. For example, filtering to Gamma and choosing it sets `choice` to
`Some(2)`, even though only one row is visible. Separate ComboBox instances
keep their own selection unless you explicitly bind them to shared state.

`query` is the editable text, while `choice` is the **last committed
selection**. Typing, including an exact label match, does not select a row
or clear the previous selection. Leaving the field does not commit it either.
Do not treat a nonempty query as a valid selection or assume it still matches
the last choice. The clear icon resets both values.

Changing `options` updates the label at the selected index. An out-of-range
index becomes `None` and clears the text. Duplicate labels are supported,
but reordering the source list keeps the index, not an application's item
identity. If records have stable IDs, translate between those IDs and option
indices in your application when updating the list.

`new_item` controls the no-match row:

| Value | Behavior |
| --- | --- |
| `NewItem::Disallow` (default) | Shows a disabled “No Results Found” row. It does not prevent typing unmatched text. |
| `NewItem::AllowInvalid` | Shows no no-match row and preserves initial free text when no option is selected. It does not commit a new selection. |
| `NewItem::Text("Create item")` | Shows a custom row that dispatches `@new_item` when clicked or tapped. |

For the custom row, set `new_item=NewItem::Text("Create item")` and
`@new_item=self.create_item` on ComboBox. Define
`pub fn create_item(&mut self, ctx: &NodeContext)` on the containing component.
The handler reads the bound query and decides whether to add an option and
select it. The event carries no payload and does not add anything automatically.
See [custom events](event-handling-rust.md#dispatch-custom-events).

The composed list supports pointer selection on web and Apple chassis, but
it does not implement arrow-key/Enter list selection or a complete accessible
combobox contract. Native text editing does not supply those list behaviors.
Use [Dropdown](api/pax-std/forms/dropdown.md) when a native fixed-choice control
fits the task, and evaluate the accessibility needs of any editable selector.
[Tabs](api/pax-std/forms/tabs.md) and
[dialog components](api/pax-std/forms/dialogs.md) are also compositions; their
presence does not imply an operating-system widget or a complete accessible
interaction contract.

## Focus and keyboard interaction

Native text fields own editing, selection, and keyboard input while focused.
On macOS, native rendering may detach distant Text and NativeImage views inside
eligible scrollers, while retaining their instances. Editable, focused, and
selected text and form controls stay attached. VoiceOver or Switch Control
disables this culling and restores the full hierarchy. See
[Updating native content](compositing-effects.md#updating-native-content) for
the scope of this optimization.

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

On web and the alpha Windows/Linux hosts, a focused native control keeps
its keyboard input instead of also dispatching canvas shortcuts. A component's
`@key_down` is not a text-entry hook
or a way to make a drawn widget focusable. See
[Keyboard events](event-handling-rust.md#keyboard-delivery) for the app-level
event boundary.

On web, pointer activation of a native control is checked against the engine's
frontmost logical hit geometry. In a browser-owned scrolling page, swiping over
a covered label can still pan the page without selecting or activating the label.
On macOS, fully opaque punch-through coverage
also excludes the covered portion of a native control from pointer hit testing.
This lets an opaque 3D solid intercept a click where it hides a live control.
Moving coverage does not cancel an established native drag, remove keyboard
focus, or disable keyboard activation. Partially transparent coverage and
shader-expanded pixels do not define a new pixel-exact interaction contract.
A shared spatial group fade preserves internal opaque cutouts: a covered control
stays covered while its assembly fades. Independent native/canvas transparency
keeps the platform approximation. See
[spatial group fades](layout-responsiveness.md#spatial-group-fades) and
[opaque curved solids](layout-responsiveness.md#opaque-curved-solids).

## Accessibility today

Pax's native text and controls provide a starting point for accessibility.
On web, a Button is a browser button, Textbox uses an input or textarea,
Dropdown uses a select, and Checkbox and Slider use input elements. The
Apple chassis uses UIKit and AppKit controls, with custom styling and some
composed implementations.

Web buttons occupy their authored rectangle without an extra inline-text
baseline offset. Their label alignment is controlled by `TextStyle`, including
when a compact or custom-styled control uses an unusually small label font.

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
    <Text width=100% height=100%
        style={
            font: "Arial", font_size: 18px, fill: WHITE
            align_horizontal: HorizontalAlign::Center
            align_vertical: VerticalAlign::Center
        }>Choose photos</Text>
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

The canonical `examples/fixtures/photo-picker` project shows library and camera
selection, metadata, size limits, and thumbnail previews. From the repository
root, run it with:

```sh
pax-cli run --path examples/fixtures/photo-picker --target web
```

## Current support

The experimental Android host provides real TextView, EditText, Button,
CheckBox, SeekBar, Spinner, RadioGroup/RadioButton, ImageView, and native scrolling containers
for Text, Textbox, Button, Checkbox, Slider, Dropdown, RadioList, Frame,
Scroller, EventBlocker, and NativeImage. Text editing uses
Android's IME buffer. Native edits supersede older queued control values; later
Rust updates can still transform or replace the text. Textbox updates edit the
existing Android buffer so an application transformation preserves the active
input connection and selection.
Editable Text declares an editable native buffer and editor role before focus,
so Android's accessibility services can identify it as an editor.
The emulator acceptance suite checks composing replacements and commits through
Android's input connection, including combining accents, Arabic text, and emoji.
A separate Gboard probe exercises visible keys, suggestions, emoji, and Rust
bindings. These checks cover the installed keyboard configuration; other
keyboards and language layouts still need qualification.
TalkBack can navigate and activate native controls, but reading order, accessible
names and roles for custom interactions, and focus containment and return for
overlays remain incomplete. Test these tasks in the actual application; emulator
coverage does not replace physical-device qualification.
Hardware key events retain
native defaults unless a Pax handler prevents them. Nested scrollers choose the
drag axis, allowing a vertical drag over a horizontal row to reach its parent.
Transforms use Android View
properties so drawing, touch input, and accessibility bounds share the same map.
Radio indicators retain Android's native shape and thickness; checked fill and
unchecked outline colors tint those indicators. Button hover styling is not
implemented. Text supports native selection, editing, and
[Markdown/source highlighting](text-fonts-images.md#markdown-content).
Scroll snapping and complete accessibility qualification remain outside this
support. Follow the
[Android setup and limits](targets-build-deploy.md#experimental-android) and
use [Android fonts](text-fonts-images.md#fonts-on-android).

The alpha Windows and Linux hosts provide plain Text,
Textbox, Button, Checkbox, Slider, Dropdown, RadioList, Frame, Scroller, and
EventBlocker, and [NativeImage](text-fonts-images.md#native-images-and-target-boundaries). They use real
WinUI or GTK controls; styling and rich-text capabilities remain narrower than
the web/Apple paths described in this chapter. Other native control families
report unsupported errors. Do not infer full input-method, touch, screen-reader,
or keyboard-navigation qualification from basic editing and click tests. The
default Roboto font is packaged for offline use; other fonts need a system
family or a bundled file. See [desktop fonts](text-fonts-images.md#fonts-on-experimental-desktop-hosts)
and the [desktop prerequisites](targets-build-deploy.md#experimental-windows-and-linux-desktop).
Desktop Sliders require finite `min <= max`, a finite value, and a positive
finite step with a representable range/step quotient. Programmatic values are
clamped to the range without snapping to the step grid or generating input
events. Native user changes snap to a grid anchored at `min`. Keep authored
values on that grid when matching web, whose native range element can sanitize
off-grid values. Desktop Dropdown and RadioList preserve the requested index
across option replacement; an invalid index or an empty list has no native
selection and does not generate a sentinel selection event.
Dropdown popups are separate native UI surfaces. A correctly transformed or
masked closed control does not establish popup placement or clipping; test
those interactions separately on the intended desktop.
Windows Textbox input reaches Pax synchronously with the native edit, before
rendering can echo an older bound value. Application-driven text replacements
do not emit another `@textbox_input` event; an unchanged value preserves the
native selection. This ordering does not expand the IME qualification below.
The Windows host applies Button `color`, `hover_color`, `outline_stroke_color`,
and `outline_stroke_width` to the genuine WinUI control. Hover and pressed states
both use `hover_color`; reactive changes update the existing native brushes.
This does not establish parity for every native control's visual states.
The Linux host applies authored Button background, hover/pressed color, border
and radius through styles scoped to the genuine GTK control. Textbox applies
its background, border, focus outline and radius to single-line and multiline
editors. Reactive changes retain the editor and its content. Native focus and
input handling remain in GTK. Check contrast and state appearance on the
intended desktop; these paths do not establish visual parity for every control.
See [desktop text styling](text-fonts-images.md#text-styling-on-experimental-desktop-hosts)
for font, underline and alignment support by control type.
For enlarged text, leave the dimensions that should grow unspecified.
The desktop font guide describes live remeasurement on Windows and Linux,
the settings paths tested, and the limits of explicitly fixed dimensions.
The Linux fixture has passed real IBus/libpinyin composition, commit/cancel,
Unicode selection, and external AT-SPI text/button checks on the qualified
Ubuntu/NVIDIA configuration, including its GTK 4.22 Flatpak package. This
coverage does not establish other input engines or screen-reader workflows.
On Windows Server 2022, Microsoft Japanese IME checks cover real preedit,
candidate selection, commit/cancel, Unicode selection, and candidate positioning
after translating, rotating, or scrolling editors inside nested scrollers.
Closing during active composition also passes the resource-retirement check.
Composition during ongoing transforms, other Windows versions/input engines,
and screen-reader workflows remain unqualified.
On the qualified Server image, native UI Automation COM clients read selected
editor and selectable Text content successfully. The managed PowerShell text
range client crashes during equivalent readback while the app remains running;
this limits that automation path and does not establish screen-reader support.
Translucent vectors overlapping native controls also have unresolved
[desktop blending limits](targets-build-deploy.md#current-desktop-limits).

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

The [`native-controls` fixture](https://github.com/paxdotdev/pax/blob/main/examples/fixtures/native-controls/README.md)
exercises shared slider values, dynamic option lists, programmatic range changes,
transformed controls, and a rounded Scroller without a clipping Frame ancestor.
Use it alongside larger scenes; successful control bindings do not establish
popup compositing, scrolling latency, or screen-reader workflows.

## Read more

Continue to [Animation and Motion](animation-motion.md) for transitions and
animated feedback. [Compositing](compositing-effects.md) explains how native
controls and rendered content share clipping and stacking; [Scrolling](scrolling-viewports.md)
covers longer forms and content regions.

For more detail, see the [forms API](api/pax-std/forms.md),
[PhotoPicker API](api/pax-std/forms/photo_picker.md),
[event payloads](api/pax-runtime-api/events.md), and
[image sources](text-fonts-images.md#image-sources).

## Reading static web content

Web builds include a semantic HTML article for statically extractable content,
without adding a visible reading-mode button. Screen readers can traverse its
headings and links before or after the live scene initializes. The article is
visually clipped during normal use and revealed when a contained link receives
keyboard focus, so focus is not invisible. It is also shown without JavaScript
and when printing. Browser reading modes can extract this HTML when their
heuristics accept the page; Pax cannot force Reader availability or activation.
[Documentation directives](template-language.md#static-page-text-on-the-web)
provide headings and exclude demonstrations from this article. This is a reading
alternative for source-known text, not a complete accessibility representation
of interactive controls or runtime state.
