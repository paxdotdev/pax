# Pax website

The first-party Pax application lives at root-level `pax-website/`, beside the
static Zola site in [`pax-blog/`](../pax-blog/README.md). It is not a reusable CLI
example or a publishable crate. The monorepo explicitly excludes it from its
Cargo workspace so the site's standalone profiles remain effective: development
dependencies use optimization level 1; the website application uses level 0.

## Local development

From the monorepo root on this workstation:

```sh
source ~/.zshrc
nvm use
./pax-website/pax run --target web
```

The launcher builds this checkout's CLI, selects the website by its own location
(not the invoking working directory), and uses `--libdev`. The equivalent direct
command is:

```sh
cargo run -p pax-cli -- run --path pax-website --target web --libdev
```

For an optimized web build without serving or publishing:

```sh
./pax-website/pax build --target web --release
```

To preview the same hero on Apple targets, run one of these commands:

```sh
./pax-website/pax run --target macos
./pax-website/pax run --target ios --ios-device simulator
./pax-website/pax run --target ipados --ios-device simulator
```

Stop the previous CLI preview before switching targets: one project shares its
generated cartridge and interface staging directory across targets.

Debug and release outputs live in `.pax/build/debug/web/` and
`.pax/build/release/web/` beneath this directory. Generated `.pax/`, `target/`,
`Cargo.lock`, AI-primer HTML, and staged blog previews stay untracked.

The hero uses `AnimatedPaxLogo` from `pax-std` through the `pax_kit` re-export.
View Source includes the canonical files in `pax-std/src/drawing/pax_logo/`.
Living Quilt remains independently runnable in `examples/living-quilt/`; neither
it nor the `pax-logo` harness is a website dependency. Cargo dependency paths
and favicon paths are relative to this manifest; Rust `include_str!` paths are
relative to their source file.

## Website and blog boundary

The website declares `/blog`, `/ai`, and `/ai.md` as `server_owned_prefixes`.
Navigating to these paths loads a server document instead of a Pax client route.
The `/blog/*` Router placeholder must not be restored. `build.rs` generates
`public/ai/index.html` from `public/ai.md` only when its contents change.

To check the separate blog locally, follow its setup instructions, start the
website, and use the website's printed port:

```sh
python3 pax-blog/blog.py stage --port <website-port>
```

This is a draft preview, generated into `pax-website/public/blog/`. It is not
production output. Remove it before a website release intended for publication:

```sh
python3 pax-blog/blog.py unstage
python3 pax-blog/blog.py build
```

The second command independently builds production blog files, excluding drafts,
into `pax-blog/public/`. Neither command deploys anything. Website publication
must exclude `blog` and `blog/*` from root uploads, including deletion, while blog
uploads remain scoped to `s3://www.pax.dev/blog/`. The publisher and production
acceptance are still pending; see the [hosting handoff](../pax-blog/infra/README.md).

The website's source is inspected in the running site, not through the CLI's
bundled `examples/` catalog. After moving or changing reusable examples, refresh
that separate snapshot with `python3 scripts/sync-docs-examples.py`.

Content and visual design history are archived in
[PAX-869](https://linear.app/paxdev/issue/PAX-869).

## Hero lettering

The three Manrope words in `RotatingHeadline` use checked-in Bézier outlines at
weight 800 of the [bundled font](assets/fonts/README.md). `HeroLetter` animates
ordinary Pax paths: high-performance accelerates and brakes, native assembles along
short arcs, and portable presses and rebounds. Native and portable freeze their
painted fields and fade in place on exit, reusing the assembled pixels rather
than resizing shader captures. Creative keeps its EMSLeague handwriting and CMY
mesh treatment, with boundary control points moving up to 26 percentage points
and the center up to 30, while keeping the cells convex. The mesh loops every
two seconds, twice the previous morph speed.
Its stroke width is `max(1.6, font_px * 0.038) + 1.5` logical pixels.
The source drawer includes the path loader, generated geometry, and letter component.

High-performance uses a cool silver gradient and a moving GPU light scoped by
`LightFrame`. Its sweep begins during the reveal and lasts 1.04 seconds.
Portable uses `ImpressionistMode::Gauguin` on a field of 18 white star
trails sweeping diagonally upward. Staggered speeds, lengths, and thicknesses
create bright heads and long translucent tails; overlapping trails accumulate
alpha over a 33%-alpha white ground. The smaller brush scale preserves the
streaks, while the outer mask keeps each letter crisp.

Native uses the built-in `ImpressionistMode::Signac` treatment on six moving
radial-gradient dots (two each of cyan, magenta, and yellow) over a paper-colored
ground. The animated letter paths provide an outer alpha mask, so the filtered
colors move inside crisp letter contours. The shader is the Mask's content;
the letter group is its separate source. Brush scale follows the font size.
Both native and portable use this crisp outer letter mask. One phase drives
their paint and source motion while either word enters or holds; it stops for
the exit fade. The native selection Text stays outside both the shader and mask.

The website enables `pax-kit/shaders`, including the generic effect runtime and
WGSL validator in Wasm. This is separate from the offline font tracing; no
runtime font outlining dependency has returned. These effects require WGPU and
use one color-field capture per word, rather than one per letter. Native's radial
centers move inside fixed rectangles; portable's gradient endpoints travel
through fixed diagonal strips. Each trail clears the strip before its phase
wraps. This keeps capture dimensions stable while their source pixels change.
Letter outlines remain pretraced path geometry.

Hero timings are 80% of the original durations: 3.36-second holds, 0.72-second
handoffs, 1.28-second creative writing, and proportionally shorter letter
entrances/staggers and logo reveal. Creative's mesh uses the faster two-second
period described above. The word's clipping Frame has
`layout_role=LayoutRole::Breakout`: animated descendant bounds cannot change
the ancestor Stacker's measured height. The fixed three-line signboard and its
normal Text nodes determine the paragraph's position.

The private offline extractor in `tools/trace-hero` uses HarfRust + Skrifa to
shape each complete phrase before splitting it into clusters. It is a standalone
tool, outside the library workspace and the website's dependency graph (including
tests). Ordinary website builds run no tracing step and embed no font bytes.

From the monorepo root, explicitly regenerate after changing the font, weight,
or the phrase list in `tools/trace-hero/src/main.rs`:

```sh
cargo run --manifest-path pax-website/tools/trace-hero/Cargo.toml
cargo run --manifest-path pax-website/tools/trace-hero/Cargo.toml -- --check
cargo test --manifest-path pax-website/tools/trace-hero/Cargo.toml
```

Commit `src/hero_outlines_data.rs` with the source change. It records the font
checksum, weight, and shaping versions, shares 17 unique cluster outlines
across the three phrases, and retains source ranges, glyph IDs, advances and
baseline metrics. `--check` compares the entire generated source, including every
coordinate and cluster placement, against fresh extraction. No manual curve
fitting or raster tracing is involved. The existing `build.rs` still only
generates the AI primer.

This is fixed, single-line Latin LTR copy, with no runtime wrapping, bidi layout,
font fallback or arbitrary text API. `hero_outlines::hero_runs()` expands the
baked data into the same `HeroCluster`/`PathElement` values once at mount.
Coordinates stay in em units; resizing changes bounds, and playback changes
scalar transforms without reshaping or replacing path elements.

Reusable runtime text and `TextGpu` API design are deferred to
[PAX-1036](https://linear.app/paxdev/issue/PAX-1036/design-textgpu-api-with-zack-and-resume-runtime-text-outlining),
starting with Zack-led API design. The full unpublished `pax-text` experiment is
preserved at `79a522edbc29614cf39aae70d30fba680f3c4b86`; it is not part of the launch
library. Neither a `TextGpu` component nor a `Text` rendering flag is implemented.

At that snapshot, the optimized web build measured 1,758,864 bytes with gzip
level 9, versus 2,208,106 bytes for the runtime-shaped predecessor: about 439 KiB
recovered.
Brotli level 11 recovered about 329 KiB. These are whole-Wasm measurements,
including the source drawer, and exclude the separate native font asset; they
do not establish the production server's compression settings.

Each vector word has one clipped native Text counterpart with transparent ink and
the same packaged font. Its selection rectangles follow the resting layout, not
the animated letters. Select during the reading hold for the closest alignment; copying
uses the actual phrase. This is approximate selection, not a complete accessibility
implementation or a public replacement for `Text`. The font remains a separate
asset for native text and selection; it is no longer embedded in the cartridge
for shaping.
Creative also has a clipped, transparent-ink Manrope counterpart outside its
handwriting mask. Its selection is approximate because the visible strokes use
EMS League. Keep the Text node itself opaque so the native selection highlight
remains visible; hide only its ink.
All website Manrope styles use the same `assets/fonts/Manrope.ttf` source and
actual family name. Web loads the file by URL; Apple registers the bundled file
at startup, without downloading another Manrope face. The existing Apple
font resolver selects its ExtraBold named instance (weight 800).
The selection box is clipped to enable the existing iOS/iPadOS text-view path;
the independently animated glyphs retain their original clipping behavior.

Headline motion stops while offscreen. The website has no pause button or
automatic OS reduced-motion handling yet. The logo plays once on mount and
replays when clicked or tapped: `AnimatedPaxLogo.click_to_replay` already defaults
to true. Its `progress=bind:logo_progress` shares one playhead with the website's
initial 1440ms reveal; there is no competing per-frame progress writer.

The manifest pins the website's Pest parser family to 2.8.7. A fresh resolution
to Pest 2.9.3 introduced `stacker`/`psm` into the development Wasm and left an
unresolved `env.rust_psm_stack_pointer` import, preventing browser startup. Keep
the pin until that upstream Wasm path has been verified with Pax.

The website owns its `assets/` directory. Paxflix media entries remain symlinks
to the example assets, so adding website fonts does not modify that example.

Apple builds also pin `libc` to 0.2.189: `backtrace` 0.3.76 still imports dyld
bindings that 0.2.190 restricts to macOS, breaking iOS simulator compilation.
Remove the pin after verifying the upstream combination on Apple mobile targets.

The site shell uses `DynamicIslandSpacer` insets on Apple mobile targets so the
header and scrolling content avoid the status bar, cutout, and home indicator,
while the background remains edge to edge.

The runtime-shaped predecessor was checked in debug builds on an Apple Silicon
Mac and in the iPhone 17 Pro and iPad Pro 11-inch simulators, including iPad rotation.
The paused native word can be selected and copied on macOS; iPhone selection
shows the platform's handles and Copy menu. These checks do not cover physical
devices or optimized Apple apps. The full macOS release build was stopped during
lengthy LLVM optimization of the application crate, which includes this baseline's
large generated Rust manifest. That release runtime check remains open. The
pre-traced replacement passes exact geometry/placement comparisons, an optimized
web build, and desktop/mobile browser checks including paused selection and
copying. The updated Apple application has not been rerun.
