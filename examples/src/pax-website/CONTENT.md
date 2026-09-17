# Website editorial pass

Reference: the repository README and PAX-975's `what-is-pax.md`. These establish
the facts and mental model; the website is an invitation, not a second manual.

## Content spine

1. Living Quilt and "A declarative language for native and web UI": identify
   the open-source Rust GUI framework, the language's focused role, current
   delivery targets, and primary CLI path.
2. Give your software a character of its own: application architecture and
   creative expression belong together.
3. Describe your interface. Power it with Rust: explain the two source layers,
   reactive relationships, and live development loop.
4. One project, native and web: name current targets, explain native composition,
   and give architectural performance evidence without numeric promises.
5. Feature gallery: preserve all authored cards, artwork, qualifications, and
   local marquee behavior. This is the detailed proof inventory.
6. Your next interface starts here: install/create/run, project maturity,
   the self-contained OSS boundary, and useful destinations.

## Layout and production boundary

The September 17 intro uses the literal, category-first message in the hero and
the complementary authoring-model message in the language section. Declarative
templates describe the interface; Rust owns application logic. Avoid introducing
"UI Description Language" as a branded category or implying there is no new
syntax to learn. The quilt supplies the expressiveness without an abstract
headline. Homepage preview metadata follows the same message.

Keep the quilt-derived dark palette, Manrope headings, mono labels, fine rules,
and yellow actions. Use wide headings, readable paragraph measures, open
two-column explanations on desktop, and stacked content on mobile. Text measures
its own height; Stacker owns paragraph flow. Each section remains inspectable
through ExampleHost. Fixed host envelopes are separate from internal text flow.

No new decorative motion, fake terminals, staged animation explanations, or
placeholder proof graphics. The quilt and feature gallery carry demonstrations
until Zack directs the next creative pass. The removed builder-proof component
remains recoverable from Git history.

## Claims boundaries

- Rust is the application language; app targets are web, macOS, iOS, and iPadOS.
  Linux/Windows workstation support is not native app support.
- WebGPU has a CPU fallback; effects can differ by backend.
- Template hot reload is debug-only on current targets. Rust logic reload is
  opt-in on web/macOS; Apple mobile logic changes need rebuilding.
- Performance describes architecture, not frame-rate, idle-work, or size guarantees.
- PAX-973's final decision keeps the current framework and tooling OSS.
- Pre-1.0 and accessibility limitations stay explicit. The blog route seam stays
  intact; publishing it is not part of this editorial pass.
