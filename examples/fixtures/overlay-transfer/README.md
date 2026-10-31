# Overlay Transfer

A stateful logo moves between a Scroller and a fixed navigation Group. Two
independent Overlays contribute the navigation and control panel. The latter is
authored beneath a transformed, translucent Group inside the Scroller, making
viewport isolation visible.

From the monorepo root:

```sh
cargo run -p pax-cli -- run --path examples/fixtures/overlay-transfer --target web --libdev --port 8097
cargo run -p pax-cli -- build --path examples/fixtures/overlay-transfer --target web --libdev --release
pax-cli docs examples overlay-transfer
```

Edit the note and increment the counter, then scroll beyond 234 px. The same
logo should ease into the nav and return when scrolling back. The mount number,
private counter, and text binding should remain unchanged. Resize or reverse
while the logo moves to exercise interruption and destination layout. A reactive
subscription responds to scroll-position changes in the same effect drain;
the 234px boundary is where the home slot meets the nav's lower edge. Returning
uses 210px to avoid jitter at the boundary. The 320ms ease-out gives immediate
visible motion while retaining the same instance. During the flight, canvas
and native content stay above the nav; destination clipping resumes on arrival.

Native browser scrolling can continue while the application thread is busy.
The transfer still needs that thread to run: the fixture's wall-clock animation
catches up after a late frame, but cannot guarantee uninterrupted motion during
a main-thread stall. Add `?pax_renderer=cpu&pax_scroll_perf=1&pax_surface_trace=1`
to open the built-in trace/probe panel when diagnosing delays.

Use the controls to exercise ownership:

- **Cleanup: return**: visit the nav, then unmount it. `on_pre_unmount` iterates
  `ctx.visiting_nodes()` and calls `move_to_home()`. The logo retains its mount.
- **Cleanup: abandon**: visit the nav, then unmount it. The logo fully unmounts
  and the original component reports one `detached_nodes()` entry. Remounting
  the nav does not re-present it. **Recover home** mounts it fresh, resetting
  the counter and note and advancing its mount number.
- **Remove owner**: remove the original declaration while the logo is visiting
  or detached. Its instance and recovery entry are discarded. Restoring the
  owner creates a new logo at home.
- Manual actions disable automatic scroll transfer; **Scroll: off** enables it
  again, immediately applying the current threshold.

Native DOM identity is retained during live moves. Focus and selection also
survive connected moves on browsers implementing `Element.moveBefore`. Older
browsers fall back to ordinary DOM reattachment, which does not guarantee that
transient browser state. Abandonment deliberately destroys native state.
Apple view identity uses the existing retained-view path; first-responder
preservation needs platform acceptance before being promised.

The public API and lifecycle contract are described in
[Events & Rust](../../../pax-docs/book/src/event-handling-rust.md#find-and-move-existing-nodes)
and [Compositing & Effects](../../../pax-docs/book/src/compositing-effects.md#fixed-overlays).
