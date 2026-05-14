# Event Handling & Rust Logic
<!-- summary: Event handlers, Rust interop, and timing patterns. -->
<!-- tags: events, rust, interop -->

- Event handlers: `@click`, `@tap`, `@mouse_move`, `@key_down`, `@textbox_change`, `@gyro`, `@accel`.
- `@click` and `@tap` both handle mouse clicks and single-touch taps by default, and both receive `Event<Click>`. If both handlers are present on the same element, mouse input routes to `@click` and touch input routes to `@tap`.
- Use lower-level handlers such as `@mouse_down`, `@mouse_up`, `@touch_start`, and `@touch_end` when a handler needs mouse-only or touch-only behavior.
- Lifecycle handlers can still be bound explicitly in `@settings`, but `on_mount`/`mount`, `on_tick`/`tick`, `on_pre_render`/`pre_render`, and `on_unmount`/`unmount` are auto-bound when those Rust methods exist.
- Interop with Rust: calling methods, updating properties, easing values.
- Timing and game loops: patterns from `breakout`, `space-game`, and `particles`.

<pax-example
  path="space-game"
  title="Space Game"
  height="560"
  files="src/lib.pax,src/lib.rs,src/animation.rs">
</pax-example>
