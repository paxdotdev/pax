# Event Handling & Rust Logic
<!-- summary: Event handlers, Rust interop, and timing patterns. -->
<!-- tags: events, rust, interop -->

- Event handlers: `@click`, `@mouse_move`, `@key_down`, `@textbox_change`.
- Lifecycle handlers can still be bound explicitly in `@settings`, but `on_mount`/`mount`, `on_tick`/`tick`, `on_pre_render`/`pre_render`, and `on_unmount`/`unmount` are auto-bound when those Rust methods exist.
- Interop with Rust: calling methods, updating properties, easing values.
- Timing and game loops: patterns from `breakout`, `space-game`, and `particles`.

<pax-example
  path="space-game"
  title="Space Game"
  height="560"
  files="src/lib.pax,src/lib.rs,src/animation.rs">
</pax-example>
