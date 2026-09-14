<a id="text-fonts--images"></a>
<a id="text-images-and-assets"></a>

# Text, Fonts, and Images
<!-- summary: Give content a readable hierarchy, size and style native text, load fonts and image assets, and choose image fitting behavior. -->
<!-- tags: text, fonts, images, assets, typography, wrapping -->

A heading, a paragraph, and an image give an interface much of its character.
They also bring their own constraints: a sentence can wrap, a font can arrive
after the first frame, and a photograph has proportions worth preserving.

This chapter covers those content decisions. It builds on the local bounds
and containers in [Layout](layout-responsiveness.md), with examples you can
place in a component's template. Keep `use pax_kit::*;` in its Rust file.

## Display text

`Text` displays a string through the target's native text system. On web,
that means DOM text; macOS, iOS, and iPadOS use the native text rendering
provided by their chassis. Pax positions and composites that content alongside
drawings and images.

Use `style` to set the type's appearance. A title and a short description can
share a family and color while differing in size and weight:

```pax
<Group x=24px y=24px width={100% - 48px} height=180px>
    <Text width=100% height=40px text="Field notes" class="heading" />
    <Text y=56px width=100%
        text={"Collect a small observation. Give it a title, " +
            "then leave room for the next idea."}
        class="body"
    />
</Group>

@settings {
    .heading {
        style: {
            font: {family: "Arial", weight: 700}
            font_size: 28px
            fill: rgb(32, 40, 48)
        }
    }
    .body {
        style: {
            font: "Arial"
            font_size: 18px
            fill: rgb(72, 80, 88)
        }
    }
}
```

`text={self.title}` can replace a literal when the content comes from state.
The same reactive bindings used for layout work here; see
[Properties](state-properties.md#reading-and-updating-state) and
[PAXEL](data-binding-expressions.md).

`TextStyle` groups font, size, fill, underline, and alignment settings. Use
pixel sizes such as `18px` for `font_size`: the current text patch expects a
pixel value. Use a solid color for `fill`. Although its Rust type is `Fill`,
native text currently receives only the first stop's color from a gradient.

Typography classes belong to their component's settings. For a shared type
scale across components, see [Imported settings](template-language.md#imported-settings)
and `ImportSettings`. Ancestor text styling is not a substitute for explicitly
applying the intended style to each Text node.

## Size and wrap text

A paragraph usually needs a width and an omitted height:

```pax
<Text x=24px y=24px width={100% - 48px}
    text={"A useful note can be one sentence or several. " +
        "Let the available width determine the line breaks."}
    style={font: "Arial", font_size: 18px, fill: rgb(32, 40, 48)}
/>
```

`wrap` defaults to `true`. With a width constraint, the native renderer breaks
lines and reports the height it needs. Pax uses that measurement for the
omitted dimension. Supplying both width and height gives Text a fixed box;
it does not reduce the font size to make everything fit.

Measurements can change when the string, width, style, or loaded font changes.
Allow the layout to settle before relying on a measured size. A following
element at a fixed `y` will not move just because the paragraph grew. Use an
autosized container when surrounding content should follow it; see
[Autosize](layout-responsiveness.md#autosize).

For a single-line label, `wrap=false` disables automatic line breaking.
`clip=true` cuts off content outside the Text's bounds. Clipping does not
insert an ellipsis. By default `clip` is false, so text may paint beyond its
box; an ancestor Frame can still clip it.

```pax
<Text x=24px y=24px width=220px height=28px wrap=false clip=true
    text="A long label with more to say than this row can hold"
    style={font: "Arial", font_size: 18px, fill: rgb(32, 40, 48)}
/>
```

### Align content within its box

Layout positions the Text node. TextStyle aligns the content inside it:

| Setting | Responsibility | Values |
| --- | --- | --- |
| `align_horizontal` | Places the text block horizontally inside the node | `TextAlignHorizontal::Left`, `Center`, `Right` |
| `align_vertical` | Places the text block vertically inside the node | `TextAlignVertical::Top`, `Center`, `Bottom` |
| `align_multiline` | Aligns lines within the text block | `TextAlignHorizontal::Left`, `Center`, `Right` |

For a centered caption, set all three deliberately:

```pax
<Text width=240px height=100px text="A place to begin, and room to continue."
    style={
        font: "Arial"
        font_size: 20px
        fill: rgb(32, 40, 48)
        align_horizontal: TextAlignHorizontal::Center
        align_vertical: TextAlignVertical::Center
        align_multiline: TextAlignHorizontal::Center
    }
/>
```

The default alignments are left, top, and left. Text metrics and line breaks
can differ between platforms even with the same declared size. Check your
longest content and the scripts your application needs on each shipping target.

<div class="docs-media-placeholder">
<p><strong>Interactive example planned:</strong> resize a Field notes paragraph and compare measured height, a fixed clipped box, and centered multiline text.</p>
<!-- Production brief: use the exact chapter snippets; show node bounds and
measured height without altering the paragraph. Include a delayed-font view
only after its loading behavior is independently verified. Provide source
tabs and a static narrow/wide comparison. -->
</div>

## Font values

A string names a locally available family: `font: "Arial"`. It selects normal
style and weight, with no font download URL. Availability belongs to the
workstation or device; a missing family falls back through the native renderer.

A named object adds modifiers:

```pax
<Text width=280px height=48px text="An observation worth keeping"
    style={
        font: {family: "Arial", style: FontStyle::Italic, weight: 700}
        font_size: 22px
        fill: rgb(32, 40, 48)
    }
/>
```

The fields are `family`, optional `url`, `style`, and `weight`, in any order.
Styles are `FontStyle::Normal`, `Italic`, or `Oblique`. Numeric weights are
100 through 900 in increments of 100; `FontWeight` variants such as
`Normal`, `Medium`, and `Bold` are also accepted. A weight selects a face or
renderer approximation; it does not supply a missing font file. Oblique is
accepted by the API, but the current Apple text path does not apply an oblique
transformation.

Supplying `family` without `url` keeps the URL empty. Omitting the entire font
uses Pax's current default: Roboto with a Google Fonts URL. Set a family or
source explicitly when offline behavior and repeatable typography matter.
`Font::Web(family, url, style, weight)` remains the explicit longhand; the
string and named-object forms describe the same public font type.

### Load a font file on web

For a project-owned web font, put the licensed font file in `assets/fonts/`
and reference it through `url`. For example, with a file named
`NotesRegular.ttf`:

```pax
<Text width=320px height=48px text="A typeface for these notes"
    style={
        font: {family: "Notes", url: "assets/fonts/NotesRegular.ttf", weight: 400}
        font_size: 24px
        fill: rgb(32, 40, 48)
    }
/>
```

Here `Notes` is the family name registered with the browser for that file.
Provide the actual font asset before trying the snippet. A direct remote
font-file URL uses the same web loading path, subject to the server's access
rules and the application's network policy.

Google Fonts stylesheet URLs containing `fonts.googleapis.com/css` receive
special handling. Other nonempty URLs are treated as font files, so an
arbitrary CSS stylesheet URL is not interchangeable with a font-file URL.

The native Apple loader also handles direct remote font files and the Google
Fonts CSS path, registering decoded fonts with the platform. The relative
`assets/fonts/...` recipe above is web-specific: the current native Web-font
loader does not resolve it into the application asset bundle. Choose an
available native family or verify your remote font source on macOS, iOS, and
iPadOS. Match the native font's actual family name; browser aliases alone do
not establish that name on Apple targets.

Font loading is asynchronous. A fallback may appear first and then change
the paragraph's metrics. Include first-load and unavailable-font cases in
your checks, and make sure the files you ship or host permit that use.

## Selection and editing

Text is selectable by default and non-editable by default. `selectable=false`
is useful for decorative labels that should not start a selection gesture.
Selection is target-dependent: the current iOS/iPadOS path uses an interactive
selection view for clipped text, while ordinary unclipped, non-editable text
uses its static rendering path. Test selection on the device rather than
assuming the property guarantees identical interaction everywhere.

`editable=true` makes Text a native editing surface. Edits update that Text
node's `text` property. Use an explicit two-way binding when the edited value
should belong to the containing component:

```pax
<Text x=24px y=24px width=280px height=40px
    editable=true text=bind:self.title
    style={font: "Arial", font_size: 24px, fill: rgb(32, 40, 48)}
/>
<Text x=24px y=80px width=280px text={"Current title: " + self.title}
    style={font: "Arial", font_size: 16px, fill: rgb(72, 80, 88)}
/>
```

This uses the `title: Property<String>` on Notes from
[Properties](state-properties.md#reading-and-updating-state). An ordinary
`text={self.title}` is a value binding and does not establish write-through
to the owner's property. See [Inputs and state ownership](components-composition.md#inputs-and-state-ownership)
for that distinction.

For a conventional form field, start with Textbox. Its input/change events
give the application explicit places for validation and committed actions.
See [Events](event-handling-rust.md#use-event-data) and
[Native Controls](accessibility-native-controls.md) for forms and focus behavior.

### Markdown content

Set `markdown=true` for authored content with inline emphasis and other
Markdown formatting:

```pax
<Text width=300px markdown=true
    text="Keep **one observation** and *one question*."
    style={font: "Arial", font_size: 18px, fill: rgb(32, 40, 48)}
/>
```

Web and Apple targets use different Markdown renderers, so start with simple
formatting and verify anything richer on your targets. Treat this as a
trusted-content surface: the web path inserts rendered markup and does not
provide a sanitization boundary for arbitrary user input. Leave `markdown`
false for plain strings from users.

## Image sources

Keep project-owned images in the project's `assets/` directory. Pax copies
that directory into the build output. Start with a bundled PNG or JPEG and
an explicit display area:

```pax
<Image x=24px y=24px width=160px height=100px
    source="assets/spaceship.png" fit=ImageFit::Fit
/>
```

For this snippet, copy `spaceship.png` from the canonical
`examples/src/space-game/assets/` directory into your project's `assets/`.
You can substitute your own image and keep the same layout. The path is
relative to the app's asset layout, not the directory containing its `.pax`
template.

`Image` is drawn through Pax's GPU/canvas rendering path. Its intrinsic pixel
dimensions determine its proportions, but do not automatically size the
node. Set width and height for the area the image should occupy.

Image assets load asynchronously. Once decoding finishes, Pax schedules a
redraw of the mounted image.

A string is shorthand for `ImageSource::Url`. `source={self.image_path}`
works the same way when `image_path` is a `Property<String>`; updating the
property changes the requested image. The explicit
`ImageSource::Url("assets/spaceship.png")` is also valid. An omitted source
uses `ImageSource::Empty`.

Despite the `Url` name, arbitrary remote URLs are not currently a portable
Image source. The web canvas loader prefixes the app's document directory,
and the Apple canvas loaders resolve bundled assets. Use app-relative assets
for this path. Web NativeImage has a separate browser URL path, described below.

### Fit an image into its bounds

Choose whether preserving the whole image, covering the area, or stretching
it matters most:

| `fit` | Result |
| --- | --- |
| `ImageFit::Fit` (default) | Preserves proportions and shows the whole image; may leave unused space |
| `ImageFit::Fill` | Preserves proportions and covers the bounds; crops the excess |
| `ImageFit::Stretch` | Maps the image directly to both dimensions; may distort it |

Image centers the fitted result and clips it to its own rectangular bounds.
With a square source in a wide box, Fit leaves space at the sides, Fill crops
the top and bottom, and Stretch makes the subject wider:

```pax
<Group width=400px height=80px>
    <Image width=128px height=80px
        source="assets/spaceship.png" fit=ImageFit::Fit />
    <Image x=136px width=128px height=80px
        source="assets/spaceship.png" fit=ImageFit::Fill />
    <Image x=272px width=128px height=80px
        source="assets/spaceship.png" fit=ImageFit::Stretch />
</Group>
```

Use a Frame around an Image for rounded clipping; an Image's own fit policy
only defines how pixels occupy its box. Read [Compositing](compositing-effects.md)
for clipping and masks.

<div class="docs-media-placeholder">
<p><strong>Interactive example planned:</strong> compare Fit, Fill, and Stretch using one image with clearly marked edges.</p>
<!-- Production brief: keep the same source and bounds in all three cells;
show the uncropped source alongside them. Let the reader resize the cells.
Use canonical example assets and source tabs; include a static comparison. -->
</div>

### Raw pixel data

Rust can provide an `ImageSource::Data(width, height, bytes)` for generated
pixels. The bytes are decoded RGBA values, four per pixel, and must contain
exactly `width * height * 4` entries. Compressed PNG/JPEG file bytes are not
that representation.

The same constructor can be demonstrated with two pixels in a template:

```pax
<Image width=240px height=120px fit=ImageFit::Stretch
    source=ImageSource::Data(2, 1, [55, 120, 90, 255, 246, 241, 230, 255])
/>
```

The four values in each group are red, green, blue, and alpha, from 0 to 255.
For substantial images, keep pixel generation or decoding in Rust and bind
the result, instead of embedding a large byte array in the template.

## Native images and target boundaries

`NativeImage` uses a platform image element instead of the canvas image
path. It has a string `url` property rather than an `ImageSource`:

```pax
<NativeImage x=24px y=24px width=160px height=100px
    url="assets/spaceship.png" fit=ImageFit::Fit
/>
```

On web, this creates a browser image element, so app-relative paths and
browser-supported remote URLs can be used. On macOS, iOS, and iPadOS, the
current native widget loads local filesystem paths or file URLs. Do not
assume the web-relative asset snippet resolves in the native app bundle.
PhotoPicker provides platform image handles for this use; see
[Photo selection](accessibility-native-controls.md).

The fit values are shared with Image. Web and iOS/iPadOS implement contain,
cover, and stretch behavior. The current macOS NativeImage maps Fill to
stretching; use the canvas Image path when proportional fill cropping is
required there.

Format support belongs to the active decoder. Browser image support does not
establish native target support, and loading a format through Image does not
promise animated playback of it. Verify actual assets on each target. The
canvas path uploads decoded pixels; large source images still cost memory
even when their display bounds are small.

Neither current image component exposes a public `alt` property. Keep
important information available as text, and give image-driven actions a
clear control label. A visible caption is useful content, but is not a claim
that an image has an associated screen-reader description. Native rendering
alone does not establish complete accessibility; test the interaction and
reading order with the platform's assistive tools.

## Read more

Continue with [Drawing and Styling](drawing-styling.md) for shapes, fills,
strokes, and visual surfaces. Read [Compositing](compositing-effects.md) for
mixing native and rendered content, and [Native Controls](accessibility-native-controls.md)
for input, selection, and form workflows.

The API references list the remaining details for
[Text and fonts](api/pax-std/core/text.md),
[Image](api/pax-std/media/image.md), and
[NativeImage](api/pax-std/media/native_image.md).
