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
    <Text width=100% height=40px class="heading">Field notes</Text>
    <Text y=56px width=100%
        class="body">{"Collect a small observation. Give it a title, " +
            "then leave room for the next idea."}</Text>
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

`<Text>{self.title}</Text>` reads content from state. Body syntax assigns the
existing `text` property; `text={self.title}` remains valid. See
[text bodies](template-language.md#text-bodies) for multiline indentation and
raw strings that preserve exact text.
The same reactive bindings used for layout work here; see
[Properties](state-properties.md#reading-and-updating-state) and
[PAXEL](data-binding-expressions.md).

`TextStyle` groups font, size, fill, underline, and alignment settings. Use
pixel sizes such as `18px` for `font_size`: the current text patch expects a
pixel value. Use a solid color for `fill`. Although its Rust type is `Paint`,
native text currently receives only the first stop's color from a gradient.
It does not accept vector Fill/Stroke stacks or per-layer materials; gradient
paint mixtures crossfade those representative colors.

Typography classes belong to their component's settings. For a shared type
scale across components, see [Imported settings](template-language.md#imported-settings)
and `ImportSettings`. Ancestor text styling is not a substitute for explicitly
applying the intended style to each Text node.

## Size and wrap text

A paragraph usually needs a width and an omitted height:

```pax
<Text x=24px y=24px width={100% - 48px}
    style={font: "Arial", font_size: 18px, fill: rgb(32, 40, 48)}>{"A useful note can be one sentence or several. " +
        "Let the available width determine the line breaks."}</Text>
```

`wrap` defaults to `true`. With a width constraint, the native renderer breaks
lines and reports the height it needs. Pax uses that measurement for the
omitted dimension. Supplying both width and height gives Text a fixed box;
it does not reduce the font size to make everything fit.

Wrapping uses the Text's local layout width. Rotating, translating, or scaling
the node or an ancestor transforms that layout without changing its line breaks.
Changing the layout width can reflow the text. Plain Text preserves authored
spaces and newlines even when wrapping is enabled, including intentional
indentation in a raw string or extra indentation in a framed body. Markdown
retains the target renderer's own whitespace rules.

On web and Apple targets, ordinary affine Text inside a Scroller retains its
placement relative to that scroller. Moving the whole host does not republish
each unchanged text transform or prepare its content and measurement again.
Local text movement, width and content edits still update normally; spatial,
camera-controlled, detached, and flight placement uses the conservative
world-coordinate path. This is automatic and requires no template setting.

Measurements can change when the string, width, style, or loaded font changes.
Allow the layout to settle before relying on a measured size. A following
element at a fixed `y` will not move just because the paragraph grew. Use an
autosized container when surrounding content should follow it; see
[Autosize](layout-responsiveness.md#autosize).

For a single-line label, `wrap=false` disables automatic line breaking.
`clip=true` cuts off content outside the Text's bounds. Clipping does not
insert an ellipsis. By default `clip` is false, so text may paint beyond its
box; an ancestor Frame can still clip it.
An unclipped single-line label keeps its horizontal alignment when it overflows:
centered text extends on both sides of the box, and right-aligned text extends
to the left.

```pax
<Text x=24px y=24px width=220px height=28px wrap=false clip=true
    style={font: "Arial", font_size: 18px, fill: rgb(32, 40, 48)}>A long label with more to say than this row can hold</Text>
```

### First presentation on web

New web `Text` nodes stay hidden while their requested font loads and their
initial native measurement feeds back through layout. They remain mounted and
measurable during this interval. Fixed-size text waits for its font but needs
no autosize response. Once shown, text stays visible during later edits and
layout changes; this is not a barrier for the whole scene or its canvases.
Failed font loads use the browser fallback. A slow font can delay the first
appearance of text that uses it.

Web builds discover literal web-font sources in templates/settings and start
loading supported Google Fonts stylesheets alongside the cartridge. Each
stylesheet is shared across weights and text nodes. This applies to debug and
release builds. Fonts constructed by Rust or computed dynamically still load
when first requested; the static catalog does not predict those values.

### Align content within its box

Layout positions the Text node. TextStyle aligns the content inside it:

| Setting | Responsibility | Values |
| --- | --- | --- |
| `align_horizontal` | Places the text block horizontally inside the node | `TextAlignHorizontal::Left`, `Center`, `Right` |
| `align_vertical` | Places the text block vertically inside the node | `TextAlignVertical::Top`, `Center`, `Bottom` |
| `align_multiline` | Aligns lines within the text block | `TextAlignHorizontal::Left`, `Center`, `Right` |

For a centered caption, set all three deliberately:

```pax
<Text width=240px height=100px
    style={
        font: "Arial"
        font_size: 20px
        fill: rgb(32, 40, 48)
        align_horizontal: TextAlignHorizontal::Center
        align_vertical: TextAlignVertical::Center
        align_multiline: TextAlignHorizontal::Center
    }>A place to begin, and room to continue.</Text>
```

The default alignments are left, top, and left. Text metrics and line breaks
can differ between platforms even with the same declared size. Check your
longest content and the scripts your application needs on each shipping target.

## Font values

A string names a locally available family: `font: "Arial"`. It selects normal
style and weight, with no font download URL. Availability belongs to the
workstation or device; a missing family falls back through the native renderer.

A named object adds modifiers:

```pax
<Text width=280px height=48px
    style={
        font: {family: "Arial", style: FontStyle::Italic, weight: 700}
        font_size: 22px
        fill: rgb(32, 40, 48)
    }>An observation worth keeping</Text>
```

The fields are `family`, optional `url`, `style`, and `weight`, in any order.
Styles are `FontStyle::Normal`, `Italic`, or `Oblique`. Numeric weights are
100 through 900 in increments of 100; `FontWeight` variants such as
`Normal`, `Medium`, and `Bold` are also accepted. A weight selects a face or
renderer approximation; it does not supply a missing font file. Oblique is
accepted by the API, but the current Apple text path does not apply an oblique
transformation.

Supplying `family` without `url` keeps the URL empty. On Apple native targets,
this resolves the installed family and its requested style and weight without
starting a font download, in both debug and release builds. Omitting the entire font
uses Pax's current default: Roboto. Web and Apple obtain that default through
Google Fonts; Windows, Linux, and Android use the standard library's packaged Roboto
normal and italic faces, including their weight variants. Set a family or
source explicitly when you need a particular font version across every target.
`Font::Web(family, url, style, weight)` remains the explicit longhand; the
string and named-object forms describe the same public font type.

The standard library's `ExampleHost` source drawer packages its Space Grotesk
and JetBrains Mono faces through the ordinary shared asset inventory. These
fonts no longer require a Google Fonts request. Asset discovery currently works
per package, so applications using other `pax-std` components also carry the
approximately 327 KiB of font files and licenses; browsers request a face only
when it is used. The files are separate from Web's Wasm binary.

For reactive font changes, use an expression that returns a complete
`Font::Web(...)` value. Reactive fields inside the named-object shorthand
currently fail during runtime property resolution. The
[`native-text-style` fixture](https://github.com/paxdotdev/pax/blob/main/examples/fixtures/native-text-style/README.md)
demonstrates switching weight and slant with the explicit form.

On iOS/iPadOS and macOS, Pax currently creates native fonts at the authored
size. It does not yet apply Dynamic Type or an equivalent accessibility
text-size policy. Browser page zoom uses the browser's viewport and pixel
scale; it is distinct from changing the native font's metrics. Test enlarged
text and zoom on each shipping target rather than assuming native font
rendering provides automatic accessibility scaling.

### Fonts on Android

The experimental Android host accepts installed families with an empty URL and
bundled `.ttf` or `.otf` files using the same asset paths as images. Pax includes
licensed Roboto normal and italic files for its exact default family/source
pair in both Debug and baked Release packages. HTTPS TrueType/OpenType sources
and simple `@font-face` stylesheets load asynchronously. Stylesheets select the
closest weight/style face; WOFF and combining CSS Unicode-range subsets are
unsupported. Bundle a complete font for predictable offline and multilingual
coverage. Font weight and italic style use Android's typeface API; text
measurement waits for the requested font or its fallback after a load failure.

For system families, Pax first tries the exact name, then a lowercase Android
alias if that lookup returns the default font. This lets names such as `Georgia`
and `Arial` use the device's aliases while preserving installed families with
mixed-case names. An alias does not install the named font: on the Android
emulator, `Georgia` selects Noto Serif and `Arial` selects Roboto. Available
families and substitutions can vary by device. Bundle a licensed font file
with `url` when the design requires the actual face across platforms.

```pax
<Text width=280px height=48px text="A native Android note"
    style={font: {family: "Manrope", url: "assets/fonts/Manrope.ttf"}, font_size: 22px}
/>
```

Put the licensed font at `assets/fonts/Manrope.ttf` and rebuild the APK after
changing assets. See [Android packaging](targets-build-deploy.md#experimental-android).

HTTPS images also load off the UI thread. Downloads have a 16 MiB limit, a
10-second connection/read timeout, and at most five HTTPS redirects. Decoded
bitmaps have an 8192-pixel axis and 16-megapixel limit. A session deduplicates
requests and uploads at most one decoded image per frame. The on-disk cache is
bounded to 64 MiB; use versioned URLs when replacing remote content. Failed
requests are logged and retried in a new session. Cleartext HTTP assets are
unsupported. Include `android.permission.INTERNET` in Android permissions
(the default) when using remote assets.

The disk-cache limit is not a total image-memory budget. Uploaded source images
remain in the runtime's image map for the session lifetime, so loading an
unbounded sequence of distinct image URLs can continue growing process memory.

<a id="fonts-on-experimental-desktop-hosts"></a>

### Fonts on Windows and Linux

The Windows and Linux hosts accept an empty font URL for a system family, or an
app-relative URL for a bundled font file. Select the file's actual family:

```pax
<Text width=280px height=48px text="A native desktop note"
    style={font: {family: "Manrope", url: "assets/fonts/Manrope.ttf"}, font_size: 22px}
/>
```

Supply that licensed font file in the application's assets. Both desktop hosts
also accept `/assets/fonts/Manrope.ttf` as a bundled-asset alias; it does not
request a file from the operating system's root directory. The default Roboto
family works offline without an explicit font declaration, including bold and
italic styles on text and controls. Its two licensed font files add about 1 MiB
to the Windows/Linux asset inventory; web and Apple application bundles do not include
them. Other remote font files and stylesheet URLs
remain unsupported; the built-in default is resolved to its packaged files,
not downloaded at runtime. A custom source URL is never silently replaced.

Windows direct Cargo builds embed a bounded font inventory and materialize a
verified cache beside the executable before initializing WinUI. That path needs
a writable executable directory with hard-link support. Put font redistribution
notices beside their font files as `OFL.txt`, `OFL-<family>.txt`, `LICENSE`,
`LICENSE.txt`, `COPYING`, or `COPYING.txt`; those notices travel with the fonts.
The limits are 256 resources including notices, 16 MiB per file, and 64 MiB total.
Existing files with mismatched content
produce an error. CLI packages already contain the font resources and perform
no startup extraction. Linux registers bundled fonts through Fontconfig.
Loading another face of an existing family refreshes the selected font and
requests remeasurement of automatic Text dimensions. Reusing an already loaded
file does not continually invalidate those measurements.

Linux follows GTK's effective font resolution. At its 96-DPI font baseline,
an authored `18px` font remains 18 logical pixels; a font resolution of 144 DPI
enlarges it to 27 logical pixels. GTK handles display scaling separately.
When GTK reports changed font metrics, the host remeasures Text's omitted
dimensions in the running app. Autosized parents and following content then
use the new measurements, including when text shrinks again. Content and width
changes also preserve automatic dimensions across updates.

This path has passed Debug and baked Release checks on GTK 4.14.5, plus the
optional GTK 4.22.5 Flatpak runtime, using live X11 font-setting changes.
Fresh launches at 144-DPI font resolution passed on all three configurations.
Whether a desktop's large-text control supplies that setting depends on its
GTK integration. The installed GTK settings portal also passed live
100% → 150% → 100% changes through the optional Flatpak on headless Wayland,
including intrinsic geometry and restoration in the same process. That isolated
GSettings/portal check does not qualify a GNOME or KDE desktop's settings
interface, physical monitor changes, or other portal providers.

WinUI honors the Windows accessibility text-size setting independently of
screen DPI. The Windows host remeasures Text's omitted dimensions after a live
system text-size change, even when its content and width have not changed.
Autosized containers can then grow and move following content using the new
measurement. Repeated Text elements keep independent measurements.
Native metric notifications also invalidate cached text measurements; retaining
text placement while its Scroller moves does not suppress live text-size updates.

Increasing text size does not automatically enlarge authored Pax widths and
heights. Leave a paragraph's height unspecified and use an autosized parent
when following content should move. An explicitly fixed 30px-high label keeps
that height even when enlarged glyphs need more room; they may overflow or
be clipped according to the target and clipping settings. Check both fresh
launches and live setting changes at the text sizes the application needs to
support.

See [desktop setup and packaging](targets-build-deploy.md#experimental-windows-and-linux-desktop).

<a id="text-styling-on-experimental-desktop-hosts"></a>

### Text styling on Windows and Linux

Windows uses WinUI text and controls; Linux uses GTK/Pango. Both accept uniform
plain-text font family, size, weight, slant and solid foreground color, including
alpha. They preserve native editing and selection while styles change. A font
still needs to be installed or bundled as described above.

A fully transparent plain `Text` fill keeps its content, measurement and native
selection behavior. On Linux, `fill: rgba(0, 0, 0, 0)` makes unselected glyphs
invisible while selected text still has GTK's native selection highlight.
Changing the fill back to a visible color preserves the same native text control.

On Windows, `Text` with `font_size: 0px` has no visible glyphs and measures zero
on omitted dimensions. Explicit width and height still define its layout box.
Changing to a positive size restores the text and remeasures omitted dimensions;
its content and native identity remain intact throughout. This zero-size behavior
is specific to `Text`, not a promise for every native form control.

| Capability | Windows | Linux |
| --- | --- | --- |
| Plain `Text` underline | Supported, including removing underline | Supported, including removing underline |
| Plain `Text` block and line alignment | Separate horizontal/vertical block alignment and multiline line alignment | Separate horizontal/vertical block alignment and multiline line alignment |
| Textbox font, color and horizontal line alignment | Native single-line and multiline editors | Native single-line and multiline editors |
| Textbox underline | Not implemented | Supported, including new text inserted into an empty multiline editor |
| Textbox vertical alignment | Native control content alignment; exact behavior remains control-specific | Single-line Entry keeps GTK's native vertical alignment; multiline vertical block alignment is not implemented |
| Button, Dropdown and RadioList text | Font, foreground and underline through native labels | Font, foreground and underline through native labels |
| Markdown / per-range rich text | Native styled runs for the [supported Markdown subset](#markdown-content) | Native styled runs for the [supported Markdown subset](#markdown-content) |

Fixed Text bounds and glyph bounds are different. Alignment places the intrinsic
text inside the authored box; it does not stretch glyphs. With `clip=false`,
overflow can extend outside the box. Set `clip=true` for an explicit cutout and
leave dimensions automatic when the layout should grow with the content.

Windows does not implement `Text editable=true`; use `Textbox` for native text
entry. Linux can switch Text to a native multiline editor and make it read-only
again, but that retained editor does not implement vertical block alignment or
`selectable=false`. Use plain Text for display-only content and Textbox for form
input when these limits matter. Rich text is display-only: neither desktop host
accepts `markdown=true` together with `editable=true`.

The [`native-text-style` fixture](https://github.com/paxdotdev/pax/blob/main/examples/fixtures/native-text-style/README.md)
compares fixed-box alignment, reactive styling and editing without remounting.
Check each control type used by an application. A styled closed Dropdown does
not establish styling or clipping parity for its separate native popup surface.
On Linux, the closed Dropdown's arrow follows the authored foreground color,
so it remains visible with a contrasting custom background. Check its separate
popup surface and disabled state with the theme used by the application.

### Load a font file on web

For a project-owned web font, put the licensed font file in `assets/fonts/`
and reference it through `url`. For example, with a file named
`NotesRegular.ttf`:

```pax
<Text width=320px height=48px
    style={
        font: {family: "Notes", url: "assets/fonts/NotesRegular.ttf", weight: 400}
        font_size: 24px
        fill: rgb(32, 40, 48)
    }>A typeface for these notes</Text>
```

Here `Notes` is the family name registered with the browser for that file.
Provide the actual font asset before trying the snippet. A direct remote
font-file URL uses the same web loading path, subject to the server's access
rules and the application's network policy.

On web, URLs whose path ends in `.css` and Google Fonts `/css` endpoints are
loaded as font stylesheets. Their relative font-asset URLs resolve against the
stylesheet's URL. Other nonempty URLs are treated as font files. Declare the
stylesheet in the font's `url`; the generated HTML does not load a global font
stylesheet for every application. For example:

```pax
<Text width=280px
    style={font: {family: "Example Sans", url: "/assets/fonts/example.css"}, font_size: 24px}>A stylesheet font</Text>
```

This requires an actual stylesheet declaring the matching family and its font
assets. Generic `.css` source handling is web-only; check the Apple guidance below
when sharing a template across targets.

On web, statically discovered font stylesheets are registered early, without
preloading every weight or language subset they declare. Text requests its face
using the actual content, so the browser can honor `unicode-range`. The initial
Text presentation waits for those fonts and updated autosize measurements. A
link font is requested only when the Text contains a link; runtime-created fonts
and changes in text content use the same loading path. Failed loads permit the
browser's fallback font. Later changes do not hide an already-presented Text.

Screenshot capture embeds registered font assets on demand so the serialized
native overlay can render them. This work is separate from ordinary page startup;
the first capture can take longer than subsequent captures.

### Bundle fonts for Apple targets

macOS, iOS, and iPadOS package the project's `assets/` directory and register
its `.ttf` and `.otf` fonts at app startup. Put a licensed font in
`assets/fonts/`, then select its actual family and desired weight, for example
`font: {family: "Manrope", weight: 800}` with Manrope installed in that bundle.
No download URL is needed on Apple. This also works in release cartridges.
`public/` is a web-only directory and does not participate in Apple packaging.

Web still needs the asset URL. A shared declaration can use both the actual
family and that URL: Apple resolves the already registered family, while web
loads the asset. Browser-only family aliases do not establish names on Apple.
Weights select available named faces or named instances of a variable font;
verify the font supplies the requested weight. This does not expose arbitrary
variation-axis controls.

The native Apple loader also handles direct remote font files and the Google
Fonts CSS path. It does not resolve relative URLs into bundle resources: a
missing or incorrectly named bundled family will not be rescued by its URL.
Verify your remote font source on every shipping target.

Remote and web font loading is asynchronous. A fallback may appear first and then change
the paragraph's metrics. Include first-load and unavailable-font cases in
your checks, and make sure the files you ship or host permit that use.

## Selection and editing

Text is selectable by default and non-editable by default. `selectable=false`
is useful for decorative labels that should not start a selection gesture.
On macOS, ordinary selectable text uses a native label; editing and selectable
Markdown use the native document text view. Pax chooses the representation
automatically, and selection works with either `clip=true` or `clip=false`.
Read-only text forwards wheel scrolling to its containing Scroller, including
when the text is selected; `clip=true` clips the label without giving it an
independent scroll position. Editable text can scroll within its editing area.
An empty macOS editor retains its declared editing height, so the insertion
point and input-method composition have room before any text is committed.
Give an editor enough height for its font; `clip=true` still clips to its bounds.

Selection is target-dependent: the current iOS/iPadOS path uses an interactive
selection view for clipped text, while ordinary unclipped, non-editable text
uses its static rendering path. Test selection on the device rather than
assuming the property guarantees identical interaction everywhere.
On macOS, both clipped and unclipped selectable Text use the native text view.
Android uses native TextView selection for plain and formatted text, and its
native editable buffer for `editable=true`. Editing Markdown exposes its
authored source so typing does not erase the formatting markers.
Application updates to Android `Textbox` and editable `Text` preserve the
existing native editing buffer and clamp its selection to the updated text,
so a keyboard can continue typing through the same input connection.

`editable=true` makes Text a native editing surface. Edits update that Text
node's `text` property. Use an explicit two-way binding when the edited value
should belong to the containing component:

```pax
<Text x=24px y=24px width=280px height=40px
    editable=true text=bind:self.title
    style={font: "Arial", font_size: 24px, fill: rgb(32, 40, 48)}
/>
<Text x=24px y=80px width=280px
    style={font: "Arial", font_size: 16px, fill: rgb(72, 80, 88)}>{"Current title: " + self.title}</Text>
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

Text defaults to plain text. Set `markdown=true` for authored content with
inline emphasis and other Markdown formatting:

```pax
<Text width=300px markdown=true
    style={font: "Arial", font_size: 18px, fill: rgb(32, 40, 48)}>Keep **one observation** and *one question*.</Text>
```

Web uses snarkdown; Apple uses its native inline Markdown path with whitespace
preservation and a separate HTML compatibility path. Android uses its native Markdown renderer. Full block Markdown
parity is not guaranteed. Start with simple formatting and verify richer content
on each target. Body whitespace normalization happens before rendering; use a
raw string expression for exact indentation or boundary newlines. Treat this as a
trusted-content surface: the web path inserts rendered markup and does not
provide a sanitization boundary for arbitrary user input. Leave `markdown`
false for plain strings from users.

On the web, `_style_link` controls the color and typography of Markdown
links separately from the paragraph's `style`. Binding it to a theme updates
already-mounted anchors in place; you don't need to change the Markdown
string or replace the Text node.

Windows and Linux share a Markdown parser and render its results using native
text controls. Supported formatting includes emphasis, strong emphasis,
strikethrough, inline code, paragraphs, headings, simple ordered/unordered lists, fenced
code blocks, and links. Native selection and automatic measurement use the
rendered text; style changes preserve inline formatting. These hosts also accept
`ExampleHost`'s preformatted, syntax-colored source markup, including escaped
characters, Unicode, blank lines, and fixed line spacing.

This is a defined subset, not a browser HTML renderer. Tables, inline images,
blockquotes, task lists, footnotes, horizontal rules, and link titles are not
implemented. Supported HTML elements are `span`, `pre`, `strong`/`b`, `em`/`i`,
`u`, `s`/`del`, `code`, `a`, and `br`; attributes and CSS are limited to link
`href`, opaque hexadecimal foreground colors, and the preformatted source
layout used by `ExampleHost`. HTML anchors must stay within one Markdown
paragraph, and a `pre` block must occupy the whole document. Headings, code or
HTML blocks, and multiple paragraphs inside one list item need richer block
layout and are not supported. Unsupported constructs fail the desktop session; correct the content
and restart the application. Keep richer content on targets that implement it,
or use ordinary Pax elements to lay it out.

Desktop links accept application-relative destinations and `http`, `https`, or
`mailto` URLs. Relative links use the native application's virtual router;
absolute URLs go to the operating system. Other schemes are rejected. Editing
rich text is not implemented; use `Textbox` or plain editable `Text` where the
target supports it. Web and Apple retain their existing parsers, so richer
formatting still needs target-specific checks.

Android supports CommonMark emphasis, headings, paragraphs, lists, block quotes,
links, and inline/fenced code, plus strikethrough. It measures the same native
styled text that it displays. `wrap=false` preserves authored newlines while
allowing long lines to overflow horizontally. Markdown links use the host's
[navigation behavior](routing.md), including internal routes and Android Back.
ExampleHost's source drawers preserve indentation, escaped characters,
monospace text, and token colors.

Markdown images display their alternative text on Android; use `Image` for
images. Arbitrary HTML is displayed literally, apart from Pax's generated
ExampleHost code markup. Tables, footnotes, and math extensions are not enabled.
A rich-text node accepts up to 1 MiB of source, 1 MiB of expanded link targets,
65,536 formatting spans, and 256 levels of nesting; exceeding these limits
reports a host error. Repeated reference links each count toward the link-target
limit. Verify selection,
link interaction, and language/font coverage on the devices you intend to ship.

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
`examples/fixtures/space-game/assets/` directory into your project's `assets/`.
You can substitute your own image and keep the same layout. The path is
relative to the app's asset layout, not the directory containing its `.pax`
template.

On Windows and Linux, `/assets/spaceship.png` is also accepted as a
bundled-asset alias, with the same resolution as `assets/spaceship.png`.

`Image` is drawn through Pax's GPU/canvas rendering path. Its intrinsic pixel
dimensions determine its proportions, but do not automatically size the
node. Set width and height for the area the image should occupy.

Image assets normally load asynchronously. The Linux host currently
decodes a local image once on its GTK thread; large files can stall that thread.
Once decoding finishes, Pax schedules a redraw of the mounted image. On web, concurrent `Image` requests with the same
source string in one app share one fetch/decode operation and one transfer of
the decoded pixels into the renderer. Every still-mounted waiting image is
scheduled to redraw. Requesting a different URL detaches that node from its
previous pending request; disposing the app aborts outstanding fetches. This does not extend to
`NativeImage`, unrelated app instances, or the Apple loaders.

Decoded pixels and GPU textures have separate lifetimes. Scrolling can require
preparing a new tile's draw bindings or uploading pixels that are no longer
resident on the GPU, without another network request or decode. Tile preparation
does not defer component mounting. See
[Scrolling & Viewports](scrolling-viewports.md) for resource sharing and diagnostics.

Windows decodes up to eight local images concurrently. Larger bursts queue
their paths until a decoder is available, so a gallery can request more than
eight images in one frame. Repeated requests for a path share its pending work.
Each decoded image remains limited to 32 MiB of RGBA pixels; this is a
per-image limit, not a total gallery memory budget. Missing files and decode
errors still fail explicitly.

A string is shorthand for `ImageSource::Url`. `source={self.image_path}`
works the same way when `image_path` is a `Property<String>`; updating the
property changes the requested image. The explicit
`ImageSource::Url("assets/spaceship.png")` is also valid. An omitted source
uses `ImageSource::Empty`.

Despite the `Url` name, arbitrary remote URLs are not currently a portable
Image source. The web canvas loader prefixes the app's document directory,
and the Apple canvas loaders resolve bundled assets. Use app-relative assets
for this path. Web NativeImage has a separate browser URL path, described below.
The alpha Windows and Linux hosts also resolve local assets; remote
image URLs currently report an unsupported error.

For portable Apple bundles, use distinct resource basenames even across
subdirectories. For example, name a large still `stills/film.jpg` and its small
derivative `thumbs/film-thumb.jpg`. Apple's Swift resource processing rejects
two processed resources named `film.jpg`. Reuse each path for repeated image
nodes, and use the smaller derivative when full-resolution pixels are unnecessary.
The `paxflix` example demonstrates this catalog pattern.

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

Animate `opacity` on the Image or a shared ancestor to fade artwork with the
rest of a component. Source pixel alpha remains part of the composed content;
changing opacity does not reload its source pixels. See
[subtree opacity](compositing-effects.md#opacity-through-a-subtree) for how
overlapping translucent elements compose.

On WGPU, the image's crop follows fractional transformed coordinates. Moving a
Fill image by a fraction of a pixel keeps excess artwork clipped to its bounds,
including where an overlaid gradient meets the edge of the image.

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
Supply straight, encoded RGB values: do not linearize them or multiply RGB by
alpha yourself. On the ordinary 8-bit web and native outputs, an opaque pixel
`[30, 120, 220, 255]` matches a vector filled with `rgb(30, 120, 220)`.
Changing its alpha to 128 makes that same color roughly half opaque; the renderer
applies alpha during composition. Image sampling follows the output's color
encoding, avoiding an extra sRGB decode on these outputs.

GPU adapters that support alternate texture views share one image upload across
encoded and linear sampling. Other adapters cache a separate upload for each
required sampling encoding; the image colors remain the same.

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

On web, the image's DOM element does not intercept pointer input. Pax still
routes image events and enclosing `Link` clicks through scene hit-testing;
wheel input reaches the scroller underneath. This prevents a visually occluded
image from stealing scrolling from a foreground scroller. Browser-native image
dragging and image-specific context menus are not provided by this path.

On Android, `NativeImage` creates an `ImageView`. It loads bundled `assets/`
paths and HTTPS URLs asynchronously using the same path rules and decode limits
as canvas images. Mounted native images with the same URL share a decoded bitmap;
changing the URL or unmounting releases that view's reference. These images remain
inside the native scrolling and clipping hierarchy. Animated playback is not
supported by this bitmap-loading path.

On Windows and Linux, `NativeImage` creates a WinUI `Image` or GTK `Picture`
inside the native scrolling, clipping, and masking hierarchy. Use packaged
asset paths such as `assets/spaceship.png`; `/assets/spaceship.png` is also
accepted as an asset-root alias. Windows requires a path inside a configured
asset root. Linux also accepts local filesystem paths and file URLs. These
desktop loaders do not fetch HTTP or HTTPS URLs.

As on web, Windows and Linux native image widgets do not intercept pointer
input. Pax routes image events through scene hit-testing, while native scrollers
retain wheel handling.

Desktop native images load asynchronously. Mounted images sharing a source
reuse the native image resource; changing `url` or removing an image releases
that view's reference. An empty URL clears the image. Source replacement does
not let an earlier load overwrite the new image. Missing or unsupported assets
report a host error rather than silently substituting artwork.

Windows has an unresolved intermittent `NativeImage` failure during startup or
early resize: WinUI reports that an interface was marshalled for a different
thread, and the host exits with an error. Recent acceptance runs did not
reproduce it, but that does not establish a fix. Include repeated launches and
early resizing when qualifying image-heavy Windows applications.

PNG, JPEG, and SVG use the platform decoders. Linux SVG support requires the
GdkPixbuf SVG loader, included in the tested GNOME runtime. Linux decodes SVG
once at its intrinsic or decoder-default raster size; later scaling resizes
those pixels rather than rerasterizing the vector artwork. Windows uses
`SvgImageSource`. Format features can differ between these decoders, especially
for SVG; test the actual artwork on each target. The desktop paths do not
provide animated-image playback. Encoded files and nominal decoded raster pixels
are limited to 32 MiB per asset. These checks do not bound total decoder or GPU
memory. Windows fixes each SVG's shared raster size after its initial WinUI
layout; later enlargement or display-scale changes reuse those pixels and can
soften the image. Its initial SVG decode and toolkit backing storage are separate
from the decoded-bitmap accounting. Linux also limits retained decoded pixels
for mounted images to 256 MiB and each decoded axis to 16,384 pixels; exceeding
a limit reports an error.

The fit values are shared with Image. Web, Android, Windows, Linux, and
iOS/iPadOS implement contain, cover, and stretch behavior. The current macOS NativeImage maps Fill to
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
