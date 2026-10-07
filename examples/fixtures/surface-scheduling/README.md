# Web surface scheduling probe (PAX-1014)

Switch between one or two independent vertical catalogs containing horizontal
shelves. Each has direct and 0.5× transformed modes: scaled modes double the
local geometry so the presented content stays identical. There are no
images, timers, effects, or application-level lazy mounts: each tile must show a
gray panel and white circle immediately after its renderer prepares it. This
isolates canvas preparation from Paxflix's scene mounting and asset decoding.

Build using a CLI compiled in this worktree:

```sh
cargo build -p pax-cli
source ~/.zshrc
nvm use
target/debug/pax-cli run --path examples/fixtures/surface-scheduling --target web --libdev
```

Exercise both catalogs independently and simultaneously, reverse direction, and
scroll a shelf sideways. Test continuous one-viewport travel over about 400 ms,
then abrupt one-page jumps and return visits. Inspect at desktop/390px and DPR
1, fractional DPR, and 2. A narrow Chromium viewport is not an iOS/WebKit test.

The mount's `data-surface-*` attributes report demand, admission, explicit visible
overload, and estimated backing bytes (not total GPU memory). Canvas
`data-expected-backing-*`, `data-replay-priority`, and
`data-surface-generation` expose the allocation contract. The cap stays 64.

At 1280×720 and DPR 2, the initial 2026-10-05 comparison was:

| Mode | Before projected density | After projected density |
| --- | ---: | ---: |
| One direct | 26.91M pixels / 26 surfaces | 26.91M / 26 |
| One scaled | 96.58M / 44 | 26.91M / 26 |
| Two direct | 48.97M / 38 | 48.97M / 38 |
| Two scaled | 124.45M / 50 | 48.97M / 38 |

These are settled admitted color-backing estimates at the top, including warm
surfaces, not frame timings or total GPU memory. Wait for density shrink to
settle after switching modes. The subsequent recursive-coverage checkpoint reduced
one direct catalog to 7.46M pixels / 5 surfaces and two scaled catalogs to
11.23M / 8 in the same settled configuration. Physical-pixel admission includes
initializing and retiring reservations; it is not a frame-time budget.

Append `?pax_surface_trace=1&pax_scroll_perf=1` to expose the **Surface trace
snapshot** inspector. Select a visible vertical Scroller and use **Probe down**
or **Probe up** for a reproducible one-viewport motion at 200, 400, or 1000 ms.
The input is programmatic native DOM scrolling, not physical touch or a capture
of every compositor-displayed frame. Inspect both cold visits and return visits:
the WGPU drawing cache can reuse recorded content after tile eviction, but cannot
avoid the first recording of previously unrendered content.
