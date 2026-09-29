# Pain points — raw findings inbox

This is a temporary intake for findings encountered during work, not a reference
or a prerequisite for agents to read before starting a task.

Add concise, dated reports: what you tried, expected versus observed behavior,
reproduction steps or evidence, relevant target/revision, and any workaround or
related Linear issue. Link repeated reports instead of copying long journals.

Periodically triage the inbox: discard resolved or obsolete findings, preserve
ongoing issues and report details in Pax Core Linear tickets, and remove each
processed entry. Keep only exceptional proactive guidance in `AGENTS.md`.
Design exploration and specifications belong directly in Linear, not here.

Last triaged: 2026-10-05 — [PAX-972](https://linear.app/paxdev/issue/PAX-972/pay-down-the-pain-pointsmd-file).

## Untriaged findings

## 2026-10-06 — macOS display-link startup

A macOS example can open a blank window if CoreVideo reports
`CVDisplayLinkCreateWithCGDisplays error -6661` with an active display count of
zero. In the Scroll Matrix smoke test, relaunching after the display became
available restored rendering without source changes. Inspect the app's system
log before treating an idle blank window as a renderer regression. The current
display-link setup does not retry that startup failure automatically.

- 2026-10-11, shader-studio / PAX-1035: PAXEL does not accept Rust method calls
  such as `self.phase.sin()`. Expose a `#[helpers]` associated function and call
  `Landscape::sun_x(self.phase)`; grouped units work as `{(Landscape::sun_x(self.phase))%}`.
  Numeric literals need a fractional digit (`24.0`, not `24.`). Frame's
  `corner_radius` is `f64`, so use `18`, not `18px` (the latter warns and keeps
  the default).
- 2026-10-11, shader-studio web debug build: a fresh resolution to Pest 2.9.3
  introduced the psm Wasm stack helper. macOS `ar`/`ranlib` warned that `wasm32.o`
  was not Mach-O; the emitted JS imported unresolved `env::rust_psm_stack_pointer`
  and the browser stayed blank. Install Rust's official `llvm-tools-preview`
  component and set `AR_wasm32_unknown_unknown` to that toolchain's `llvm-ar`
  for the web build. This links the real helper and removes the `env` import.
- 2026-10-11, shader-studio / PAX-1035: `pax-cli dev look` places the styled
  mode-button labels at the upper left, although the live browser centers them
  with `TextAlignHorizontal::Center` and `TextAlignVertical::Center`. Reproduce
  with the example's `.mode_button` class and compare a dev-look PNG to a browser
  screenshot. Use the browser to judge native-control alignment; the capture
  remains useful for inspecting GPU brush pixels. After hot reload and scrolling,
  dev-look captures also showed native buttons/sliders duplicated at old positions
  while the live browser remained correct.

## 2026-09-29 — Small live studies inside a virtualized gallery

A nested Rust component imported as `use paint_demo::FeaturePaintDemo` passed
Rust tests but failed Pax static analysis with an unresolved type. Use the
crate-qualified path (`crate::feature_gallery::paint_demo::FeaturePaintDemo`)
and expose the module so generated cartridge code can resolve it too. Test
the actual web build as well as the Rust library.

Mask sources do not receive viewport-proximity events. Keep the animation clock
on the visible demo wrapper, use its clipped global viewport sample to pause
offscreen work, and bind progress into the source. This covers both the page's
vertical clipping and the rail's horizontal clipping without polling geometry
per frame. Keep meaningful text alternatives outside the detached mask source.

For moving CMY paint through handwriting, repeat the paint period and translate
the shared parent instead of regenerating the letter paths. Keep the paint clock
separate from a signboard clock that snaps to a legible word on pause; otherwise
pausing also jumps the colors. Palette indexing in PAXEL uses `%%` for modulo;
the single `%` token denotes a percentage unit.

For the subsequent one-shot pinstripe treatment, scale across the stripes before
rotating them so their 45-degree axis stays fixed. Compute the rotated coverage
when panning them out; moving just their center past the word can still
leave a colored tail. Keep the white underpaint and alpha source unlit, and scope
the stripe light with LightFrame so the accent does not relight the rest of the
page. The visible wrapper owns the clock; pause can finish a one-shot to its clean
resting state instead of freezing a bright glint.

When those bands must remain contiguous, use one rectangle with hard CMY gradient
stops instead of separate spaced rectangles: scaling also scales any authored
gaps. Pipeline the glint with writing on the same clock. For a diagonal ribbon,
compute its final travel against the vertical slice occupied by the mask, not
the full long rectangle; otherwise its visible portion exits before its large
off-mask ends, making the effect seem to finish early.

When replacing a lighting study with flat moving paint, explicitly use unlit
material on both the content and mask source; deleting the local light alone
would still allow other scene lights to influence a default-lit shape. The
cycling gradient placeholder reuses the mask-strokes fixture's two repeated
CMY periods and translates one period in four seconds. Keep this paint clock
independent of writing completion, so pause finishes the lettering without
jumping its colors. Preserve the parked pinstripe as a clock-free component
instead of leaving its per-frame work mounted invisibly. Genuine mesh-gradient
paint is implemented separately in PAX-1011; overlapping radial gradients and
this repeating linear gradient are not substitutes for it.

When rebasing component-store tests onto the Paint/Fill split, migrate recorder
trait signatures to `Paint`, implement the image-opacity entry point, and use
the shared designtime-aware runtime/engine fixture constructors. A default-only
test run would miss the latter integration issue.

## 2026-09-29 — Embedding a complete example without a second runtime

Expose the calculator as a normal component and keep a thin standalone root;
a Cargo path dependency lets the website reuse its actual source, model, and
per-instance store. A viewport-proximity host can defer the first mount, then
retain session state while gating the example's update handler offscreen. This
is not general subtree suspension: mounted nodes and retained GPU resources
still exist after the first visit. Measure heavier examples independently.

An embedded calculator needs an explicit keyboard policy: @key_down delivery
is global, so an offscreen mounted example must not consume the page's keys.
Disable its keyboard handler in this host while keeping on-screen controls live.
Let the host supply logical safe-area insets for a mock phone rather than
letting zero-valued web DynamicIslandSpacers overwrite them. Keep the background
full-bleed and inset only the controls. Put the island before the clipped slot
in Pax source so it renders above the app. Suppress layer-wide AmbientLight in
embedded mode; a LightFrame alone does not localize that lighting resource.

Frame uses `corner_radius`, not `border_radius`; the latter did not round the
website's old device clip. Keep the hardware outline and actual clipping shape
aligned. For mouse dragging inside a transformed example, convert both the
initial and current window points through the same `ctx.local_point` and then
multiply their delta by local bounds; raw window deltas under-pan a scaled app.
