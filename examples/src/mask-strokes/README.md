# Live component mask sources

PAX-1008 regression fixture. Run from the monorepo root:

```sh
PAX_WORKSPACE_ROOT="$PWD" pax-cli run --path examples/src/mask-strokes --target web --libdev
PAX_WORKSPACE_ROOT="$PWD" pax-cli build --path examples/src/mask-strokes --target web --libdev --release
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
        stroke={color: WHITE, width: 5px}/>
</Mask>
```

Direct gradient stroke paint is described in the separate
[Paint and layer specification](../../../pax-docs/book/src/design/PAX-1008-paint-fill-stroke-layers.md)
and is not implemented by this fixture yet.
Source-side Frame/Mask clips and native content are not supported by the current
paint collector. The Scrollers in this fixture contain the masked content;
they are not part of the offscreen source trees.

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
