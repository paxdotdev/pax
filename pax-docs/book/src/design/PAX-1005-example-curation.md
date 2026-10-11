# PAX-1005 — Example curation and migration proposal

Status: **implemented, validated, and cleaned; ready for review.**
Prepared 2026-10-10 in the PAX-1015 checkout.
Updated 2026-10-11 to retain Liquid Glass, Materials and Viewport Proximity at the top level.

Issue: [PAX-1005](https://linear.app/paxdev/issue/PAX-1005/curate-top-level-examples-separate-fixtures-and-scope-directory).

## Approved layout

Keep nine first stops at `examples/<name>`: **Calculator, Paxflix, Living Quilt,
Ink & Light, Increment, Pax Logo, Liquid Glass, Materials, and Viewport Proximity**.
Present the first three as app showcases, Liquid Glass and Materials as rendering
showcases, Viewport Proximity as a viewport interaction study, Ink & Light as a
hands-on example, and Increment/Pax Logo as small learning examples.
Put the other 40 packages under `examples/fixtures/<name>`, with an
index grouped into teaching studies, platform/regression probes, and legacy
experiments. “Fixture” is a location/presentation distinction, not a claim that
all secondary projects are tests or unsuitable for learning.

The original extra `examples/src` directory was already removed in PAX-1015.
This proposal finishes the hierarchy in one migration; it does not flatten each
ordinary Rust package’s own `src` directory. No package, asset or runnable name
is deleted. The CLI create set remains Living Quilt (default), Ink & Light and
Increment. Showcase promotion does not add assets to the installed CLI.

Zack approved the nine-example roster and physical move on 2026-10-11, including
updating imports, bundles and other references. This classification is based on
source, existing docs and prior task validation; it is not a fresh visual
endorsement of all 49 examples.

## Presentation and command compatibility

- Add `examples/README.md` with the nine first stops, a short description of each,
  CLI web commands, optional macOS Cargo commands and a link to the fixture index.
- Add `examples/fixtures/README.md` grouping all secondary projects by purpose.
  Keep architecture/reference and async integration material easy to find.
- Keep package names and `cargo run -p <name>` unchanged. Keep all existing
  `cargo run --example <name>` names. Move secondary `.rs` wrappers alongside
  the fixture directories (`examples/fixtures/<name>.rs`) and register explicit
  `[[example]]` paths at the root, avoiding 40 secondary wrappers at the top.
- Use explicit top-level workspace members plus `examples/fixtures/*`; do not
  let the `fixtures` container be mistaken for a Cargo package.
- CLI `--path` commands for moved examples change. Publish the old→new table and
  update current instructions. Do not retain duplicate sources or symlinks.
- Preserve CLI docs lookup by short name, accept canonical fixture paths, and
  distinguish first stops from fixtures in listings. Keep all 49 source entries.
- Preserve hosted example identities by short name while resolving source paths
  through their current category. Validate generated links; publishing any
  updated hosted docs remains a separately authorized operation.

## Complete path map

Every row starts at `examples/<name>`. “Keep” means that exact directory remains;
“Move” means `examples/fixtures/<name>`. Each moved wrapper follows the same
category. All packages are root workspace/Cargo-example consumers; the last
column lists additional notable consumers. Generated snapshots are regenerated,
not hand-edited. Historical design records are not globally rewritten.

| Name | Destination | Purpose | Additional consumers |
| --- | --- | --- | --- |
| `adaptive-cards` | Move: `examples/fixtures/adaptive-cards` | Target/viewport responsive composition study | Docs source discovery |
| `architecture-diagram` | Move: `examples/fixtures/architecture-diagram` | Engine architecture reference and diagram export tool | AGENTS/CLAUDE architecture reading path; Pax Logo dependency; raster-export workflow |
| `async-workbench` | Move: `examples/fixtures/async-workbench` | Async ownership, cancellation, startup and embedding probe | Events/Rust docs; host embedding and Tokio fixtures |
| `auto-sized-containers` | Move: `examples/fixtures/auto-sized-containers` | Intrinsic sizing and nested Scroller/Stacker teaching example | pax-docs/book/src/layout-responsiveness.md |
| `breakout` | Move: `examples/fixtures/breakout` | Stateful game and collision simulation | Legacy starter dependency |
| `calculator` | Keep: `examples/calculator` | Flagship graphing calculator; native input and two-axis scrolling | pax-docs/book/src/layout-responsiveness.md; pax-docs/book/src/scrolling-viewports.md; tests/src/host-embedding/README.md; tests/src/tokio-support-probe/README.md |
| `color-picker` | Move: `examples/fixtures/color-picker` | Reactive color channels and palette interaction | Legacy starter dependency |
| `dynamic-linking-lab` | Move: `examples/fixtures/dynamic-linking-lab` | Historical Rust reload experiment; label unsupported reload controls clearly | Design-server tests; current on-screen reload claims need correction |
| `example-host` | Move: `examples/fixtures/example-host` | Projected content and source drawer teaching example | Docs source discovery |
| `fireworks` | Move: `examples/fixtures/fireworks` | Scroll-driven procedural motion; dependency of legacy examples | Legacy starter and Slot Particles dependencies; CONTRIBUTING; initial designtime manifest |
| `game-of-life` | Move: `examples/fixtures/game-of-life` | Editable grid and Rust simulation state | Docs source discovery |
| `glow-buttons` | Move: `examples/fixtures/glow-buttons` | Scoped lighting regression/study | Docs source discovery |
| `group-compositing` | Move: `examples/fixtures/group-compositing` | Mixed native/canvas group opacity acceptance fixture | pax-docs/book/src/compositing-effects.md |
| `gyro-helper` | Move: `examples/fixtures/gyro-helper` | Device sensor integration probe | Docs source discovery |
| `increment` | Keep: `examples/increment` | Smallest complete template-to-Rust interaction | CLI starter bundle; compiler/codegen/IR/binary tests; CI build/deploy; first-touch docs |
| `ink-and-light` | Keep: `examples/ink-and-light` | Responsive postcard, local assets and Rust interaction | CLI starter bundle |
| `liquid-glass` | Keep: `examples/liquid-glass` | Apple native material and fallback showcase | pax-docs/book/src/compositing-effects.md |
| `living-quilt` | Keep: `examples/living-quilt` | Canonical starter; expressive composition and interaction | Default CLI starter bundle; source-bundle tests; What Is Pax and website references |
| `marionette` | Move: `examples/fixtures/marionette` | Jointed puppet and nested declarative timelines; possible promotion | Docs source discovery |
| `mask-strokes` | Move: `examples/fixtures/mask-strokes` | Paint layers and live component masks regression fixture | Docs source discovery |
| `materials` | Keep: `examples/materials` | Lighting/material composition showcase | pax-docs/book/src/compositing-effects.md; pax-docs/book/src/layout-responsiveness.md |
| `mesh-gradients` | Move: `examples/fixtures/mesh-gradients` | Gradient control points and reactive binding study | pax-docs/book/src/drawing-styling.md |
| `mouse-animation` | Move: `examples/fixtures/mouse-animation` | Pointer-driven transform teaching example | Docs source discovery |
| `native-text-compatibility` | Move: `examples/fixtures/native-text-compatibility` | macOS text/control/accessibility regression fixture | Docs source discovery |
| `neon-opacity` | Move: `examples/fixtures/neon-opacity` | Opacity, masks and compositing study | pax-docs/book/src/compositing-effects.md |
| `occlusion` | Move: `examples/fixtures/occlusion` | Native/canvas occlusion and masking acceptance fixture | pax-docs/book/src/compositing-effects.md |
| `particles` | Move: `examples/fixtures/particles` | Repeated animated content/performance example | Docs source discovery |
| `path-drawing` | Move: `examples/fixtures/path-drawing` | SVG import, path stroking and drawing studies | pax-docs/book/src/drawing-styling.md |
| `pax-logo` | Keep: `examples/pax-logo` | Compact motion example; reusable website/diagram dependency | Website Cargo dependency and include_str links; architecture diagram dependency; build benchmark |
| `paxflix` | Keep: `examples/paxflix` | Flagship app composition, nested scrolling and runtime themes | Animation, themes and scrolling docs; active website integration work |
| `photo-picker` | Move: `examples/fixtures/photo-picker` | Library/camera and platform permission integration probe | pax-docs/book/src/accessibility-native-controls.md |
| `pick-up-sticks` | Move: `examples/fixtures/pick-up-sticks` | Interactive game with vector animation; possible promotion | Docs source discovery |
| `responsive-helpers` | Move: `examples/fixtures/responsive-helpers` | Target, viewport, motion and time globals probe | Docs source discovery |
| `rounded-scroller-tiles` | Move: `examples/fixtures/rounded-scroller-tiles` | Rounded native/canvas clipping regression fixture | pax-docs/book/src/scrolling-viewports.md |
| `route-card` | Move: `examples/fixtures/route-card` | Route transitions and modal/card integration example | Docs source discovery |
| `router-playground` | Move: `examples/fixtures/router-playground` | Nested routing and navigation teaching example | pax-docs/book/src/components-composition.md; pax-docs/book/src/routing.md |
| `runtime-settings-themes` | Move: `examples/fixtures/runtime-settings-themes` | Dynamic theme providers and transitions teaching example | pax-docs/book/src/drawing-styling.md |
| `scroll-cull-stress` | Move: `examples/fixtures/scroll-cull-stress` | Heavy scene/culling performance probe | Docs source discovery |
| `scroll-garden` | Move: `examples/fixtures/scroll-garden` | Scroll-driven kinetic composition study | pax-docs/book/src/scrolling-viewports.md |
| `scroll-matrix` | Move: `examples/fixtures/scroll-matrix` | Nested/axis-locked/transformed Scroller correctness matrix | pax-docs/book/src/scrolling-viewports.md |
| `scroll-swatch` | Move: `examples/fixtures/scroll-swatch` | Nested two-axis scroll interaction study | Docs source discovery |
| `slot-particles` | Move: `examples/fixtures/slot-particles` | Slot expansion stress case; known GPU transform-limit follow-up | Fireworks dependency; renderer limit intentionally outside this task |
| `slot-projection-resolver` | Move: `examples/fixtures/slot-projection-resolver` | Slot projection/remainder/identity correctness fixture | pax-docs/book/src/components-composition.md |
| `space-game` | Move: `examples/fixtures/space-game` | Sprites, keyboard and touch controls teaching example; possible promotion | Legacy starter dependency; Events/Rust and Text/Fonts docs |
| `starter-project` | Move: `examples/fixtures/starter-project` | Legacy composite starter; preserve without suggesting current default | Depends on Fireworks, Color Picker, Breakout and Space Game |
| `store-ownership` | Move: `examples/fixtures/store-ownership` | Owner scopes and shared state integration fixture | pax-docs/book/src/components-composition.md |
| `timeline-playground` | Move: `examples/fixtures/timeline-playground` | Declarative timeline units and hierarchy study | Docs source discovery |
| `transition-grid` | Move: `examples/fixtures/transition-grid` | Transitions, exit handling and reflow teaching example | pax-docs/book/src/animation-motion.md |
| `viewport-proximity` | Keep: `examples/viewport-proximity` | Viewport enter/change/exit, data loading and motion study | pax-docs/book/src/scrolling-viewports.md |

## Consumer audit and migration order

1. **Cargo and source paths:** root workspace members/dev-dependencies/example
   targets; 40 package dependency prefixes; sibling dependencies in the legacy
   starter, Slot Particles and architecture diagram. Internal package-relative
   template/assets paths stay intact. Explicitly verify the website’s Pax Logo
   dependency and source includes even though the recommended roster keeps it.
2. **Starter packaging:** retain the allow-list in
   `examples/bundled-cli-examples.toml`. `scripts/sync-cli-examples.py` currently
   requires `examples/<name>`; if the selected roster moves any starter, permit
   exactly the two supported layouts while retaining containment, uniqueness,
   symlink and deterministic output checks. Regenerate the `.paxbundle` only
   from the registry, and test create outside the checkout with asset hashes.
3. **Docs discovery:** `scripts/sync-docs-examples.py` and
   `pax-docs/src/examples.rs::discover_example_paths` only visit immediate
   children today. Teach both exactly the two layouts, reject duplicate short
   names, and verify all 49 packages remain in the source snapshot/CLI listing.
   Update embed paths, source-bundle tests, generated run commands and hosted
   asset identity handling together.
4. **Docs watch mode:** `pax-cli/src/docs.rs` still tests `examples/src` in
   `should_skip_watch_dir` and `should_watch_file`. Fix this to the actual
   examples root and verify `.pax`/asset edits and `--skip-examples` behavior.
   Its query normalization also contains a duplicated `examples/` prefix; cover
   old short names and canonical fixture paths explicitly.
5. **Harnesses:** update `examples/create-example.sh` to make secondary examples
   the normal contribution location and register their Cargo wrapper; preserve
   explicit showcase selection. Audit compiler/static-analysis/design-server
   fixture paths, `scripts/release.py`, CI caches/build/deploy paths and cleanup
   discovery. Exercise cleanup in dry-run mode only until final branch closeout.
6. **Learning path:** update current book/source/API references and root/example
   READMEs. Follow source comments when regenerating API output. Update the
   AGENTS/CLAUDE architecture reading path if Architecture Diagram moves.
   Label Dynamic Linking Lab’s removed Rust-reload behavior accurately and mark
   Starter Project as a legacy composite rather than the default starter.

## Coordination, effort and validation

This is a medium-risk mechanical migration across Cargo, two generated bundles,
docs discovery/build/watch, CI and test paths. The dangerous failure is silent
fixture omission, not the directory renames themselves. Keep the move and all
consumer updates atomic. Budget one migration pass and one focused validation
pass; no engine or example redesign is needed.

On 2026-10-09, **PAX-1014 - Scroller scheduling spec** and
**PAX-839 - Lazy initialization** were active on website/runtime work. They may
consume example paths even when not editing an example package. Recheck their
working-tree paths immediately before moving files, preserve their content on
integration, and call out the path migration in the handoff. No other task has
been messaged or modified.

Acceptance checks:

- Cargo metadata inventories exactly 49 example packages and 49 runnable example
  names; native `cargo check` covers a top-level and a moved example.
- Both source sync scripts pass `--check`; CLI create/list/source lookup work
  outside the monorepo, with starter assets complete and unchanged eligibility.
- Focused compiler/CLI/docs tests cover affected paths, nested discovery,
  watch mode and short-name lookup; relevant baked/release manifest tests pass.
- Run one curated app and one moved fixture on web from their final paths;
  spot-check native Cargo invocation on macOS. Slot Particles’ known renderer
  limit remains the separate follow-up already requested by Zack.
- `mdbook build`, changed links/anchors, generated source checks and
  `git diff --check` pass. No publication or Git commit is implied.

## Approval and coordination record

On 2026-10-11, Zack approved the physical split with the nine curated examples
above and asked that all imports, bundles, and references follow the move. He
also requested updating and running the cleanup script after validation.

Before moving files, the active worktrees were checked read-only. PAX-1014 still
has Paxflix edits at its older `examples/src/paxflix` path; that example remains
curated here. PAX-839 and PAX-1030 have website/runtime/docs changes. This task
only updates applicable path references and does not copy or overwrite any
other worktree's content. When those branches integrate, preserve their content
at the new canonical paths and regenerate the source snapshot after resolution.

All 374 tracked files under the 40 moved packages/wrappers were preserved. Cargo
metadata still identifies 49 example packages and the same 49 Cargo example
names. All three CLI starters remain curated, with unchanged starter names and assets.
The archive was regenerated after correcting their helper scripts’ stale path
depth from the earlier flattening. The docs snapshot was regenerated for both levels.

## Validation — 2026-10-11

- Cargo metadata: 49 example packages and the same 49 Cargo example target names;
  every local dependency path resolves. All 374 tracked moved files remain.
- Native `cargo check --example increment --example auto-sized-containers` passed,
  compiling all 49 example libraries and their imports. Direct
  `cargo run -p auto-sized-containers` built and launched successfully. The raw
  executable is not exposed by the available native UI tool, so interactive
  verification in this pass was performed on web.
- Both `pax-cli build --path examples/increment --target web` and the corresponding
  `examples/fixtures/auto-sized-containers` build passed. In the browser,
  Increment advanced from 0 to 1; the moved fixture scrolled and its PING button
  advanced from 0 to 1. Small standalone preview copies remain outside the
  checkout so cleanup can remove their build directories.
- CLI unit tests (31), local-network tests (6), packaged creation (1), docs source
  bundle tests (5), hosted-example build/cache tests (6), API generator tests (5), and
  binary manifest/ProgramIR round trips (2) passed. The network tests needed
  normal loopback access after the sandbox denied their local listeners.
- Example-layout/scaffold/cleanup tests (4), docs publication tests with external
  calls mocked (37), embed tests (11), and route-location/metadata tests (9) passed.
  CLI docs listing and short-name lookup were also exercised outside the checkout.
- Both source sync checks, local Markdown link checks, all nine public book embed
  paths/source selections, `mdbook build`, and `git diff --check` passed.
- The actual docs generator built all nine embedded apps as optimized release
  WebAssembly: Architecture Diagram, Living Quilt, Materials, Mesh Gradients,
  Neon Opacity, Router Playground, Space Game, Transition Grid, and Viewport
  Proximity. Every generated manifest has a successful current-source build,
  valid source panels, the correct source-directory run command, and the
  unchanged short hosted URL. Each cartridge exports its selected `init_web`;
  each generated HTML host invokes `Pax.init_web` explicitly.
- All nine rendered inside their book pages without browser console errors.
  Architecture Diagram zoom responded; Space Game rendered its packaged sprites
  and restarted through Play Again; its Rust tab showed `runnable!`. Viewport
  Proximity loaded its public note data inside the iframe. The publisher's
  output validator and local `_pax_docs_build.json` digest check passed. Nothing
  was uploaded and no published version snapshot was changed.
- The embed cache recipe now includes the extracted `pax-codegen`, `pax-host`,
  and web chassis sources, plus Cargo manifests, build scripts, and the lockfile.
  A regression test proves those changes invalidate an otherwise unchanged
  example's cached app. Documentation edits alone still reuse compiled apps.
- The starter archive retains all 41 file names, assets, modes, and starter
  identities; only its three `pax` helpers changed. No runtime serialization or
  public SDK API changed in this migration. Historical design records retain
  their original evidence paths; the map above records the relocation.
- Ran `scripts/clean-temp-files.sh` after validation: the first pass removed
  13 directories (about 16.8 GiB); the docs-validation pass removed nine more
  fixture/curated `.pax` directories (78.6 MiB). Removed this task's dedicated
  50.0 GiB Cargo cache after stopping its remaining development servers, plus
  generated docs copies. The final cleanup dry run found no temp directories.
  Small static app/docs previews remain outside the checkout; other worktrees
  and their caches were not touched.
