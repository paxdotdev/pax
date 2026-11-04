# Group compositing

Focused reproduction and acceptance fixture for [PAX-1004](https://linear.app/paxdev/issue/PAX-1004/add-isolated-group-compositing-for-mixed-nativecanvas-content).

From the monorepo root:

```sh
cargo run -p pax-cli -- run --path examples/fixtures/group-compositing --target web --libdev
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

## Backend behavior

Vello CPU composes overlapping shapes before fading and keeps browser-native
text and controls in their existing hosts. Force it with `?pax_renderer=cpu`;
use `?pax_renderer=gpu` for the GPU comparison. Software rendering replays dirty
content using reusable raster buffers; it does not retain composed group pixels
across frames.

## Physical-device CPU profiling

After coordinating a connected device, a local release run can enable these
opt-in diagnostics. Supply the development team configured for that device:

```sh
DEVICECTL_CHILD_PAX_GROUP_COMPOSITING_PROFILE=1 \
DEVICECTL_CHILD_PAX_IOS_FRAME_INSTRUMENTATION=1 \
DEVICECTL_CHILD_PAX_NATIVE_MASK_INSTRUMENTATION=1 \
cargo run -p pax-cli -- run --path examples/fixtures/group-compositing \
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
substitute for measurements of Paxflix's larger catalog.

See the design and validation archive in [PAX-1004](https://linear.app/paxdev/issue/PAX-1004)
for scope, approximation, and remaining validation.
