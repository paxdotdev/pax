# Increment Cube

The centered cube initially reads **CLICK** on desktop or **TAP** on mobile
(`@mobile`, independent of window width). Its count starts at zero. The first
activation spins the cube and lands on **1**; each later activation rolls to
the next number. From 7 onward, the newest value replaces the oldest of the
six printed numbers. The count survives responsive remounts in the website.

Each roll adds one full pending turn per axis, then aligns the new number
upright and square-on to the orthographic camera. Directions are randomized
when starting from rest; the cube settles one second after the last activation.

Hovering the cube shows a pointing-hand cursor, including on the flat CPU counter.
Pressing compresses the cube. Releasing makes it expand briefly and return to
normal; cancellation relaxes it without incrementing. Mouse and touch share
this feedback. There is no separate button or keyboard activation control.
Captured press/release events keep rapid pokes reliable while faces move under
the pointer. Moving more than 12 window pixels cancels the poke; `src/input.rs`
owns this small gesture recognizer and its tests.

From the repository root:

```sh
pax-cli run --path examples/increment-cube --target web
cargo run -p increment-cube  # native macOS
```

`src/increment.pax` contains the GPU/CPU branches and six live face labels.
Geometry, rotation/scale bindings, material and typography live in its local
`@settings` block, including the component's `@tick: tick` binding. The Rust
handler returns immediately at rest. `src/lib.rs` owns input and publishes
the pose and labels; `src/motion.rs` contains face history and interrupted-roll
math.

With `3d` and `shaders` enabled on WGPU, each `CuboidFace` paints its own
translucent rectangle through `RimRefraction`. The captured contour stays on
that face as it rotates; the native label sits outside the capture. The cube
uses its host's lighting: a fixed point in the standalone wrapper, or the
moving pointer light on the website. Ambient light alone does not light the rim. Rear labels
remain mounted but transparent, so they do not obscure the current number or
interrupt a captured poke. CPU rendering uses the same invitation, count, and
press/release feedback without the 3D roll or shader.

All authored source is available through `increment_cube::sources()`.
Active rolls continue while mounted, including offscreen; the example does
not read an operating-system motion preference. The bundled
Manrope font is licensed under the SIL Open Font License in
`assets/fonts/OFL-Manrope.txt`.

For the smaller introductory counter, see [Increment](../fixtures/increment).
Its CLI starter remains `pax-cli create my-counter --example=increment`.
See [Animation and Motion](../../pax-docs/book/src/animation-motion.md#wind-up-a-roll)
for the teaching walkthrough.
