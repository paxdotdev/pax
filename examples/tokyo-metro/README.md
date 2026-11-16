# Tokyo — a moving atlas

A portable Pax transit demo covering all nine Tokyo Metro lines, four Toei
subway lines, and JR Yamanote. The overview is an **octilinear schematic**:
horizontal, vertical and 45° route segments deliberately distort geography
to make the network readable. Thick, cased routes, rounded station-code badges
and grouped interchanges follow the visual language of a printed transit map.
It is drawn with Paths, not a map bitmap.

All 321 line-station entries are present. Every train is **simulated**.
This is an interactive graphics example, not a journey planner or a source
of service information.

## Run

From the repository root, using a CLI built from this checkout:

```sh
cargo run -p pax-cli -- run --path examples/tokyo-metro --target web
cargo run -p tokyo-metro
```

The second command runs directly on macOS. For template hot reload or mobile
packaging, use the CLI:

```sh
pax-cli run --path examples/tokyo-metro --target macos
pax-cli run --path examples/tokyo-metro --target ios --ios-device simulator
pax-cli run --path examples/tokyo-metro --target ipados --ios-device simulator
pax-cli build --path examples/tokyo-metro --target web --release
```

See the repository's [target prerequisites](../../pax-docs/book/src/targets-build-deploy.md).
Rust changes require rebuilding and refreshing the browser; `.pax` templates
reload in a debug CLI session. No account, credentials or network data service
is required. The example is not in the CLI starter bundle or website gallery.

## Explore

- Select a colored route, use the line dropdown, or choose a line in the desktop
  legend. Tapping ambiguous overlapping routes opens a chooser.
- Open lines straighten; Yamanote becomes a long closed pill. Oedo preserves
  its loop and attached tail. Marunouchi retains the Honancho branch.
- Portrait layouts run vertically. Scroll the canvas to explore long routes;
  **Fit** shows the entire route and **+ / −** change its scale.
- **Find station** accepts English names or station codes and centers the
  matching station in a line view. **← Main map** is the leading navigation
  control in every line view, including phone and tablet widths. The line
  dropdown's first option also returns to the network.
  Escape dismisses an open panel before returning to the network.
- On the web, line selection updates the URL (for example,
  `/line/yamanote`). Browser **Back** and **Forward** restore the corresponding
  view. The explicit **← Main map** control always goes to `/`, even when the
  previous history entry is another line or an external page. Direct line
  visits and reloads work when the host serves the app entry for route paths;
  `pax-cli run` provides this fallback. Unknown paths or line slugs show the
  network. For static hosting, configure the
  [application-route fallback](../../pax-docs/book/src/targets-build-deploy.md#direct-links-and-the-base-url).
- Tap a train or use **Inspect a train…** to see its synthetic identity,
  direction, current segment and next scheduled stop. The panel is placed at
  the lower edge on phones and at the lower right on larger screens.
- **Pause** freezes simulation time. Desktop **1× / 6×** changes its rate.
  **Motion off** also freezes trains and skips morph animations.
  This is an explicit application setting, not automatic OS preference detection.

The native dropdowns and buttons provide keyboard alternatives to canvas
picking. Station names are English; line titles also include Japanese. This
example does not claim a complete screen-reader navigation model for the map.

## Topology, geometry and simulation

`network.rs` holds station identities, line memberships, track edges,
pedestrian transfers, verified shared tracks and out-of-scope continuations.
A crossing is not automatically a transfer. Namboku/Mita share the
Meguro–Shirokane-takanawa tracks; Yurakucho/Fukutoshin share the northern
Wakoshi–Kotake-mukaihara corridor. Their display lanes remain separate.

`map_data.rs` supplies authored schematic anchors. Intermediate stations are
distributed along octilinear corridors. `geometry.rs` represents every edge
with four cubic pieces in both the overview and isolated layouts. The same
curves are rendered, hit-tested and sampled by an arc-length lookup table.
`cartography.rs` places one station name per interchange, prioritizes major
hubs, and reserves space around the line-specific letter/number badges. Names
and badges scale with the overview; at phone overview scale only major names
remain, with local names and codes revealed by zoom. The line view uses larger,
screen-sized labels. The northern shared corridors use separate parallel lanes.

Morphing interpolates the numeric control points; it does not rely on
`PathElement` property interpolation. Repeated station visits retain one
geometric junction, and loops have no opened seam.

`motion.rs` keeps camera zoom and layout morphs on independent clocks. Zooming
during a morph does not restart it or rebuild curve samples. Layout changes
retarget from the displayed geometry and update only affected lines. The
station, train and path components in `layers.rs` publish their own changed
data; immutable line metadata stays separate from animation state. Each line
layer owns its fade, rather than publishing opacity on every station or train.
Paused simulation observations and unchanged label layouts are retained.
These are application-side changes; rendering still uses the engine's normal
vector and native-text paths without changing raster resolution.

`navigation.rs` connects the declarative Router branches in `lib.pax` to the
atlas selection. The scene stays mounted outside those branches so navigation
retains the simulation clock, map camera and current morph pose. Incoming
history changes are distinguished from UI selections to avoid pushing another
history entry when handling Back or Forward. The same explicit destination
navigation uses Pax's in-app location on native targets.

`simulation.rs` owns a small `TrainProvider` contract with topology-relative
observations. The deterministic simulator takes an injected clock, precomputes
service legs and uses stable arithmetic offsets. It has no platform APIs or
pixel coordinates. A future data adapter can translate external station and
line identifiers at this boundary without changing the geometry model.

Synthetic services include locals, the Marunouchi branch, Tozai rapid and
Fukutoshin express stopping patterns. Express trains traverse every physical
edge but omit dwell at skipped stops. Yamanote circulates in both directions.
Oedo travels between Hikarigaoka and Tochomae via the loop, then reverses;
it is not modeled as endless circulation. Other simulated services turn around
at their modeled termini. Continuation labels identify connected railways
beyond the demo; trains on those external tracks are not simulated.

Train IDs, dwell times, travel times, headways and dispatch are invented.
The demo does not reproduce timetables, all short turns, every through-service,
disruptions or rolling-stock rules. There is no live mode or speculative live
networking infrastructure. Suspended frames do not advance the simulation by
the entire time spent in the background.

## Data provenance

Station names, codes and ordering were checked against the operator sources
on 15 November 2026. Future extensions are not included before opening.

- [Tokyo Metro station information](https://www.tokyometro.jp/lang_en/station/index.html)
  and the line-specific station tables for
  [Ginza](https://www.tokyometro.jp/lang_en/station/line_ginza/index.html),
  [Marunouchi](https://www.tokyometro.jp/lang_en/station/line_marunouchi/index.html),
  [Hibiya](https://www.tokyometro.jp/lang_en/station/line_hibiya/index.html),
  [Tozai](https://www.tokyometro.jp/lang_en/station/line_tozai/index.html),
  [Chiyoda](https://www.tokyometro.jp/lang_en/station/line_chiyoda/index.html),
  [Yurakucho](https://www.tokyometro.jp/lang_en/station/line_yurakucho/index.html),
  [Hanzomon](https://www.tokyometro.jp/lang_en/station/line_hanzomon/index.html),
  [Namboku](https://www.tokyometro.jp/lang_en/station/line_namboku/index.html), and
  [Fukutoshin](https://www.tokyometro.jp/lang_en/station/line_fukutoshin/index.html).
- [Toei station lists](https://www.kotsu.metro.tokyo.jp/eng/subway_stations/)
  and [Oedo service explanation](https://www.kotsu.metro.tokyo.jp/enq/customer/pdf/customer-april_31.pdf).
- [JR East metropolitan route maps](https://www.jreast.co.jp/multi/downloads/index.html)
  and [Yamanote overview](https://www.jreast.co.jp/en/tokyomovinground/english/about/).
- [Namboku/Mita shared section](https://www.tokyometro.jp/lang_en/ticket/particularcase/meguro_shiroganetakanawa/index.html).

Operator map artwork is not bundled. The paths, layout and interface are
authored for this demo; operator names and colors identify the underlying
network. No operator endorsement or open license for official map artwork
is implied.

## Validation

```sh
cargo test -p tokyo-metro --lib --no-default-features
```

Tests cover station/code counts, real service edges, transfer/shared-track
identity, branch and loop closure, terminal turnarounds, dwell and express
semantics, station/train attachment throughout both morph orientations,
interrupted selection, independent zoom timing and reduced-motion endpoints.
Navigation tests also cover all 14 stable line URLs and history updates without
duplicate navigation entries.
Browser layouts are developed at phone, landscape phone, tablet and desktop
sizes. The schematic styling is authored in `src/route_*.pax`; label placement is
in `src/cartography.rs`, with route anchors in `src/map_data.rs`. Native target qualification is separate from a successful web build.

The initial qualification on 15 November 2026 passed all eight model/geometry
tests, debug and baked-release web builds, and browser checks at 360×780,
844×390, 768×1024 and 1440×900. Direct route/train picking, station search,
branch/loop views, return navigation and motion controls were exercised.
The station-badge refinement was checked at those same four browser sizes.
The application performance pass passed 11 tests and both web build modes;
browser checks cover zoom during morphs, interrupted selection, paused return,
station search, train inspection and orientation changes. Residual zoom and
transition stalls are tracked in the
[performance finding](https://linear.app/paxdev/issue/PAX-1063#comment-34d9423c-a0c6-46ae-9925-5eaee7474796).
The navigation follow-up passes all 13 tests and both web builds, with browser
Back/Forward, direct line visits, reload and the explicit return control checked
at phone, tablet and desktop widths.
A faint background extent artifact can remain after viewport resizing; reload
clears it ([finding](https://linear.app/paxdev/issue/PAX-1063#comment-f745af8f-e995-46b2-9b5d-0db679820847)).

macOS and iOS Simulator debug packages build and render the overview. Native
qualification remains incomplete: macOS stalled during line selection in Metal
command-buffer allocation ([finding](https://linear.app/paxdev/issue/PAX-1063#comment-8045b479-4146-47b3-bac0-c8bb56000bc4));
iPhone 17 Pro / iOS 26.4 showed the safe-area layout and moving trains, but
interaction verification was inconclusive after accessibility timeouts.
iPadOS and physical devices have not been qualified. This checkout does not
provide native Android, Windows or Linux targets. No native renderer workaround
is included in the example.
