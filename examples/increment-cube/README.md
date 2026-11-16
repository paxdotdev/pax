# Increment Cube

The cube starts with 1–6 on its faces, showing 6 toward the viewer. Click or tap
to roll to 7 and beyond. Each face keeps one of the last six counts: the smallest
value is replaced on each roll.
The new value lands upright, square-on to the orthographic camera. Additional
clicks add one full pending turn per axis, then align to the selected face,
and increase speed. Directions are randomized when starting from rest; the cube
always settles one second after the last click.

A metallic gold Pax banner invites desktop users to “CLICK” and mobile users
to “TAP” (`@mobile`, independent of window size). The flag matches the settled
cube's resting width, with a thick flat white border and white Black-weight
lettering (the heaviest supported weight). It sits alongside the cube when
the component's own container is wider than it is tall;
otherwise it sits above, pointing down with upright text. The pair fits that
container together, independently of viewport orientation. Its resting gap is
41 design pixels beside the cube or 45.5 above, before the shared fit scale.
The vertical label sits slightly lower for optical centering.

Before the first roll, the sign repeats a 1.8-second bounce: its tip stays
planted while it squashes, then it stretches along the jump axis and travels
away from the cube on a 32-pixel quadratic arc. A landing squash returns it to
rest. The perpendicular scale is reciprocal, preserving area. The first roll
fades the whole sign over 360ms, then unmounts it so invisible content cannot
intercept input. The cube's offset multiplies that same opacity, sliding it
into the container's center during the fade without a second animation state.
Responsive remounts keep the sign hidden and cube centered. Both renderers
animate the hint. GPU materials respond to the host's lights; CPU
rendering retains the gold base paint and white border.

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
handler returns immediately after the hint has faded and the cube is at rest.
`src/lib.rs` owns input and publishes the pose and labels; `src/motion.rs`
contains face history, interrupted-roll math, and the hint's bounce with tests.

The cube uses the `3d` feature, a solid translucent paint, and a custom material
with subdued ambient and stronger diffuse/specular response. There are no
shader filters, gradients, or component-owned lights. The website provides its
existing global lights; the standalone wrapper supplies only ambient light.
Native face text participates in the spatial compositor, with its existing
transparency/coverage limitations; this is surface translucency, not refraction.
Back-facing labels remain mounted but transparent, keeping rear numbers from
overlapping the current number while preserving input capture during a roll.
CPU rendering uses a flat, opaque counter with the same press/release feedback.
All authored source is available through `increment_cube::sources()`.

Cube rotation is always enabled on GPU. Tick dispatch remains registered, but
skips motion calculations and property writes at rest after the hint fades.
There is no viewport proximity listener; the hint and active rolls keep their
clock while mounted, including offscreen. The example does not read an
operating-system motion preference. The bundled
Manrope font is licensed under the SIL Open Font License in
`assets/fonts/OFL-Manrope.txt`.

For the smaller introductory counter, see [Increment](../fixtures/increment).
Its CLI starter remains `pax-cli create my-counter --example=increment`.
See [Animation and Motion](../../pax-docs/book/src/animation-motion.md#wind-up-a-roll)
for the teaching walkthrough.
