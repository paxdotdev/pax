# Native controls

A focused companion to `scroll-matrix` and `neon-opacity`, covering genuine
native Slider, Dropdown, RadioList, and standalone rounded Scroller behavior.

```sh
pax-cli run --path examples/fixtures/native-controls --target web
cargo run -p native-controls
cargo run -p native-controls --release
```

The native launcher uses a 1000 × 800 logical-pixel window. The status line is
application state, so changes prove that native input reached the shared engine.
Keep the pointer outside a control when testing keyboard input after focus.

- **Reset** sets the slider to 0.5 in −2…2 with step 0.25, dropdown to Beta,
  first radio group to Alpha, and independent second group to Three.
  Slider event counts remain intact so programmatic updates cannot hide echoes.
- **Replace options** changes the shared list to Delta/Epsilon, selecting Epsilon
  in its dropdowns and radio group. The independent One/Two/Three group stays put.
- **Empty options** empties those lists. Check for stale selections, phantom
  callbacks, and crashes; Reset restores the original list.
- **Move range** moves both slider endpoints beyond the old range in one update:
  10…20, step 2, value 14. Reset exercises the reverse transition.
- **Off-grid value** sets 0.6 in the original range or 14.6 in the moved range.
  Windows/Linux retain authored values within the range and apply step to user
  input. The web range element can round its displayed value to its step grid;
  record that separately from Rust state and do not treat it as an input event.
- **Toggle controls** removes and recreates the top controls. The Scroller's
  controls remain present and share the same state.
- **Cycle radius** switches the standalone Scroller between 48, 8, and 24px.
  It has no rounded Frame ancestor that could conceal a missing Scroller clip.
  Its cyan content must leave the magenta background visible outside each corner;
  the corner button and wheel must obey the same viewport boundary.
- **Toggle viewport** changes the Scroller height from 250px to zero and back.
  Restoring it must restore its rounded clip and input boundary without requiring
  another radius change.

Use real pointer/keyboard input for drag capture, step behavior, selection,
radio-group independence, and scrolling. Also open the transformed and scrolled
dropdowns: a correct closed widget does not establish popup placement, dismissal,
or compositing. Record those separately from native accessibility roles/actions.
The two bottom-right sliders are expanded from the same repeated template. All
sliders share one value; dragging any one must update every peer without extra
slider-change callbacks from programmatic synchronization.
This fixture does not establish display frame rate or exact translucent blending.

Manrope and its license are bundled under `assets/fonts/`, matching the existing
native desktop smoke fixture. No font download is required.
