# LED Oscilloscope

A front-facing instrument cabinet with a coarse round RGB matrix. Cyan, magenta,
and yellow waves share a center baseline behind a bold, vertically centered
marquee. All content is sampled onto discrete on/off cells. Round lamp edges and
lens highlights are drawing details; they do not introduce partial edge cells.

```sh
pax-cli run --path examples/led-oscilloscope --target web
cargo run -p led-oscilloscope
```

The controls form four vertical pairs. Each color has an amplitude knob above
a frequency knob; frequency ranges from 0.25× to 4×, with the original wave
spacing at the midpoint, 1×. RATE adjusts wave motion and marquee speed together.
SATURATION beneath RATE defaults to zero (white) and blends the foreground to a vibrant
rainbow at maximum. Each text or drawing column keeps its hue while scrolling;
the rainbow repeats every 112 source columns and does not change with time.
Scroll in the direction that moves a webpage down/right
to increase; scroll up/left to decrease. Drag up/right to increase and down/left
to decrease. Values are bounded; reversing direction at a limit responds immediately.
Mouse and touch drags keep their original target through release/cancel,
including outside the hit region and across other controls. Releasing a knob
over the LED display does not open the editor. The cabinet fills its assigned
width and height, including the standalone window, with no outer margin or
maximum-width cap. The four pairs sit beside the display at widths of at least
1040 pixels, or in landscape containers at least 800 pixels wide with a 1.5:1
aspect ratio; otherwise they move below it. The visible column count adapts to
the remaining display area, keeping the LEDs round and the text centered.

Click or tap anywhere on the LED display to open its fading editor. The Text
tab uppercases supported ASCII and drops unsupported characters. The Draw tab
provides a signature pad with a thick, round brush, plus Clear and OK buttons.
Text and drawing drafts survive tab switches and reopening; OK applies the
selected mode and restarts the marquee. Escape dismisses the editor without
applying the current draft, including from the focused web textbox. The editor blocks pointer interaction
with the cabinet while open.

Draw with the mouse or one finger. Leaving the pad lifts the pen; returning
requires a new press, so separate strokes do not reconnect. The pad retains
its 56×21 visible logical aspect ratio as the editor resizes. Scroll sideways
with a trackpad or use the bar below the pad to continue a longer message.
The native Scroller starts with four windows of canvas and adds empty space
as the drawing approaches the right edge, up to 32 windows. Scrolling lifts
the pen, so moving the viewport does not join distant strokes. Its three-cell brush is
sampled onto the LEDs with four clear rows above and below, and the drawing
scrolls in front of the waves just like text, omitting unused canvas beyond
the last mark. Empty drawings show only waves.
Clear starts a fresh drawing without changing the text draft.

## Reuse

Depend on this package with `default-features = false`, import
`led_oscilloscope::LedOscilloscope`, and embed it directly:

```pax
<LedOscilloscope width=100% height=420px message="DREAM IT. BUILD IT."/>
```

The component uses both dimensions supplied by its host. Use `height=100%` to
fill a viewport, or choose a panel height for an embed. For a suggested height
based on width, use `LedOscilloscope::height(available_width)` in Rust, as the
website's `FrameworkSection` does. The host-provided `paused`
property can suspend updates while retaining the scene. Each instance owns its
own display and controls. Motion starts automatically and stops outside the
viewport. System reduced-motion settings are not yet connected; that work is
tracked by [PAX-1048](https://linear.app/paxdev/issue/PAX-1048).

Messages support ASCII letters (lowercase folds to uppercase), digits, spaces,
and `. , ! : - + / ' ?`. Other characters are dropped; input is bounded to 256
characters. The original 5×7 font is enlarged to 15×21 cells with three-LED
strokes and three columns between letters. Spaces advance 12 columns, two-thirds
of a regular letter's 18-column advance. It is centered in 29 rows, with four
clear rows above and below, and scrolls in integer columns. The complete message
does not fit at once.

## Rendering

`display.rs` composes a bounded row-major cell buffer. Text overwrites wave
cells; there is no font rasterizer or continuous curve hiding behind the grid.
The display fits up to 96 visible columns to its allocated area at a square
cell pitch, retaining 29 rows at every size. The suggested-height helper aims
for 56 columns, or 28 when the cabinet is narrower than 600 pixels.
Text and drawn content share a cached column mask; content composition and
lamp appearance are independent. The signature pad keeps at most 8192 sampled
points and rebuilds its smooth preview only after drawing or resizing. Its
LED mask is rasterized once on OK, not on animation frames. Arbitrary image
import is not implemented.

The cabinet and reusable `InstrumentKnob` are Pax templates. Lamp contours are
batched by color into a few `Path` nodes, following the calculator's batched
plot geometry pattern. Unlit lamps and highlights rebuild only on resize;
animated paths publish only when the cell buffer changes, at most 30 times per
second.

`ForegroundInk` in `foreground.rs` colors the white foreground in one full-resolution
shader pass. Its column address matches the display buffer, including message
wraparound. Saturation and scrolling change uniforms without creating a path
for every rainbow color. The colored foreground then enters `LedGlow` with the
wave paths, so the halo follows the lamp colors.

`LedGlow` in `led_glow.rs` is an example-owned shader primitive using Pax's
experimental shader API. It captures only the lit paths, blurs them horizontally
at half resolution, then blurs vertically and composites the original crisp
bulbs over the halo at full resolution. Both passes use a fixed nine-tap kernel.
Faint stationary grain modulates the halo, so moving light reveals texture
without another animation clock. Off lamps, the cabinet, and native controls
stay outside the capture. LED occupancy remains binary. Together, foreground
coloring and glow use three filter passes per effect tile, plus their subtree captures.

Adjust `strength` on `LedGlow` in `lib.pax` (0–1; initially 0.85) to compare the
appearance; zero returns the original image but still mounts the filter.
`pitch` scales the halo with the bulbs. The capture bounds and sampling support
stay fixed while the content animates, allowing the renderer to reuse its
shader pipelines and textures. Changed LED geometry still needs capture and
filter work; this is not a claim that animation is free. Offscreen suspension
also stops the glow's source updates.

The package enables `pax-kit/shaders`. This treatment requires the WGPU renderer
(WebGPU on web); the legacy Piet backend does not support shader containers.
Shader support adds runtime code even when no effect is mounted. See the
[shader container guide](../../pax-docs/book/src/compositing-effects.md#experimental-shader-containers)
for caching, compositing, and backend restrictions.

The same component supplies the website's creative-freedom headline panel.
