# Paint layers and live component mask sources

PAX-1008 regression fixture. Run from the monorepo root:

```sh
PAX_WORKSPACE_ROOT="$PWD" pax-cli run --path examples/fixtures/mask-strokes --target web --libdev
PAX_WORKSPACE_ROOT="$PWD" pax-cli build --path examples/fixtures/mask-strokes --target web --libdev --release
PAX_WORKSPACE_ROOT="$PWD" pax-cli run --path examples/fixtures/mask-strokes --target macos --libdev
PAX_WORKSPACE_ROOT="$PWD" pax-cli run --path examples/fixtures/mask-strokes --target ipados --ios-device simulator --libdev
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
    <Handwriter width=100% height=100%
        font=HandwriterFont::EMSLeague draw_end={self.progress}
        stroke={paint: WHITE, width: 5px}>creative</Handwriter>
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

## Retained alpha-source integration

WGPU alpha sources now record ordinary vector draws in a detached retained
scene and reuse the capture infrastructure from group opacity. Primitive and
group opacity, gradients, materials, internal Frame/Mask clips, and stroke draw
ranges share the visible renderer. Source RGB is discarded after composition.
Geometry masks retain their existing coverage semantics. Vello CPU now supports
alpha masks and feathering through replayed vector captures. Native sources,
images, and source-side Scrollers remain unsupported.

The regression suites compare source alpha with visible transformed gradient
strokes, distinguish 50% group opacity from 75% overlapping per-shape opacity,
check nested sources and clips, and read feather contributions beyond a tile.
Resource counters verify zero new tessellation for warmed source reveals;
removal checks verify detached draws and captures retire.

## Control activation

The controls use `@button_click` and `Event<ButtonClick>` for native Button
activation. See [Events & Rust](../../../pax-docs/book/src/event-handling-rust.md#buttons-and-custom-activation).
