# Website editorial pass

Reference: the repository README and PAX-975's `what-is-pax.md`. These establish
the facts and mental model; the website is an invitation, not a second manual.

## Content spine

1. Animated Pax logo and "A declarative language for creative / high-performance /
   native / portable user interfaces": a three-line rotating headline, compact
   paragraphs connecting declarative Pax, Rust application logic, and the four shipping
   targets, followed by the primary CLI path. Deeper explanation stays below.
2. Give your software a character of its own: application architecture and
   creative expression belong together.
3. Describe your interface. Power it with Rust: explain the two source layers,
   reactive relationships, and live development loop.
4. One project, native and web: name current targets, explain native composition,
   and give architectural performance evidence without numeric promises.
5. Feature gallery: preserve all authored cards, artwork, qualifications, and
   local marquee behavior. Hot reloading leads, followed by path drawing;
   offscreen auto-advance pauses to preserve that first impression.
   Layered Paint and Dynamic Vector Masks follow with real grayscale demonstrations
   and links to the drawing/compositing guides; alpha masking stays GPU-qualified.
6. Your next interface starts here: install/create/run, project maturity,
   the self-contained OSS boundary, and useful destinations.
7. Do the math: a live graphing calculator in a full-bleed phone silhouette,
   with the floating island, real keypad/graph interaction, and inspectable
   canonical Pax and Rust source. A quieter technical proof after the main CTA.

## Layout and production boundary

The September 25 hero keeps the literal, category-first message and makes the
logo and type its visual focus. Declarative templates describe the interface;
Rust owns application logic. Avoid introducing "UI Description Language" as a
branded category or implying there is no new syntax to learn. Preview metadata
keeps a stable category description rather than following the animated word.

Keep the dark palette, Manrope headings, mono labels, fine rules,
and neutral actions. Use wide headings, readable paragraph measures, open
two-column explanations on desktop, and stacked content on mobile. Text measures
its own height; Stacker owns paragraph flow. Each section remains inspectable
through ExampleHost. Fixed host envelopes are separate from internal text flow.

The hero uses native text with affine word transitions; “creative” uses
Handwriter's bundled EMS League strokes revealing the proving fixture's cycling
CMY gradient. This is temporary paint while PAX-1011 explores genuine mesh gradients.
A native text equivalent stays outside the mask. Both writing and paint obey
the hero's pause/offscreen controls.
The animated logo is imported from its canonical example. Decorative contours
remain unplugged; the feature gallery and device studies carry the deeper
demonstrations. Removed experiments remain recoverable from Git history.

## Claims boundaries

- Rust is the application language; app targets are web, macOS, iOS, and iPadOS.
  Linux/Windows workstation support is not native app support.
- WebGPU has a CPU fallback; effects can differ by backend.
- Template hot reload is debug-only on current targets. Rust logic reload is
  opt-in on web/macOS; Apple mobile logic changes need rebuilding.
- Performance describes architecture, not frame-rate, idle-work, or size guarantees.
- PAX-973's final decision keeps the current framework and tooling OSS.
- Pre-1.0 and accessibility limitations stay explicit. `/blog` belongs to the
  sibling Zola site in `pax-blog/`, not a Pax Router placeholder. Preserve the
  server-owned navigation boundary; publication needs separate approval.
