# Native wheel input

A small end-to-end fixture for physical wheel/trackpad input, Rust `@wheel`
handlers, bubbling, default prevention, and native Scroller ownership. It is
intended for Debug and baked Release qualification on Windows and Linux, with
web as a comparison target. Linux and web provide the handler/default-prevention
contract. The experimental Windows ScrollViewer adapter implements the same raw
mouse-wheel bridge before native defaults. Its integrated NativeScene scrolling
and mask/input contracts have passed on Server 2022/T4. Native child-control and
scrollbar checks cover defaults and cancellation with visible and captured
controls, both Viewer axes, and empty viewports. The packaged fixture results
below cover this replacement adapter. Touchscreens, precision touchpads, and
open native popups remain unqualified. Apple hosts forward scroll input through
`@scroll` and do not provide this fixture's `@wheel` contract. The vector graph is
ordinary scrollable content, not a custom wheel-to-position implementation.

On October 25, the ordinary Linux Release CLI build, with automatic embedded
SDK extraction and no SDK override, passed all 162 strict assertions after
installation as a Flatpak. The run used Ubuntu, the pinned GNOME 50 runtime
(GTK 4.22.5), Xorg at scale 1, and an NVIDIA Tesla T4. It checked exactly one
callback per handler for each injected discrete notch, both scroll axes,
native/bound position agreement, root cancellation, rounded ownership,
modifiers, normal close, and GL resource retirement. Both vector-only overlay
points stayed outside the observed native text bounds, with cancellation
disabled and both Scrollers stationary. This qualifies the tested mouse-wheel
path, not arbitrary trackpad or touchscreen gestures.

On October 26, fresh Windows Debug and default baked Release packages each passed
all 121 assertions across 14 cases on Server 2022/Tesla T4, WinUI 1.8, at 96 DPI.
Real `SendInput` notches reached each handler exactly once, preserved both axes
and window-space coordinates, respected root cancellation and rounded ownership,
and agreed with read-only native UI Automation offsets within one pixel. The
vector-only overlay points stayed outside the observed native label bounds with
cancellation disabled and both Scrollers stationary. Both apps closed normally
with exact GPU resource retirement.

Both used the ordinary CLI's automatic embedded SDK build, without an SDK
override. Debug used workspace opt-level 1; Release used the default opt-level
`z`, LTO enabled, and one codegen unit. The audited final-core sources include the
DX12 retained-buffer cache charge and opt-in startup subtimings. The earlier
October 25 Release pass used a pinned SDK override and opt-level 3, LTO disabled,
and 16 codegen units; its receipt remains separate. The
[Windows probe instructions](../../../tests/src/native-platform-probes/windows/README.md#packaged-wheel-callbacks)
describe the reusable checker, source audits, and dated qualification scope.

```sh
pax-cli run --path examples/fixtures/native-wheel --target web
pax-cli run --path examples/fixtures/native-wheel --target windows
pax-cli run --path examples/fixtures/native-wheel --target windows --release
```

Use `--target linux` on a configured Linux host. The CLI builds and packages both
the cartridge and launcher. Direct Cargo users must first build both targets;
`cargo run` alone can leave a stale cartridge that the launcher rejects. Follow
the [desktop prerequisites and direct Cargo steps](../../../pax-docs/book/src/targets-build-deploy.md#build-and-run-a-desktop-package).

The native launcher opens a 1024 × 780 logical-pixel window. The outer viewport
is `(24, 260, 760, 430)`, with a 1360 × 900 content pane and 40px corner radius.
Initially the inner viewport is `(104, 360, 440, 260)`, with a 940 × 660 content
pane and 32px radius. Both axes have room to move. The red root overlay at
`(410, 410, 220, 90)` is a sibling of the outer Scroller and stays fixed.

The top panel displays individual handler counts, both bound Scroller positions,
and the last raw wheel delta, window position, modifiers, and cancellation
state. Raw delta magnitude is device/platform dependent; it is not the content
travel in pixels. A physical gesture may produce several events. Compare the
increments along the same handler path, not an assumed number of events per
notch. Momentum can change offsets without delivering additional raw wheel events.

Use actual wheel or trackpad input, including a horizontal gesture where the
device supports it. Injecting `Scroll` or setting offsets through development
commands does not test the `Wheel` bridge or native default prevention.

1. **Nested movement:** after Reset, wheel at `(250, 500)` in the purple graph.
   Each delivered event increments inner, outer, and root exactly once; overlay
   stays unchanged. The inner native position moves while it has room, and its
   bound position must follow. Reverse and exercise X as well as Y. At an inner
   edge, record the platform's native chaining policy rather than demanding
   identical momentum behavior across toolkits. Isolated Windows ScrollViewer
   checks currently consume further same-axis wheel input at either boundary;
   reversing direction moves the inner viewport. An axis with no travel has a
   separate native route to an eligible ancestor.
2. **Ancestor-only movement:** Reset, then wheel at `(720, 580)` in the outer
   blue grid. Only outer and root increment; inner positions/count stay fixed.
3. **Cancellation:** wait for all momentum to stop, check **Prevent native
   default**, then repeat inside purple and blue. The same handler paths still
   increment once per event, but neither native position changes. Uncheck it
   and verify native movement resumes. This cancels at the root handler, after
   the inner/outer handlers have already observed the same event.
4. **Opaque sibling:** Reset and leave cancellation unchecked. Wheel at
   `(460, 450)` on the red overlay, geometrically above the inner Scroller,
   then at `(600, 450)` above the outer-only area. Only overlay and root
   increment. Repeat at `(418, 475)` and `(622, 475)`, which are inside the
   red vector but outside the overlay's native text. Verify those points are
   outside the observed text bounds before testing: a native text hit alone does
   not qualify the vector mask's input ownership. Neither viewport may acquire
   the unrelated sibling's default action; cancellation remains unchecked and
   the overlay handler itself does not call `prevent_default()`.
5. **Rounded ownership:** Reset before each point. At `(108, 364)`, inside the
   inner bounding box but outside its rounded clip, only outer and root should
   receive input, and only the outer viewport may move. At `(28, 264)`, outside
   the outer rounded clip, only root should receive input and both offsets stay
   fixed. Compare with nearby points inside each curved edge.
6. **Modifiers and lifetime:** hold Shift, Control, Alt, or Command/Windows
   while sending input that the OS does not reserve. Verify the recorded
   modifier without assuming Shift converts Y to X on every platform. Close
   and relaunch to verify fresh counters/positions and repeat an inner gesture.

**Reset** disables cancellation and resets positions, counters and the last
sample. Let native acknowledgements settle before beginning the next step.
**Snapshot to log** writes one compact `pax-native-wheel:` line to stderr and,
when `PAX_NATIVE_WHEEL_LOG` is set, appends it to that file. This keeps the same
observations available in packaged Release without development endpoints.
Capture snapshots before and after each step, alongside native position
inspection where available; visible counters alone do not prove scrolling.

This fixture is a functional input check, not a framerate benchmark or proof of
all device/gesture combinations. It uses system Arial. See
[Event Handling](../../../pax-docs/book/src/event-handling-rust.md#default-actions)
and [Scrolling & Viewports](../../../pax-docs/book/src/scrolling-viewports.md)
for the shared event and native scrolling contracts.
