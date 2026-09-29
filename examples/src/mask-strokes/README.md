# Paint layers and live component mask sources

PAX-1008 regression fixture. Run from the monorepo root:

```sh
PAX_WORKSPACE_ROOT="$PWD" pax-cli run --path examples/src/mask-strokes --target web --libdev
PAX_WORKSPACE_ROOT="$PWD" pax-cli build --path examples/src/mask-strokes --target web --libdev --release
PAX_WORKSPACE_ROOT="$PWD" pax-cli run --path examples/src/mask-strokes --target macos --libdev
PAX_WORKSPACE_ROOT="$PWD" pax-cli run --path examples/src/mask-strokes --target ipados --ios-device simulator --libdev
```

On this workstation, first run `source ~/.zshrc && nvm use` for the Node toolchain.

The first alpha mask uses `WritingSource`, a normal component whose own mount and
pre-render handlers drive a Handwriter. Moving CMY paint is the content, not the
source. The second mask uses a direct Path with a partial draw range. Both live
inside the page Scroller. A third probe places a cyan ring mask inside an offset
Frame and a nested Scroller: scroll that ring and the outer page independently.
Pause/resume, remove/remount, resize, and changing `writing_source.pax` exercise
their lifetimes. Accessible explanatory text stays outside the source tree.

## Website integration

Keep the content first and the source second, with the same aspect-ratio sizing
already used by the website's Handwriter:

```pax
<Mask width=100% height=100% alpha=true>
    <Rectangle width=100% height=100% fill=@gradient {
        0%: rgb(0, 225, 235)
        50%: rgb(240, 0, 185)
        100%: rgb(250, 230, 0)
    }/>
    <Handwriter width=100% height=100% text="creative"
        font=HandwriterFont::EMSLeague draw_end={self.progress}
        stroke={paint: WHITE, width: 5px}/>
</Mask>
```

The leading `LayerStudies` section exercises mixed typed/untyped fill lists,
reactive paint fields, linear and radial gradient strokes, and unequal outline
widths. Pause the paint, reverse the stroke stack (the white highlight should
disappear under the wide gradient), then remove/restore it. The two swatches
show 50% element opacity versus two 50% layers (75% combined alpha). The path
reveals without moving its gradient domain; nested Rust paint edits do not
replace its outer stack.

`CaptureStudies` compares a visible clipped, half-opacity group with the same
component used as an alpha source. Their overlapping shapes stay equally bright.
A second swatch combines a nested source mask and feathering. Source-side
Frame/Mask vector clips are supported. The Scrollers contain masked content;
source-side Scrollers and native leaves remain outside capture support.

## Validation and coordinate regression

On September 27, 2026, root-level WGPU web debug and baked release runs both
rendered the component source and direct Path correctly at desktop and 390px
width. Debug pause/resume, removal/remount, and source-template hot reload were
verified. No native Handwriter text duplicate appeared in the accessibility
tree. No Apple-native target or Piet alpha-mask support is claimed.

On base `0b5c68be1`, wrapping the fixture in a Scroller made both masks blank,
even with the old source initialization. Paint was incorrectly positioned
relative to the intermediate Mask frame, while coverage was positioned relative
to the owning Scroller. The coordinate fix uses the registered surface owner
for both. A regression test covers intermediate frames, nested transformed
owners, and root surfaces; the fixture now exercises the Scroller case directly.
This is library-fixture validation, not acceptance of the website integration.

After the coordinate fix, the Scroller fixture rendered in both WGPU web debug
and baked release builds at desktop and narrow widths (390px debug, 354px
release). Pause/resume and removal/remount passed in both builds. Scrolling the
nested ring and then the outer page independently preserved mask alignment in
release. The runtime and standard-library suites passed all 234 tests, including
the new transform regression; API generation and the book build also passed.


## Paint-layer validation (September 28, 2026)

After rebasing onto main, the extended study rendered in WGPU web debug and
baked release at desktop and 390px widths. Stack reversal, removal/restoration,
nested paint animation, opacity comparisons, template hot reload, and the
existing component/Scroller masks were exercised. A temporary vector-only
mount of `LayerStudies` with `pax-kit/piet` also rendered linear/radial strokes,
a solid/gradient paint mixture, and the two opacity cases in Canvas2D. Do not
mount the alpha-mask portions when checking Piet: that backend explicitly
rejects alpha masks.

The integrated core suites passed 640 tests, Metal pixel/resource suites passed
16, and clock-driven rendering passed 5. API/source-bundle tests and the book
build passed; no full macOS/iOS application execution is claimed. At that
checkpoint alpha-source reveals still rebuilt extracted outlines. The retained
capture integration below replaces that path. The website's migrated existing
surfaces compile and render; no new
website mask composition was introduced.


## Retained alpha-source integration

WGPU alpha sources now record ordinary vector draws in a detached retained
scene and reuse the capture infrastructure from group opacity. Primitive and
group opacity, gradients, materials, internal Frame/Mask clips, and stroke draw
ranges share the visible renderer. Source RGB is discarded after composition.
Geometry masks retain their existing coverage semantics; Piet alpha masks,
native sources, images, and source-side Scrollers remain unsupported.

The regression suites compare source alpha with visible transformed gradient
strokes, distinguish 50% group opacity from 75% overlapping per-shape opacity,
check nested sources and clips, and read feather contributions beyond a tile.
Resource counters verify zero new tessellation for warmed source reveals;
removal checks verify detached draws and captures retire.

## Apple debug smoke checks (September 29, 2026)

The fixture builds and launches on native macOS and the iPad Pro 13-inch (M5)
simulator running iOS 26.4. Both show layered linear/radial strokes, the element
and layer opacity comparison, matching visible/captured group alpha, and nested
feathered masks. Native paint pause/resume and stroke-stack reversal pass on
both targets. macOS also passes live handwriting, source removal/remounting,
outer scrolling, and nested ring scrolling with aligned mask content.

The controls now use `@button_click` and `Event<ButtonClick>`. The earlier
`@click` bindings happened to work with browser mouse input but did not receive
Apple native Button activation. See [Events & Rust](../../../pax-docs/book/src/event-handling-rust.md#buttons-and-custom-activation).

Simulator automation could activate Buttons but could not move the page with
its swipe/scroll attempts. iPad handwriting, source removal/remounting, and
nested scrolling therefore still need a manual check; this does not establish
a Scroller defect. These are debug app checks, not Apple release or physical
device validation.
