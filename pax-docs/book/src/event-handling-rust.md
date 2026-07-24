# Event Handling & Rust Logic
<!-- summary: Event handlers, Rust interop, and timing patterns. -->
<!-- tags: events, rust, interop -->

- Event handlers: `@click`, `@tap`, `@mouse_move`, `@key_down`, `@textbox_change`, `@gyro`, `@accel`.
- `@click` and `@tap` both handle mouse clicks and single-touch taps by default, and both receive `Event<Click>`. If both handlers are present on the same element, mouse input routes to `@click` and touch input routes to `@tap`. Touch activation fires after `TouchEnd`, at release rather than initial contact.
- Use lower-level handlers such as `@mouse_down`, `@mouse_up`, `@touch_start`, `@touch_end`, and `@touch_cancel` when a handler needs mouse-only or touch-only behavior.
- Lifecycle handlers can still be bound explicitly in `@settings`, but `on_mount`/`mount`, `on_tick`/`tick`, `on_pre_render`/`pre_render`, and `on_unmount`/`unmount` are auto-bound when those Rust methods exist.
- Interop with Rust: calling methods, updating properties, easing values.
- Timing and game loops: patterns from `breakout`, `space-game`, and `particles`.

Mouse and touch coordinates are expressed in window space. Convert them through
the receiving node before using them for local interaction geometry:

```rust
pub fn mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
    let local = ctx.local_point(Point2::new(event.mouse.x, event.mouse.y));
    let (width, height) = ctx.bounds_self.get();
    let x = local.x.clamp(0.0, 1.0) * width;
    let y = local.y.clamp(0.0, 1.0) * height;
    // Update the local effect using x and y.
}
```

`local_point` returns normalized local coordinates, so multiply by the node's
bounds when pixel-like values are needed. This conversion accounts for the
node's current transform.

The primary touch identifier captures the topmost hit node at `TouchStart`.
`TouchMove`, `TouchEnd`, and `TouchCancel` for that sequence continue to target
the captured subtree even when the finger moves beyond its bounds. `TouchEnd`
releases a normally completed sequence, including one that simultaneously drove
a native `Scroller`. `TouchCancel` releases a sequence the platform aborted
before normal release, such as during a system interruption; cancelled touches
never synthesize `@click` or `@tap`. Components should retain the identifier
they accepted and clear pressed or dragging state in both their end and cancel
handlers.

Controls inside a `Scroller` receive `TouchStart` immediately, so optimistic
pressed or illuminated feedback can appear before the gesture is resolved. The
child continues receiving captured `TouchMove` events while the Scroller pans,
then receives `TouchEnd` on release. Because the moving content transform is
part of local-coordinate conversion, a contact that moves with the scroll stays
approximately fixed vertically on its original child while horizontal finger
motion remains visible. Crossing the platform scroll threshold disqualifies
`@click` or `@tap` activation without cancelling the lower-level touch stream.

<pax-example
  path="space-game"
  title="Space Game"
  height="560"
  files="src/lib.pax,src/lib.rs,src/animation.rs">
</pax-example>
