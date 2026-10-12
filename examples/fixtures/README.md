# Fixtures and supporting examples

These packages preserve focused learning material, platform integration probes,
regression cases, and historical experiments. For the main starting points, see
[the curated examples](../README.md). “Fixture” describes their place in this
collection; many are useful teaching examples.

Run from the repository root:

```sh
pax-cli run --path examples/fixtures/auto-sized-containers --target web
cargo run -p auto-sized-containers           # native macOS
cargo run --example auto-sized-containers   # native macOS
pax-cli docs examples auto-sized-containers
```

The move from `examples/<name>` to `examples/fixtures/<name>` changes filesystem
paths and CLI `--path` arguments. Cargo package names, Cargo example names, CLI
docs short names, and hosted example URLs stay the same. The docs command also
accepts `fixtures/<name>` and `examples/fixtures/<name>`.

## Teaching studies

| Example | Purpose |
| --- | --- |
| [adaptive-cards](adaptive-cards) | Target/viewport responsive composition study |
| [auto-sized-containers](auto-sized-containers) | Intrinsic sizing and nested Scroller/Stacker teaching example |
| [breakout](breakout) | Stateful game and collision simulation |
| [color-picker](color-picker) | Reactive color channels and palette interaction |
| [example-host](example-host) | Projected content and source drawer teaching example |
| [fireworks](fireworks) | Scroll-driven procedural motion; dependency of legacy examples |
| [game-of-life](game-of-life) | Editable grid and Rust simulation state |
| [glow-buttons](glow-buttons) | Scoped lighting regression/study |
| [marionette](marionette) | Jointed puppet and nested declarative timelines |
| [mesh-gradients](mesh-gradients) | Gradient control points and reactive binding study |
| [mouse-animation](mouse-animation) | Pointer-driven transform teaching example |
| [particles](particles) | Repeated animated content/performance example |
| [path-drawing](path-drawing) | SVG import, path stroking and drawing studies |
| [pick-up-sticks](pick-up-sticks) | Interactive game with vector animation |
| [route-card](route-card) | Route transitions and modal/card integration example |
| [router-playground](router-playground) | Nested routing and navigation teaching example |
| [runtime-settings-themes](runtime-settings-themes) | Dynamic theme providers and transitions teaching example |
| [scroll-garden](scroll-garden) | Scroll-driven kinetic composition study |
| [scroll-swatch](scroll-swatch) | Nested two-axis scroll interaction study |
| [shader-studio](shader-studio) | Interactive Gaussian blur and impressionist shader study |
| [space-game](space-game) | Sprites, keyboard and touch controls teaching example |
| [timeline-playground](timeline-playground) | Declarative timeline units and hierarchy study |
| [transition-grid](transition-grid) | Transitions, exit handling and reflow teaching example |

## Architecture, platform and regression fixtures

| Example | Purpose |
| --- | --- |
| [architecture-diagram](architecture-diagram) | Engine architecture reference and diagram export tool |
| [async-workbench](async-workbench) | Async ownership, cancellation, startup and embedding probe |
| [group-compositing](group-compositing) | Mixed native/canvas group opacity acceptance fixture |
| [gyro-helper](gyro-helper) | Device sensor integration probe |
| [mask-strokes](mask-strokes) | Paint layers and live component masks regression fixture |
| [native-text-compatibility](native-text-compatibility) | macOS text/control/accessibility regression fixture |
| [neon-opacity](neon-opacity) | Opacity, masks and compositing study |
| [occlusion](occlusion) | Native/canvas occlusion and masking acceptance fixture |
| [photo-picker](photo-picker) | Library/camera and platform permission integration probe |
| [responsive-helpers](responsive-helpers) | Target, viewport, motion and time globals probe |
| [rounded-scroller-tiles](rounded-scroller-tiles) | Rounded native/canvas clipping regression fixture |
| [scroll-cull-stress](scroll-cull-stress) | Heavy scene/culling performance probe |
| [scroll-matrix](scroll-matrix) | Nested/axis-locked/transformed Scroller correctness matrix |
| [slot-particles](slot-particles) | Slot expansion stress case; known GPU transform-limit follow-up |
| [slot-projection-resolver](slot-projection-resolver) | Slot projection/remainder/identity correctness fixture |
| [store-ownership](store-ownership) | Owner scopes and shared state integration fixture |

## Historical experiments

| Example | Purpose |
| --- | --- |
| [dynamic-linking-lab](dynamic-linking-lab) | Historical Rust reload experiment; label unsupported reload controls clearly |
| [starter-project](starter-project) | Legacy composite starter; preserve without suggesting current default |

## Support notes

- Liquid Glass, Materials, and Viewport Proximity remain in the curated list.
- Slot Particles has a known WebGPU scene-transform limit; renderer work is tracked separately.
- Dynamic Linking Lab preserves the old editor experiment. Rust logic replacement
  is unsupported; Rust changes require rebuilding and restarting. Template reload
  remains available through debug CLI sessions.
- Starter Project is a legacy composite, not the project produced by `pax-cli create`.
- Native macOS/iOS fixtures require an Apple workstation and the appropriate SDK;
  some platform controls have different behavior or fallback rendering on web.

For new supporting examples, use `examples/create-example.sh <name>` from the
repository root. Keep this index, Cargo registration, and the docs source snapshot
in sync. `scripts/clean-temp-files.sh` removes ignored `target` and `.pax` build
artifacts at both example levels; use `--dry-run` to inspect its scope.
