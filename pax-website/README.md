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

The hero imports `AnimatedPaxLogo` directly from `examples/src/pax-logo/` and
includes the original component sources for View Source. Living Quilt remains
independently runnable in `examples/src/living-quilt/`, but is no longer mounted
or a website dependency. No duplicate implementation or standard-library
migration is needed. Cargo dependency paths
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

The three Manrope words in `RotatingHeadline` are shaped at mount time by
HarfRust and outlined by Skrifa, at weight 800 of the [bundled font](assets/fonts/README.md).
`HeroLetter` animates ordinary Pax paths: high-performance accelerates and brakes,
native presses and rebounds, and portable assembles along short arcs. Creative
keeps its EMSLeague handwriting and CMY mesh treatment. The source drawer includes
the shaping helper and letter component.

This is a website-local, single-line Latin LTR implementation. It preserves whole-run
kerning, advances, and shaping clusters (including ligatures and combining marks).
It has no wrapping, bidi layout, font fallback or arbitrary font-loading API.
Coordinates stay in em units; resizing changes bounds, and playback changes scalar
transforms without reshaping or replacing path elements. All three runs share an
outline cache during prewarming; their data lives with the mounted headline.
Unsupported glyphs or text outside this scope are rejected; if prewarming fails,
the headline keeps its native Text rendering. Accent support depends on the
font's glyph coverage, not just on the input being Latin.

Each vector word has one clipped native Text counterpart with transparent ink and
the same packaged font. Its selection rectangles follow the resting layout, not
the animated letters. Pause before selecting for the closest alignment; copying
uses the actual phrase. This is approximate selection, not a complete accessibility
implementation or a public replacement for `Text`. The font bytes are embedded
in the cartridge and also packaged for native text (a deliberate duplicate).
All website Manrope styles use the same `assets/fonts/Manrope.ttf` source and
actual family name. Web loads the file by URL; Apple registers the bundled file
at startup, without downloading another Manrope face. The existing Apple
font resolver selects its ExtraBold named instance (weight 800).
The selection box is clipped to enable the existing iOS/iPadOS text-view path;
the independently animated glyphs retain their original clipping behavior.

The existing Pause hero motion button resolves lettering to a legible resting
pose; resuming does not replay the entrance. Offscreen time does not advance.
OS reduced-motion preferences are not queried automatically.

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

Apple parity has been checked in debug builds on an Apple Silicon Mac and in
the iPhone 17 Pro and iPad Pro 11-inch simulators, including iPad rotation.
The paused native word can be selected and copied on macOS; iPhone selection
shows the platform's handles and Copy menu. These checks do not cover physical
devices or optimized Apple apps. The full macOS release build was stopped during
lengthy LLVM optimization of the application crate, which includes this baseline's
large generated Rust manifest. That release runtime check remains open.
