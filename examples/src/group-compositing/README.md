# Group compositing

Focused reproduction and acceptance fixture for [PAX-1004](https://linear.app/paxdev/issue/PAX-1004/add-isolated-group-compositing-for-mixed-nativecanvas-content).

From the monorepo root:

```sh
cargo run -p pax-cli -- run --path examples/src/group-compositing --target web --libdev
```

The initial alpha is 50%. **+25%** cycles through 75%, 100%, 0%, 25%, 50%.
The left probe paints overlapping opaque rectangles beneath a Group; the right
paints the same silhouette as one Path. With subtree opacity, the shapes match
at intermediate values. At the old baseline (`090348859`) their overlap differed:
50% descendant opacity accumulated to 75% coverage in the overlap.

The card combines native Text/Textbox, a GPU Image, gradient and background
over horizontal Scrollers nested inside a vertical Scroller. **Move 30px**
changes geometry. **Image** toggles deterministic 2×2 RGBA artwork. Scroll the
rows and page, edit the text field, and compare the card over tiles and gutters.
GPU artwork composes before fading; native content fades separately, so this
fixture does not promise exact mixed native/vector blending.

**Hide/Show** runs the card's 800ms exit and 600ms entrance. **Reverse** starts
an exit and restores the conditional after 300ms. The mount/exit counters
distinguish rescue of the existing component from a completed unmount and new
instance. Edit the native field before reversing; the same field should remain.
The deliberately slow transitions make the retained lifetime observable.

**Late art** removes the image and supplies different decoded pixels after
350ms; **Theme** changes the background paint while the group is cached. These
are deterministic content-invalidation probes, independent of network loading.

## Validation in progress

Debug and baked release web builds, iOS simulator builds and Metal pixel
regressions pass. Web, iPhone and iPad overlap probes match at intermediate
alpha, and edited text survives opacity changes and movement. The iOS caret
remains in the field while those buttons are used, including after fading to
zero and restoring. iPhone rotation preserves the reference probe appearance.
Metal tests cover nested translucent content, clipping, restoration from zero,
and content reuse without GPU draw replay, image draw-buffer allocation or
source-image uploads.

The lifecycle extension verifies rescue without another mount, completed exit
removal, delayed decoded artwork, and changed paint on web. Runtime regression
coverage checks stable scope/draw IDs through rescue and final cleanup; a Metal
pixel regression checks refresh of a shared image without replaying the grouped
consumer. Full accessibility, network image decoding and catalog performance
are separate checks. Apple validation uses synchronous native/mask/Metal publication.
Native macOS and physical Molino also have matching overlap probes and preserve
edited text through Reverse. The fixture has its own app identity and does not
replace the physical Paxflix sessions; coordinate further device deployments
with Zack.

A Metal regression covers the image/gradient seam during fractional vertical
motion: oversized Fill artwork must stay within its crop at all tested display
scales. Fractional clips use stencil coverage; pixel-aligned rectangles retain
the scissor fast path.

Piet/Canvas2D also composes the overlap probe before fading. Its debug and baked
release checks cover reversed and completed exits, input preservation, zero
opacity/restoration, delayed artwork, theme changes and scrolling beneath the
card. Piet reuses scratch canvases but replays dirty content. To force that path
in a separate copy of this fixture, add `features = ["piet"]` to its `pax-kit`
dependency and run the same web build command. Ordinary browser selection still
chooses the renderer automatically. The physical Molino seam fix above was
checked on native WGPU, not browser WebKit.

## Physical-device CPU profiling

After coordinating a connected device, a local release run can enable these
opt-in diagnostics. Supply the development team configured for that device:

```sh
DEVICECTL_CHILD_PAX_GROUP_COMPOSITING_PROFILE=1 \
DEVICECTL_CHILD_PAX_IOS_FRAME_INSTRUMENTATION=1 \
DEVICECTL_CHILD_PAX_NATIVE_MASK_INSTRUMENTATION=1 \
cargo run -p pax-cli -- run --path examples/src/group-compositing \
  --target ipados --ios-device 'device:DEVICE_NAME' \
  --ios-development-team TEAM_ID --release --libdev
```

The fixture waits three seconds, then alternates twelve exit/entrance phases
1.4 seconds apart before returning to interactive use. `[GroupProfile]` marks
the phases. `[PaxFrame]` reports CPU frame phases and display-link callbacks;
its one-second windows include idle portions, so they are not transition-only
FPS. `[PaxNativeMask]` reports each uncached raster call's pixel size, path count
and CPU duration. Neither diagnostic measures GPU execution or displayed frames.
The timing flag is off by default and does not change mask presentation.

This five-row fixture is a bounded regression case. Its measurements cannot
substitute for Paxflix's larger catalog or Argus's previous mask measurements.

See the [design note](../../../pax-docs/book/src/design/isolated-compositing.md)
for scope, approximation, and remaining validation.
