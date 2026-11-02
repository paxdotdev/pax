# Native scroll snapping

A focused platform fixture for native snap-point updates, viewport changes,
engine feedback, and programmatic offsets between anchors.

```sh
pax-cli run --path examples/fixtures/native-scroll-snap --target web
pax-cli run --path examples/fixtures/native-scroll-snap --target windows
pax-cli run --path examples/fixtures/native-scroll-snap --target windows --release
```

Use `--target linux` on a configured Linux host. The CLI builds and packages both
the cartridge and launcher. Direct Cargo users must first build both targets;
`cargo run` alone can leave a stale cartridge that the launcher rejects. Follow
the [desktop prerequisites and direct Cargo steps](../../../pax-docs/book/src/targets-build-deploy.md#build-and-run-a-desktop-package).

Windows uses WinUI's native mandatory snapping without an opt-in. On October 26,
2026, fresh packages using the current ScrollViewer adapter passed all 36 checks
in both Debug and baked Release on Windows Server 2022/Tesla T4 at 96 DPI.
These ordinary CLI builds used the automatically extracted SDK without an
override; Release used this workspace's `opt-level="z"`, LTO, and one codegen
unit. The checks covered both axes, dynamic anchors, resizing, repeated
off-anchor requests, native/shared offset agreement, Button states, and normal
shutdown with complete exported-image retirement. Touch/trackpad momentum and
Windows client versions remain separate qualification work. The final runs used
the 784-input source snapshot (778 Windows-relevant inputs), including the DX12
vector-buffer budget correction and additional engine startup subtimings. Each
package matched 16 extracted SDK inputs. Debug used the ordinary engine opt-level
1 and fixture opt-level 0. The earlier morning 36-check passes used a separate
782-input snapshot (776 relevant inputs) before those later core changes; their
receipts remain preserved. Build each package with:

```sh
pax-cli build --path examples/fixtures/native-scroll-snap --target windows
pax-cli build --path examples/fixtures/native-scroll-snap --target windows --release
```

See the
[Windows probe guide](../../../tests/src/native-platform-probes/windows/README.md#native-snapping-and-button-states).
Linux preserves
GTK momentum, then applies the anchors after a guarded 150 ms settling period;
held input postpones the snap and fresh input interrupts it. Web acknowledges repeated off-anchor
requests even when the browser stays at its existing anchor; compare the
native offset with the shared status.

The native launcher opens a 1024 × 680 logical-pixel window. The horizontal
viewport starts at (24, 224), size 450 × 240; the vertical viewport starts at
(640, 224), size 340 × 240. The status text reports actual shared offsets.

- **Reset** restores both offsets to zero, the original viewport sizes, and
  page anchors `[0%, 100%, 200%]` on both axes. It retains the click count.
- **Pixel anchors** replaces the positions with `[0px, 125px, 325px, 625px]`.
  The last anchor exceeds the vertical scrollable range and tests clamping.
- **Clear anchors** removes both lists. Wheel input should settle freely.
- **Page anchors** restores percentage anchors without resetting offsets.
- **Resize viewports** changes horizontal width 450 ↔ 360 and vertical height
  240 ↔ 180 without remounting. Page anchors must follow the new viewport;
  pixel anchors retain their authored positions, clamped to the current extent.
- **Jump to 173** requests offset 173 on both axes. Record the native target's
  settling policy; do not assume identical off-anchor behavior across platforms.
  Repeat the request after settling and compare the native position with the
  status text: they must agree even if the second request causes no movement.
- **Switch colors** changes every Button's base/hover colors and outline width
  without remounting. The Reset button supplies a stable pixel-sampling target:
  base `(42, 68, 104)`, hover `(63, 96, 140)`, outline `(24, 35, 52)`, white text.
  The warm state uses base `(152, 53, 37)`, hover `(190, 79, 55)`, and 3px outline.
- Each page's button increments the shared click count and checks that native
  hit testing follows the settled viewport.

Use real wheel/trackpad input in each viewport and wait for native settling.
Verify repeated input can advance and reverse, and that engine acknowledgements
do not continuously retarget momentum. The fixture uses system Arial; it is not
font-fidelity or display-frame-rate qualification. Platform snapping support
and limits are documented in [Scrolling & Viewports](../../../pax-docs/book/src/scrolling-viewports.md#horizontal-regions-and-snapping).
