# PAX-1014 — Web surface scheduling and preparation

Status: **first pass implemented with remaining motion gaps; phase 2 proposed,
awaiting approval**.
First-pass date: 2026-10-01. Proposal updated: 2026-10-05.
Issue: [PAX-1014](https://linear.app/paxdev/issue/PAX-1014/stabilize-web-scroller-surface-scheduling-and-preparation-contracts).
Base: `65a8a5530`, plus the inherited PAX-869 working changes in the dedicated
`87cf/pax` worktree. Those changes are baseline, not new work for this proposal.

The sections through the implementation checkpoint preserve the first-pass
contract and its validation history. The [phase 2 proposal](#phase-2-proposal--viewport-budgeted-recursive-preparation)
below incorporates the subsequent live findings and Zack's direction on
recursive, directional preparation. Its changes to geometry, density, and
budgeting are **proposed**, not already implemented or approved.

## Recommendation

Keep Rust authoritative for tile geometry and backing metrics. Give the web
host one explicit, testable scheduler for **which candidate surfaces are
admitted and in what order they should be prepared**. Separate scheduling
policy from DOM execution and GPU ownership. Preserve the existing renderer,
retained surfaces, asynchronous leases, and generation checks.

This is a bounded consolidation of the web presentation pipeline, not a new
renderer, scene virtualization system, or public Scroller API. Fix the known
density-contract mismatch first; then establish global visible coverage before
adding interaction-aware preparation.

## Evidence and limits

The inherited source reproduces a concrete mismatch: for a 500×250 logical
tiled scroller at DPR 2, Rust requests a 1000×500 backing, while
`collectCanvasBackingMismatchLayers` expects 500×250. Running the real JS
reconciliation method for three unchanged frames produces three refresh
requests. An undersized 1× backing is incorrectly accepted. This was
independently rechecked in this worktree without source changes.

The parent audit also established:

- Both Paxflix embeds share the app's 64-canvas pool. A sampled tablet scroll
  held 31 tablet canvases, 25 phone canvases, and 6 page/root canvases; 15 of the
  stationary phone's canvases belonged to offscreen rows.
- Host admission bundles required and speculative tiles. A rejected host never
  calls checkout, so allocation-miss counters do not measure admission denial.
- Admission, materialization, initialization, and replay have different
  priority mechanisms. Spatially "active" does not mean receiving input.
- Existing reconciliation tests stub out the backing mismatch checker. All 46
  web tests passed in the parent audit despite the density inconsistency.
- First-visit row mounting also produced roughly 100–170 ms engine ticks in
  prior browser traces. That is a separate contributor, not proven to be fixed
  by surface scheduling.

The live browser measurements above are prior evidence, not new measurements
from this fork. The old preview belongs to the website task and must not be
restarted or modified by this task.

## Required invariants

1. **One density contract.** Planning, allocation, and mismatch detection agree
   on logical size, requested DPR, physical size, rounding, and hard limits.
   A valid unchanged surface produces no refresh request.
2. **Visible before speculative, globally.** If visible demand fits the budget,
   no optional surface may cause required visible coverage to be denied.
   This includes the stationary neighboring demo and the root page.
3. **Admission is not readiness.** A requested or admitted surface is not called
   initialized or painted until the corresponding operation completes.
4. **Reuse requires ownership release.** Neither priority nor budget pressure
   can bypass an outstanding renderer/initializer lease.
5. **Survivors survive.** Unchanged canvases, retained scenes, and image bindings
   stay intact across sibling allocation and priority changes.
6. **Priority is not identity.** Changing priority alone neither replaces a
   surface generation nor cancels otherwise valid initialization.
7. **Scrolling remains native.** No interception, throttling, scroll-position
   rollback, or changes to event/proximity semantics are introduced.
8. **Resource use stays bounded.** Keep the existing canvas cap and screen-DPR
   policy. A hard backing-limit fallback remains explicit and inspectable.

## Ownership and boundaries

| Responsibility | Proposed owner | Boundary |
| --- | --- | --- |
| Candidate tile geometry, stable slot keys, logical extents | Rust tile planner | No DOM-dependent duplicate algorithm |
| Browser/backend limits and expected physical backing metrics | Rust web chassis | Serialized expectations, including policy revision |
| Observed scroll movement, host ancestry, presentation geometry | Web measurement adapter | Read-only snapshot for scheduling |
| Global admission and preparation rank | Pure web scheduler | Snapshot + previous decision + time → decision |
| DOM placement, retirement, checkout, installed-plan publication | Web surface coordinator | Executes the decision; does not reinterpret priority |
| Leases, GPU initialization, retained replay and painting | Existing renderer/chassis adapters | Consume installed surfaces and scheduler rank |

Extract the surface coordinator and scheduler out of `NativeElementPool`;
leave controls, screenshots, and unrelated native-element handling alone.
Suggested module names are `surface-coordinator.ts`, `surface-scheduler.ts`,
and `scroll-activity.ts`; names are illustrative, not additional abstractions
to create if an existing module is the clearer owner.

Keep this policy web-specific initially. Native Apple scheduling retains its
current behavior. Shared Rust hooks must have behavior-preserving defaults.

### Canonical plans and coordinate spaces

The Rust web bridge exports a coherent candidate-plan snapshot after the engine
tick and native patches. It contains layer/host ownership, stable surface keys,
logical regions, expected backing metrics, and a geometry/policy revision.
The ordinary path must not fall back to a second JS tiling algorithm when the
plan is temporarily unavailable: mark it pending, retain valid survivors, and
request the next coherent snapshot. Remove the duplicate fallback after a
call-site audit, or retain only a clearly isolated non-production test adapter.

Use explicitly named/branded values for host-local logical rectangles,
presentation-space rectangles, and physical pixel extents. Convert at the
measurement boundary, not in admission comparisons. Visibility includes the
page viewport and ancestor clipping; non-axis-aligned clips may use conservative
coverage bounds, documented and tested. Do not claim exact occlusion detection.

Preparation is a **future visibility** query, not a broader definition of
currently visible. A row below a clipped phone viewport may become visible
after that phone scrolls. Evaluate its projected region under the bounded
ancestor-scroll window; intersecting its current bounds with the current clip
before that projection would incorrectly prohibit preparation. No change to
public viewport-proximity events or Frame/Mask rendering follows from this.

Expected backing dimensions come from the same Rust metrics helper used by
allocation, including fractional-pixel rounding. JS compares those dimensions
instead of independently choosing a DPR. Newly discovered device limits change
the policy revision and trigger a replan; exceptional clamping reports requested
versus actual metrics rather than causing an endless mismatch-repair loop.

## Admission and priority policy

Operate on **individual surfaces**, not all-or-nothing host costs. Each candidate
has an owner, required dependency set, visibility class, preparation distance,
activity relationship, and stable identity. Empty drawable layers cost zero;
host ancestry alone must not manufacture a canvas requirement.

Selection order:

1. Required root/ancestor surfaces and all currently visible coverage.
2. Preparation ahead of a moving scroller, including descendants exposed by
   that movement. Favor the nearest predicted exposure first.
3. A small retained trailing/reversal buffer for moving branches.
4. Nearest preparation for stationary visible branches.
5. More distant eligible preparation, only while capacity remains.

The candidate region remains bounded by the canonical preparation policy;
interaction priority redistributes that region's budget rather than growing
the candidate set without limit. Start with existing Rust preparation distances.
Do not simultaneously increase overscan to hide initialization cost.

Use one lexicographic rank with named fields, not separately invented integer
scores. Within a class, order by exposure/distance, then reuse of already-ready
surfaces, then stable identity. Fairly interleave equally ranked work from
different branches. Existing warmth is only a tie-breaker, not a reservation.
Propagation into GPU initialization and replay must preserve these meanings.
Scheduling order must not change visual draw/z-order.

The pure result contains selected surfaces, ordered work, retirement requests,
and rejected candidates with reasons. It accounts for canvases still leased
while retiring; those remain unavailable until release. Visible demand blocked
temporarily by a lease is different from demand that exceeds total capacity.

### Genuine overload

If required visible demand alone exceeds the cap, reject all speculative work
and report `visible-demand-exceeds-budget`. Preserve valid visible survivors,
then distribute remaining capacity deterministically across visible branches,
respecting required dependencies. Do not cycle assignments every frame to give
the appearance of fairness, silently raise the cap, or downsample.

This is an explicit degraded condition, not a claim of complete visible
coverage. Record enough information to decide later whether canvas topology or
a separate memory-policy project is needed. Report estimated backing pixels and
bytes, but do not mislabel that estimate as total GPU memory: attachments,
textures, and caches are additional allocations.

## Scroll activity

Use **actual observed offset movement**, in the scroller's local coordinate
space, as the authoritative signal—not mouse hover, DOM focus, or a bubbled
wheel event. This covers wheel, momentum, touch, keyboard, and programmatic
scrolling. Input events may wake measurement but do not independently claim
preparation priority.

- Track activity per scroller and axis. Multiple genuinely moving branches
  may coexist; do not introduce one global "focused scroller" variable.
- A parent moving a child's screen position does not make the child an
  independent mover. The parent's priority propagates to affected descendants.
- Native scroll chaining naturally transfers priority when the parent starts
  changing offset. Do not count every ancestor receiving the same event.
- Apply new direction on the next sampled movement. Smooth velocity for
  distance ordering only; smoothing must not delay a genuine reversal.
- Proposed initial expiry: 250 ms after the last observed movement. Momentum
  renews it. On expiry, return to spatial ordering. Treat this as a tested
  internal parameter, not a new public setting.
- Keep ready survivors as tie-breakers to avoid idle churn. A trailing buffer
  may consume optional capacity, never capacity required by visible content.
- Resize/reparent/clamping discontinuities invalidate the affected geometry;
  they must not masquerade as high-velocity input. Reset velocity after
  suspension, large sampling gaps, and ownership changes.

## Materialization and readiness

Keep orthogonal records for desired/admitted status, physical allocation/lease,
initialization, and painted-content revision. Avoid a giant enum representing
every combination. "Warm" may remain a diagnostic umbrella, but cannot be used
as proof that pixels exist.

Use an explicit two-phase installed-plan transaction:

1. Retire obsolete assignments and notify Rust to drop obsolete renderer
   ownership. In-flight leases remain protected.
2. Allocate available replacements in scheduler order; install final host,
   geometry, expected backing metrics, generation, and priority metadata.
3. Publish the installed snapshot once. It includes pending allocations rather
   than pretending the desired plan was fully materialized.
4. Initialize only installed, valid targets; replay affected coverage and mark
   first submission for the matching surface/content revision.

DOM attributes remain useful for inspection, but do not constitute another
independent scheduling model. Publish typed/versioned snapshots through the
existing chassis seam; avoid per-surface ad hoc interrupts where a batch is
available. Exact bridge method names are an implementation detail.

The installed snapshot replaces normal-path "wait N animation frames for the
DOM to look stable" polling. Do not merely remove safety waits: retain a bounded,
diagnosed fallback for genuinely unresolved native measurement. Waiting for a
host should not occupy one of the four GPU-creation slots. Ready installed work
is ranked when a slot opens; already-started GPU calls are not forcibly aborted.

Validate async completion against surface lease generation, owner, and geometry
identity. A newer priority-only snapshot does not make the result stale. Removed,
rebound, or incompatible geometry results cannot publish into a new assignment.
Do not increase initialization concurrency as part of this work.

Do not conceal a blank by showing pixels from an unrelated old tile. New or
retargeted surfaces need the matching content revision. A first submission is a
useful timestamp, not proof that every asynchronously loaded image has arrived.

## Diagnostics and tests

Add opt-in bounded tracing, available in debug and release web builds, with a
stable surface identity linking:

- demand/admission and denial reason;
- retirement and outstanding leases;
- installed snapshot revision;
- initialization queue/start/completion and stale-result rejection;
- replay request and first submitted content revision;
- first visible time, requested/actual backing metrics, and refresh cause.

Separate scheduler delay, DOM/host readiness, GPU initialization, replay, image
loading, and scene mounting. No unbounded per-frame console dump in normal use.

Required automated scenarios:

- Real Rust metric/plan fixtures consumed by JS reconciliation, without stubbing
  the backing checker: DPR 1, 1.25, 1.5, 2, 3; fractional sizes; hard limits.
- Repeated unchanged frames: no refreshes, allocation changes, or generation
  changes. Priority-only changes update scheduling without replacing surfaces.
- Two visible sibling scrollers: all required tiles fit even when one host's
  full speculative plan would consume the remaining capacity.
- Scaled/nested geometry, fixed clipping ancestors, root scroll, reparent,
  hidden-to-visible transitions, and genuinely impossible demand.
- Direction reversals, two simultaneous movers, momentum expiry, scroll
  chaining, and programmatic movement with a deterministic fake clock.
- Delayed initialization, late stale completions, occupied retiring leases,
  failed creation, detach/remount, and hot-reload teardown. No leaked capacity.
- Both backend adapters consume the same admission result; native scheduling
  retains its previous default behavior.

Browser validation uses an isolated two-scroller fixture first, then the actual
website at desktop and 390px. Run cold and revisited sequences, one viewport of
continuous travel over approximately 400 ms, an abrupt one-page jump, horizontal
travel, and reversal, at DPR 1/2 and fractional DPR where supported. Keep the
cap and density unchanged. Record first-visible-to-first-paint delay and explain
each missing region; a settled screenshot alone is not a pass.

For the lightweight fixture, once the bounded preparation region is ready,
continuous travel within it must not expose scheduler-caused missing coverage.
The real Paxflix result must distinguish that guarantee from late component or
image preparation. Remaining visible gaps are reported as unresolved, not
declared solved because allocation counters are zero. Validate WebGPU and Piet;
physical iOS/WebKit coverage is a separate explicit check, not inferred from a
390px Chromium viewport.

## Staged implementation after approval

1. **Contract repair:** reproduce the DPR failure in a persistent cross-boundary
   test, align expected metrics, and establish idle reconciliation baselines.
2. **Behavior-preserving extraction:** isolate the coordinator and measurement
   snapshot; preserve lease/generation and survivor tests. Remove duplicate
   fallback planning only after accounting for every caller.
3. **Global admission:** split required/optional surfaces, implement the pure
   budget decision and overload reporting, then add per-axis activity ranking.
4. **End-to-end scheduling:** propagate rank through initialization/replay and
   introduce explicit installed-plan readiness. This is the highest-risk stage;
   land it behind deterministic lifecycle tests, not a timing-only browser fix.
5. **Prove and document:** debug/release builds, backend matrix, browser traces,
   and documentation matching the behavior established.

Update the canonical [Scrolling & Viewports](../scrolling-viewports.md) article,
relevant Rust API comments/generated output, and
[authoring pain points](pain-points.md) during implementation. Reconcile the
architecture topology if ownership changes. No new public book chapter is
required. No authoring syntax or manifest fields are proposed; audit host-message
serialization and debug/release paths. If implementation reaches the
compiler/runtime program representation, explicitly audit binary baking too.

## Scope boundaries and approval

Not included: general scene virtualization, Paxflix content changes, a larger
canvas cap, adaptive resolution, a renderer rewrite, demand-driven engine
sleeping, or a full GPU memory manager. Scene-mount profiling remains a distinct
follow-up unless evidence shows a narrowly necessary integration fix.

Recommended approval is for the five stages above, with these defaults: keep
the 64-canvas cap and existing candidate distances; use per-scroller movement
with a 250 ms activity tail; preserve every visible branch ahead of speculative
preparation; make impossible demand observable rather than silently lowering
quality. Zack approved this scope on 2026-10-01.

## Implementation checkpoint — 2026-10-01

### Follow-up after during-motion user repro

The original settled checks missed a presentation gate: admitted prewarm hosts
had `visibility: hidden` until a later measurement classified them visible.
An immediate post-input capture reproduced entirely black Paxflix rows, while
the trace showed several affected canvases had been submitted before exposure.
Keep warm canvas and native-content hosts active/presentable under existing
ancestor clips; cold denied hosts remain parked. Regression tests cover both.
No pool-size, density, clip-boundary, or proximity-event changes are involved.

The trace now has an opt-in DOM snapshot control, change-only visible-unready
events, and (with `pax_scroll_perf=1`) phases over 20ms. Before this change,
first-visit scroll frames measured 95–234ms; after it, immediate captures of
successive 0.8-page tablet moves showed imagery instead of wholly black rows.
Long first-visit ticks remain (roughly 140–155ms in the follow-up sample;
compilation overlapped these later measurements, so they are not a clean
before/after performance benchmark).
This is a presentation fix, not a claim that scene-mount costs are solved.

The calculator comparison exposed a separate application boundary:
`refresh_plot` builds paths for exactly the visible plot rectangle. A sampled
graph scroll exposed empty content on already-submitted surfaces, with a 78ms
frame and no visible-unready transition. Bounded graph draw-ahead is a distinct
example-level follow-up, outside the approved surface scheduler scope. Continue
to report residual motion gaps rather than treating first submission as proof
that application content is ready.

The feature carousel also owns an application-level mounted window (two
overscan cards). A sampled horizontal move showed no visible-unready surface
transition; this does not prove all carousel gaps resolved. A 390 CSS-pixel
phone test showed imagery during first-page exposure with 28 assigned canvases,
no pending allocation, and no scroll-time visible-unready transition. Neither
check substitutes for continuous-frame stress or physical iOS testing.

### What changed

- `surface_metrics.rs` is the single allocation/reconciliation calculation.
  The web bridge exports physical dimensions and a backend-policy revision.
  The independent JavaScript fallback tiler and its 1× assumptions are removed.
- `surface-scheduler.ts` ranks individual surfaces globally; `scroll-activity.ts`
  tracks actual movement and expiry. `surface-geometry.ts` projects local tiles
  through affine transforms. Frame clipping uses conservative path bounds rather
  than assuming a zero-size SVG wrapper is a zero-size clipping region.
- `surface-coordinator.ts` owns retirement, ordered checkout, installed-plan
  publication, and reconciliation. Missing Rust plans preserve survivors until
  the next coherent engine generation. Native-only visible hosts stay visible
  without requiring a GPU allocation.
- The Rust installed-plan registry replaces animation-frame settling polls.
  WebGPU consumes one newly requested surface per initialization batch, using the
  same global order for admission, materialization, initialization, and replay.
  Native whole-layer defaults remain unchanged. Priority is not lease identity.
- Submitted-content revisions distinguish genuinely prepared survivors from
  merely assigned canvases, even when tracing is disabled. Opt-in bounded
  tracing records denied visible demand, initialization queue/start/completion,
  stale/failed initialization, retirement, lease release, and first submission.

The implementation keeps physical lease accounting at the coordinator/pool
boundary instead of predicting asynchronous release in the pure scheduler.
Admission expresses desired coverage under the total cap; the installed snapshot
reports any allocation still pending after retirement. Pool snapshots distinguish
assigned, retiring, and reusable capacity. This preserves required work's rank
without pretending an occupied canvas can be reused. Ancestor dependencies are
zero-cost DOM host activation, not a new generic GPU dependency graph.

### Validation recorded

- 84 web tests pass. The density matrix compiles the real Rust metrics helper
  and feeds its output to real JS reconciliation: DPR 1, 1.25, 1.5, 2, and 3,
  fractional dimensions, hard limits, and repeated unchanged frames.
- Runtime library suite: 240 pass, 2 Metal tests excluded by their normal
  `ignore` markers. Both Metal lifecycle tests also pass when run explicitly,
  including incremental batches, survivors, failed additions, and rebinding.
- Web chassis library suite: 14 pass with one separately identified image-unmount
  test filtered. That test also fails in isolation: it manually unmounts an Image
  and then renders the stale scene entry, reading cleared component properties.
  The image/unmount implementation is outside this change; this is **not** an
  unqualified green chassis suite.
- Debug and release website builds pass. The image-free fixture builds and runs
  with WebGPU and with the Piet feature. The final fixture source uses the normal
  default backend. A wasm32 `piet,designtime` compile check passes.
- API generation, `mdbook build pax-docs/book`, and `git diff --check` pass.
  `tsc --noEmit` still reports 15 existing diagnostics in native patch typing,
  DOM/font typings, event touches, and cartridge declarations; the new modules
  introduce no additional diagnostics.
- Browser observations: image-free fixture at 1280×900/DPR 1; website at
  791×932/DPR 2 and 390×844/DPR 1. A final release check at the browser's
  observed DPR 2.2 also passed (44 assigned canvases after a tablet page scroll,
  no mismatched backing or unsubmitted visible revision, tracing disabled).
  One-page vertical travel, horizontal shelf
  travel, and reversal showed matching visible submitted-content revisions and
  exact expected backing dimensions. Sampled website demand stayed within the
  64-canvas cap (43 assigned desktop, 30 mobile). These are observations, not
  fixed topology counts or performance guarantees.
- In the warmed fixture, newly exposed tiles had already submitted before first
  visibility; sampled initial cold submissions were about 144–158 ms after
  visibility on WebGPU and 37–38 ms on Piet. A first submission is not a promise
  that asynchronous movie artwork has decoded. These are local measurements,
  not a controlled before/after performance benchmark.

### Remaining acceptance work / separate findings

- Continuous approximately 400 ms travel has not been sampled frame-by-frame;
  the completed browser checks use discrete native scroll actions plus per-tile
  timestamps. The full fractional-DPR matrix is covered cross-boundary in tests,
  not by browser overrides. Physical iOS/WebKit and touch/momentum stress remain
  untested here.
- The lifecycle suite covers late completion, failure, occupied retirement,
  removal/remount, and incremental survivor retention. A long-running browser
  soak combining hot reload, forced slow GPU initialization, and exhausted budget
  remains a useful final acceptance check, not an established pass.
- The website's full-screen mobile demo overlay has a separate hit-testing
  obstruction: its backdrop is ahead of the inner Scroller in the browser hit
  stack. The overlay renders, but wheel input lands on the backdrop. No website
  input/compositing workaround is included here; embedded scrolling was tested.
- Keep first-visit component mounting and image decoding as separate profiling
  lanes. This implementation does not virtualize scene graphs, change Paxflix's
  mounting policy, increase the canvas cap, or reduce screen density.

The parent website worktree and server were not changed. No commit, rebase,
merge, or ticket closure is part of this implementation checkpoint.

## Phase 2 — viewport-budgeted recursive preparation

**2026-10-05 — approved by Zack; implementation in progress.**

### Outcome and scope

Make nested scrolling consume resources in proportion to the effective
screen-space coverage and a bounded preparation allowance, rather than the
sum of independently padded local viewports. Spend preparation work first on
content likely to become visible soon, following actual movement recursively.

Preserve independent native scrolling, native/vector interleaving, masks,
z-order, screen-quality rendering, and the first pass's lease/readiness
contracts. This is not a proposal to flatten the page into one canvas.
Transparent overlapping layers, effect intermediates, and tile quantization
have real costs; equal final screen area does not guarantee identical storage.
Those costs must be explicit, not an unexplained multiplier from nesting.

Fixed visible area bounds useful resident coverage; it does not bound the rate
of new content exposure. Faster scrolling requires more preparation per second.
The design therefore needs both a **residency budget** and a **work budget**.
It cannot promise instantaneous readiness after arbitrary cold jumps.

**Shared policy, Web-first rollout:** the reusable planner and scheduler belong
in shared Rust from the start of this phase. Web is the first proving chassis,
not the permanent owner of these algorithms. If the results establish a benefit,
macOS, iOS, and iPadOS should adopt the same policy through their chassis
adapters, with platform-appropriate costs and limits. Native activation is a
later validation gate, not a reason to defer sharing until after a Web-only
implementation has diverged.

Proposed scope expansion beyond phase 1: clip-aware candidate geometry,
transform-aware backing density, pixel-cost admission, and bounded preparation
work. General scene virtualization, a GPU allocator rewrite, arbitrary
interruptible component mounting, and changes to demo content remain outside
this proposal. Profile content preparation separately; request another explicit
decision if meeting the website target requires changing its mounting policy.

### Findings motivating the change

- `pax-runtime/src/engine/layer_tiling.rs` builds each layer's tile window from
  its local viewport, content extent, offsets, DPR, and fixed padding. Effective
  ancestor clips and projected scale do not determine that geometry.
- The web scheduler correctly projects tiles through ancestry for visibility
  and priority, but selects by surface count. Backing pixels are diagnostics,
  not an admission constraint. A small strip and a multi-megapixel tile both
  consume one slot.
- A read-only live snapshot of the current release website showed 68 candidate
  surfaces, 64 admitted, and approximately 107 million admitted backing pixels
  for a roughly 3-million-pixel screen. This includes preparation and overlapping
  layers. It is neither total GPU memory nor a measurement of pixels redrawn per
  frame, and it does not prove the same distribution during every reported gap.
- In that snapshot, a tablet canvas with 1,024 local units of width appeared
  about 596 CSS pixels wide but had a 2,253-pixel backing at browser DPR 2.2:
  approximately three times the backing area needed at its presented scale.
  The allocation path uses browser DPR without the ancestor scale.
- Origin retargeting currently resets a tile's retained scene and requests
  replay. New assignments can initialize a renderer/backend even if the DOM
  canvas is reused. Bounded canvas count does not imply inexpensive turnover.
- Paxflix prepares whole ten-card rows and retains visited rows. Calculator
  plotting generates geometry for its current viewport, and the feature gallery
  has an application-owned mounted window. None of these content-readiness
  boundaries is solved merely by allocating a surface earlier.

Zack's current repro is improved slow/moderate scrolling, but clear first-visit
hitches and blank regions at higher speed. Earlier engine-phase measurements
establish long ticks, not precise attribution to component mounting. Later
timings overlapped compilation. There is not yet a controlled, same-size
standalone-versus-nested benchmark or a continuous-frame proof of readiness.

### Required behavior

1. All currently visible branches precede speculative work. A scrolling phone
   cannot displace the stationary tablet's visible coverage. This guarantee is
   conditional on mandatory demand fitting the physical budgets; overload is
   explicit rather than hidden by reduced density.
2. A child's preparation follows its **anticipated clipped exposure** under
   ancestor movement, not its whole local viewport plus a fresh padding margin.
   Currently empty clip intersection does not prohibit advance preparation.
3. Inherited exposure is not independent scroll intent. Vertical catalog motion
   prepares the horizontal span of upcoming shelves that will actually appear;
   it does not confer large horizontal warm windows on those shelves.
4. Optional residency and queued work remain globally bounded as row count,
   content length, and redundant clipping depth grow. Adding a real compositing
   layer may add real mandatory cost; adding a non-drawing wrapper must not.
5. A priority change changes order, not identity. Geometry that remains covered
   keeps its surface and valid content. Scroll sampling does not cause continuous
   resizing, density oscillation, or retained-scene resets.
6. Warm, correctly painted content remains presentable under its clips while
   native scrolling proceeds independently of the main thread. Preserve the
   prior fix that removed the warm-host CSS visibility gate.
7. Equivalent normalized inputs and capabilities produce the same planning and
   scheduling decision on every chassis. Platform adapters supply observations,
   capabilities, and execution; they must not fork the policy or its heuristics.

### Ownership and planning contract

Keep geometry, density math, activity tracking, deadline prediction, residency
admission, and work ordering in a shared, deterministic Rust core alongside
`pax-runtime/src/engine/layer_tiling.rs` and `layer_surface.rs`. Prefer focused
modules in the existing runtime over a new crate or speculative plugin system.
This revises the earlier phase 2 draft's placement of admission/work policy in
the web host. The first-pass TypeScript scheduler is the migration source, not
a second policy implementation to preserve indefinitely.

| Responsibility | Owner / output |
| --- | --- |
| Sample presentation geometry and movement | Chassis adapter: versioned ancestry, local extents, affine transforms, clips, observed offsets, monotonic timestamps |
| Track activity and compute prospective coverage | Shared Rust core: per-axis movement/velocity, expiry, inherited exposure, conservative clipping and composition dependencies |
| Choose tile geometry and density | Shared Rust core: stable tiles and physical metrics using adapter-supplied display scale and backend limits |
| Admit residency and order preparation | Shared Rust scheduler: costs, deadlines, required coverage first, bounded optional work, explicit rejection reasons |
| Install, retire, initialize, replay | Chassis coordinator and renderer: execute the plan, preserve leases/generations/survivors, report actual readiness and costs |

The shared core consumes plain typed snapshots plus prior scheduling state and
an explicit clock. It owns policy state per engine instance, with no browser,
DOM, Swift, or platform-event dependency. Inputs include resource residency and
outstanding reservations, measured preparation costs, supported execution
capabilities, and platform limits. Outputs are coverage/tile requirements,
admission, ordered preparation requests, and rejection/retirement reasons.
Platform-specific serialization lives at the adapter boundary.

Web's adapter reads native browser presentation state, transports a batched
snapshot through the existing Wasm bridge, and executes the returned decision
through the DOM coordinator. The native adapter obtains equivalent observations
from its compositor/scroll hosts. Neither adapter independently re-ranks work.
Actual allocation, lease release, asynchronous completion, and command execution
remain with the existing platform/backend machinery; the core consumes those
facts rather than predicting ownership release.

The native path already calls `scroller_canvas_plan_with_policy` and consumes
`LayerSurfaceLayout`/replay priorities in
`pax-chassis-common/src/core_graphics_c_bridge.rs`. Use these existing seams as
the starting point for adoption. Do not assume the remaining work is zero:
audit native presentation-offset sampling, clip/transform snapshots, resource
lifetime reporting, and whether initialization can consume incremental work.
Different canvas/Metal surface topologies need capability/cost inputs, not
browser-specific concepts forced into a shared interface. Browser canvas caps,
WebKit fallbacks, and native surface limits remain adapter-provided constraints.

Move the reusable first-pass scheduler/activity tests to the shared policy and
keep browser tests for measurement, serialization, DOM installation, and lease
integration. Remove the superseded TS policy after parity is established. Do
not retain a JavaScript fallback planner/ranker or build a parallel Swift one.

Use an immutable measurement generation for each planning transaction. Geometry
revisions and surface/content generations remain distinct from priority-only
updates. An incompatible or missing snapshot preserves compatible survivors
and requests a fresh plan; it does not authorize a guessed JS fallback.

The planner emits mandatory tiles plus bounded, incremental optional coverage
extensions, each with a marginal physical cost. It must not expand the entire
scroll tree into every combination of ancestor/child warm windows. The scheduler
selects extensions under the shared budgets before any DOM allocation. Tile
quantization is included in their cost, not added after admission.

Coverage construction proceeds top-down from the browser viewport. Account for
each ancestor's transform and clips, including clips inside a moving content
subtree. Sweep the prospective view through the observed active scroll chain
over a bounded horizon, then map it into child content coordinates. Deduplicate
overlapping demand for the same owner/tile, while retaining distinct surfaces
where composition requires them. Simultaneous parent/child movement uses their
combined predicted trajectory, not a Cartesian product of independent margins.

Rectangular clips should be tight; arbitrary paths/masks may use conservative
bounds. This is coverage planning, not exact occlusion detection. Mask sources
and other required effect inputs must retain their existing render dependencies
even when they do not independently intersect the screen. Native-only ancestry
does not manufacture a GPU surface. Singular transforms must be handled
explicitly, without invalid inverse coordinates or unbounded demand.

### Screen-density and tile geometry policy

Backing density derives from the full local-to-presentation transform and
chassis display scale (browser DPR on Web). For uniform scale `s`, use
`display_scale * abs(s)` per local unit. A local backing density below one is
valid when a component is genuinely scaled down;
it is not a performance-quality fallback. Magnification increases required
density. Native text continues to use browser rasterization.

For orthogonal transformed local axes, including rotated nonuniform scale, use
their per-axis lengths. For general shear, begin with a conservative bound
using the largest singular value of the linear transform. Do not mistake a
rotated bounding box for scale.
This preserves quality under rotation/shear, at the cost of explicit
over-allocation in some anisotropic cases; tighter raster bases are deferred.

Share the physical sizing calculation; chassis supply backend limits and use
its result for planning, allocation, and reconciliation. Initially verify this
on WebGPU and Piet, then on native adoption. Quantize sizes conservatively
upward; grow density when needed and defer optional shrinkage until stable to
avoid resize churn.
Translation and pure rotation must not change density. Revisit the current
minimum-local-DPR assumption without relaxing hard backend safety limits.

Tiles stay anchored in owner content space, not recentered on the viewport
every frame. Derive their physical shape/extent from projected coverage and
backend limits. Sweep a bounded set of tile sizes in the baseline experiment;
do not assume that globally shrinking the current 2,496-pixel target wins,
because smaller tiles also increase surface count and initialization overhead.
Do not crop/reallocate to each tiny clip change. Charge the resulting stable
edge-tile slack explicitly and test that it stays bounded.

### Residency and work budgets

Retain existing platform canvas caps (64 on the current WebGPU path; preserve
the lower platform/fallback policies). Add an explicit physical backing-pixel
limit and a shared speculative allowance related to the physical viewport:

```text
optional pixel allowance = min(viewport-related allowance,
                               hard pixel limit - mandatory - other unreleased)
```

Clamp the result to zero. Charge a physical allocation once: resident and
retiring backings, plus reservations for in-flight allocation, count until
release. `other unreleased` excludes allocations/reservations already charged
to mandatory coverage and includes obsolete backing still awaiting release.
A pooled canvas is not free in pixel terms if its large backing is
still retained. Reuse existing pool/backend lifecycle signals; do not introduce
a second contradictory ownership ledger. RGBA backing bytes remain an estimate,
not total GPU memory including image caches, attachments, and effect textures.

Optional work is not entitled to fill every spare slot. A stationary offscreen
branch gets no independent large warm window. On-screen stationary children
may get a small first-interaction cushion, bounded by their projected viewport
and the global allowance. Ancestor-exposed children get that same small cushion,
not a full per-axis viewport expansion. It can fall to zero under pressure.

Keep a separate preparation-work allowance for each frame/turn, informed by
measured initialization, replay, and upload costs. Rank queued jobs by coverage
class and predicted deadline; within comparable urgency prefer ready/reusable
work and deterministic fairness between branches. Residency retention is not
a request to repaint. A ready trailing tile can be cheap to keep even when
preparing a new trailing tile would be inappropriate.

Do not start a burst of optional work that exceeds the frame's preparation
allowance. Required overdue work remains first, but do not process an unbounded
queue synchronously. Existing indivisible jobs and asynchronous GPU calls are
not magically preemptible: report overruns, stop additional optional starts,
and split replay into bounded units where the existing contract permits it.
Keep initialization concurrency unchanged. General interruptible scene mounting
requires a separate design decision if profiling establishes that need.

If mandatory demand alone exceeds count/pixel limits, drop optional demand,
preserve valid visible survivors, and report the exact exhausted constraint.
Distinguish this from temporary lease pressure and missed work deadlines. Never
silently increase the cap, lower projected screen density, or present unrelated
old pixels as though they were the newly exposed content.

### Directional and recursive activity policy

Use per-axis observed offset movement, covering wheel, touch, inertia, keyboard,
and programmatic scrolling. Hover, DOM focus, and bubbled input do not establish
activity. Multiple branches can move concurrently. Reset velocity for resize,
reparenting, clamping discontinuities, and resuming after suspension.

For each moving axis, estimate a preparation horizon from a bounded recent
readiness-latency estimate plus a safety margin. Project speed over that horizon,
then cap the extent by the shared pixel/work allowance. Latency includes queue
wait; use bounded estimates so congestion cannot produce a positive feedback
loop of larger windows and more congestion. Use time-to-exposure as the ordering
signal, not raw distance in incompatible local coordinate spaces.

- **Moving parent, stationary child:** inherit the deadline for the child's
  soon-visible clipped region. No large warm expansion on the child's idle axis.
- **Moving child:** add directional coverage on its own moving axes. Required
  parent coverage remains intact, but activity does not spread to unrelated
  siblings merely because they share an ancestor.
- **Both moving:** combine transformations/movement for exposure prediction and
  deduplicate demand. Native scroll chaining changes priority when offsets
  actually begin changing in the receiving ancestor.
- **Reversal:** adopt the new direction on the next sample. Re-rank queued work,
  keep a bounded ready reversal buffer, and retain valid in-flight completions.
  Cancel obsolete queued speculation, not valid work solely because rank changed.
- **Pause:** keep the current 250 ms activity tail as an initial hypothesis;
  momentum renews it. On expiry, reduce speculative entitlement without tearing
  down useful ready surfaces every frame. Optional fairness cannot override a
  nearer deadline or displace visible coverage.

Numerical policy choices remain internal and must be calibrated in stage 1:
tile extent/shape, pixel cap and viewport multiplier, inactive-child cushion,
latency estimator/safety margin, work allowance, and shrink hysteresis. Record
chosen defaults and sensitivity results before enabling the policy in the real
website. The invariants above are requirements; these constants are not claims
of already-established optimal values.

### Content readiness and turnover boundaries

Trace the critical path separately: component expansion/layout, image request/
decode/upload, surface allocation/initialization, retained replay, submission,
and exposure. A submitted tile can still contain missing application content.
Use image-free fixtures to isolate the planner, then the real examples to
expose these other costs. Do not label a missing-content gap a capacity denial.

Prefer preserving renderer resources and content-space retained records when
a surface slot moves. Audit why each origin change requires a reset before
attempting to remove it; masks, lighting, transforms, and resource ownership must
remain correct. If making replay independent of tile origin requires a broader
retained-scene redesign, present that as a separate scope decision. Instrument
it now; do not promise the planner alone eliminates those hitches.

No new public preparation/proximity API is approved here. If application-owned
virtualization needs access to predicted exposure, propose an explicit contract
instead of widening public viewport events or teaching the scheduler Paxflix-
specific rules. Calculator draw-ahead and carousel mounting remain separate
integration findings until their changes are expressly included.

### Validation and acceptance

Extend `examples/src/surface-scheduling` to compare identical content at the
same presented dimensions/DPR: standalone, one clipped/scaled embed, and two
independent embeds. Compare aggregate visible area, not an entire standalone
window against two smaller demos. Keep a pre-mounted, image-free mode and a
controlled delayed-content mode. Run baselines without concurrent compilation.

Required deterministic cases and browser checks:

| Case | Required result |
| --- | --- |
| Same normalized snapshot/capabilities in native Rust and Wasm | Same coverage, activity, admission, deadlines, and work order; no browser-only policy dependency |
| Add rows/content length or non-drawing clip wrappers | No growth in admitted optional pixels beyond policy; no new mandatory surfaces for wrappers |
| Vertical parent exposes horizontal shelves | Prepare upcoming clipped shelf frames; idle horizontal speculation stays within the small child allowance |
| Start horizontal shelf movement | Optional work follows that shelf/axis without losing sibling visible coverage |
| Two movers, reversal, momentum, chaining | Stable deterministic priority, bounded queues, no rank-only generation churn |
| Scaled/rotated/sheared embeds; DPR 1/2/fractional | Correct projected density, bounded conservative slack, no translation/rotation density churn |
| Exhausted pixels/count, outstanding leases, late completion | Distinct diagnostics, safe reuse, no stale publication or oscillating eviction |
| Repeated warmed scroll and a long soak | Stable resources/scene size where content is fixed; no growing per-cycle allocation/reset work |
| Cold jump beyond prepared coverage | Immediate visible-first recovery with attributed latency; no false claim of gap-free preparation |

Capture continuous motion, not only settled screenshots: slow/moderate travel,
one viewport in about 400 ms, faster travel, horizontal movement, and abrupt
reversal. Use both directions and first-visit/revisit runs at desktop and 390px.
Record exposed-unready area and duration, first matching submission, backing
pixels by required/optional/retiring class, candidate/admission counts, pool
allocation, renderer creation, scene resets, replay work, queue latency, image
readiness, and engine/frame p50/p95/max. Native text/content presentation belongs
in the visual checks, not only canvas revision checks.

For the warmed image-free fixture, motion whose exposure deadlines are within
the measured preparation capability must show no scheduler-caused missing
coverage and no tile-turnover long task above 50 ms. Use a 16.7 ms total-frame
target on the reference 60 Hz run; report higher-refresh results separately.
The 50 ms threshold is a failure alarm, not the smoothness goal. Baseline work
must set the repeatable workload/velocity envelope and per-stage work allowance
before implementation; do not redefine the envelope after observing failures.

Real Paxflix must show reduced backing overhead and first-visit/revisit timings
without hiding either app or reducing screen quality. Remaining gaps/hitches
must be attributed and remain open acceptance findings, even if the image-free
fixture passes. Validate debug/release and WebGPU/Piet; physical iOS/WebKit and
touch/momentum require actual target checks, not inference from viewport size.

During the Web proving stage, run shared-core tests on the native host as well
as the Wasm/JS bridge tests, and retain native chassis regression coverage with
its current policy activation. When enabling the new policy on Apple targets,
repeat matched nested fixtures and real examples on macOS and iOS/iPadOS;
report device/simulator and backend distinctions. Shared tests establish policy
portability, not a claim that native scrolling performance has been validated.

### Implementation sequence and approval boundary

1. **Baseline and policy calibration:** matched fixtures, continuous traces,
   component/replay attribution, budget accounting, and recorded numerical
   defaults. Establish which portion of the hitch is planner/renderer work;
   audit Web/native adapter inputs and capabilities before fixing the interface.
2. **Shared core, coverage, and density:** port reusable admission/activity policy
   to Rust with parity tests; add the versioned clip/transform snapshot,
   required/optional geometry, projected density, stable content-anchored tiles,
   and cross-boundary allocation tests. Keep existing ownership protections.
3. **Recursive budget policy, enabled on Web first:** physical-pixel admission,
   parent-inherited exposure, minimal inactive-axis preparation, bounded velocity/deadline
   lookahead, and a preparation-work allowance consumed end-to-end. The policy
   resides in shared Rust even while Web is its only enabled adapter.
4. **Turnover audit and targeted fixes:** remove avoidable recreation/replay
   bursts supported by traces; do not bypass retained-scene correctness. Bring
   back any broader renderer or component-mount redesign for approval.
5. **Real integration and documentation:** repeat website tests; update
   `scrolling-viewports.md`, applicable API comments/generated docs, architecture
   topology if ownership changes, and pain-points with proven behavior. Keep
   proposed guarantees out of the public learning path until established.
   Record the Web benefit and a concrete native-adoption delta: missing adapter
   inputs, capability differences, and required platform tests. If beneficial,
   follow with macOS/iOS/iPadOS activation using the same shared core, rather
   than treating Web completion as the endpoint of the work.

No authoring syntax or baked program fields are planned. Version/test the web
bridge contract in debug and release. If implementation crosses into manifest
or program representation, audit binary baking as required by AGENTS.md.
Native Apple activation remains unchanged during Web proving; shared hooks
retain current defaults until the relevant adapter is enabled and validated.
This is a rollout boundary, not a permanent exclusion of other platforms.

**Approved:** proceed with these five stages, treating numerical defaults as
measured policy choices, shared policy from the outset, Web-first activation
with a native-adoption follow-through, and separate scope gates for general
mount scheduling, application virtualization APIs, and retained-scene redesign.

### Phase 2 checkpoint A — shared admission and projected density

Implemented on 2026-10-05:

- Shared Rust now owns observed per-axis activity, affine projection, ranking,
  deterministic count admission, and testable pixel/optional-budget admission.
  Web currently activates the existing count-only budget; the pixel-budget API
  is not yet an enabled policy. The superseded TS activity/ranking/projection
  implementations have been removed. Browser integration tests invoke the real
  Rust serialization adapter through a native fixture executable.
- Shared backing metrics use the full affine basis and display scale. Orthogonal
  axes retain independent densities; shear uses the maximum singular value.
  Density is rounded up to 16 logarithmic steps per doubling (under 4.5% density
  overhead), grows immediately, and shrinks after 500ms at a stable target.
  These are initial anti-churn settings, not a tuned preparation-work policy.
- Installed surface contract version 2 carries exact backing dimensions. The
  renderer consumes those dimensions instead of reapplying global browser DPR.
  A newly discovered hard backend limit triggers replanning rather than a second
  sizing decision. Surface signatures include physical dimensions; priority-only
  changes do not alter identity. Readiness requires matching geometry as well as
  host/origin. No manifest or baked program representation changed.
- The image-free fixture has matched direct/scaled single and paired modes.
  At 1280×720, DPR 2, settled top-of-page admitted backing pixels changed as follows:

| Mode | Before | After | Surfaces before → after |
| --- | ---: | ---: | ---: |
| One direct | 26,910,720 | 26,910,720 | 26 → 26 |
| One 0.5× embed | 96,583,680 | 26,910,720 | 44 → 26 |
| Two direct | 48,973,824 | 48,973,824 | 38 → 38 |
| Two 0.5× embeds | 124,452,864 | 48,973,824 | 50 → 38 |

Measurements were taken without concurrent builds. These include warm backing
storage; they are not redraw costs, total GPU memory, or continuous-frame timing
results. One-page scrolling in the paired transformed fixture rendered correctly
in settled visual checks and left the stationary sibling intact.

Remaining before phase 2 acceptance: replace the independent JS warm-host window
with shared recursive clipped exposure, calibrate/enable physical residency and
work budgets with unreleased-resource accounting, collect continuous-motion
readiness/timing traces, audit turnover cost, and validate WebGPU/Piet, mobile
input, release runtime, and native adapter follow-through. The density result is
not evidence that first-visit mounting or rapid-scroll gaps have been solved.

Checkpoint validation:

- 87 browser adapter/regression tests pass, including real Rust bridge decisions.
- 80 runtime engine tests pass (2 intentionally ignored), including projected
  density, minified tiling at 0.25/0.5 local density, hysteresis, affine projection,
  activity, and count/pixel admission cases.
- Six focused Web chassis installation/density tests pass. The broader default
  chassis suite has 14 passes and one failure in the existing image-loading
  fixture `late_completion_after_unmount_keeps_the_cache_without_scheduling_stale_nodes`
  (`properties not of expected type`). Enabling `designtime` for that suite also
  exposes its unguarded use of `PaxEngine::new_empty`. Neither is claimed fixed.
- Production-feature Wasm chassis check and native `pax-chassis-common` check
  pass. The website and probe build for Web; desktop and 390px browser smoke
  checks render both embedded apps and allow independent phone scrolling.
  The narrow viewport uses desktop browser input, not a physical iOS device.
- API reference regeneration, `mdbook build pax-docs/book`, Pax fixture
  formatting, and whitespace checks pass. Full `tsc` still reports the 15
  pre-existing host/control/type-declaration diagnostics; no clean full-typecheck
  claim is made.
- The dev-look sampled capture is not a continuous-motion performance result:
  only one frame was returned, and its composed native-scroller snapshot differed
  from the browser presentation. Browser screenshots were used for the settled
  visual checks. Repeated root `pax-tile-window-escape` diagnostics were also
  observed at a 32px tile-edge boundary; investigate during turnover/tracing work.

The original release server on port 52957 is untouched. The new checkpoint is a
separate debug development session (hot reload disabled for this runtime probe).
No commits or ticket-state changes were made.

### Phase 2 checkpoint B — recursive exposure and physical admission

Implemented on 2026-10-05, before the replay extension:

- Shared Rust `surface_preparation` computes ancestor/child motion on one time
  axis, intersects intermediate clips, and solves between-sample exposure.
  Independent child overscan no longer multiplies preparation windows.
- Stable absolute content-grid cells retain their geometry as preparation changes.
  Current defaults are a 250ms horizon, at most one predicted viewport per active
  axis, 2496-physical-pixel tile target, 64Mi backing pixels, and two physical
  viewports of optional residency. These are initial proving values, not calibrated
  optimums. The lower fallback surface-count policy is preserved.
- The pool reserves physical pixels before initialization and charges retiring
  allocations until their final lease releases. Pixels count once; estimates
  exclude effect attachments and image caches. This is not a work-time budget.
- At 1280×720/DPR 2, the image-free fixture settled at 7,460,352 backing pixels
  / 5 surfaces for one direct catalog and 11,234,304 / 8 for two scaled catalogs.
  The corresponding checkpoint-A values were 26,910,720 / 26 and 48,973,824 / 38.
- A corrected-clock, 400ms paired/scaled debug probe had no visible-unready
  transitions, p95 callback interval 48.9ms, and phase p95 values around 16ms
  tick, 12.1ms render, and 2.5ms surface reconciliation. Its release counterpart
  also had no visible-unready transitions but a 59ms maximum callback interval.
  These short traces do not establish 60fps, nor a release speedup. An abrupt
  cold page jump previously exposed a roughly 51ms readiness gap.

Validation at this boundary: 91 browser tests, 87 runtime engine tests (2 ignored),
Wasm production check, native chassis-common check, and debug/release fixture
builds passed. TypeScript retains 15 pre-existing diagnostics. Physical iOS,
complete fallback/browser coverage, and the preparation-work budget remain open.

### Phase 2 checkpoint C — content recording is independent of tile replay

Zack approved this extension after the replay-design discussion on 2026-10-05.
Its scope is drawing reuse and invalidation separation, not a general renderer
rewrite, new list virtualization, new tile limits, or an adaptive quality policy.

The shared WGPU adapter now owns a logical drawing cache above physical tiles:

- Bounded-node recordings contain immutable drawing values and inherited graphics
  state, including Frame clips, detached mask-source programs, and opacity scopes.
  Paths are converted once during recording. No component callbacks or live
  Property handles are stored; images refer to the existing versioned image store.
- Region requests select candidates through the existing scene geometry index.
  Valid cached drawings queue for tile execution; only misses become dirty
  component work. Ordinary content changes remain independent and reach all
  affected resident tiles. Concurrent changes/removals no longer force the WGPU
  path into the old whole-layer conflict fallback.
- Surface replay runs after current content recording and removals. Recording or
  removing a node cancels its queued old record. Tile origin/size/generation
  changes do not evict logical recordings; node removal and layer truncation do.
- The cache owns no GPU surfaces. LRU retention is limited to 4096 records and
  1,000,000 weighted payload units (commands, path segments, paint entries, etc.).
  Shared prefixes are conservatively charged per record. These bounds are not a
  claim of exact byte accounting; pending replay pins its selected records until
  the layer flush. Eviction falls back to ordinary component rendering.
- Physical WgpuRenderer scenes, tile-local bindings, GPU preparation and ordered
  rasterization still exist. This is a reusable drawing-program cache, not yet a
  fully shared prepared GPU scene or persistent batch-buffer redesign. Ancestor
  programs may execute for multiple leaves. Those costs should be measured before
  another optimization. Piet keeps the legacy replay path through trait defaults.
- No manifest, compiler/cartridge, or baked program representation changes.

Focused regression coverage checks cache bounds and lifetime; sibling clip-state
isolation; detached-source preservation; a runtime replay without component render
calls; independent content invalidation; and real Metal tile survival, cached
density-change replay, masks and group opacity using pixel readback.

Website comparison: frozen pre-extension debug build versus rebuilt debug build,
1280×720/DPR 2, one phone-local viewport over a 400ms programmatic probe, no
concurrent builds during these measurements:

| Sample | Before | After |
| --- | ---: | ---: |
| First visit: callback interval p95 | 103.4ms | 101.7ms |
| First visit: tick p95 | 69.6ms | 64.9ms |
| First visit: render p95 | 13.2ms | 13.2ms |
| First visit: visible-unready transition | ~25ms | ~23ms |
| Return: callback interval p95 | 46.7ms | 25.5ms |
| Return: render p95 | 1.9ms | 1.9ms |
| Return: visible-unready transition | none | none |

These are single short local samples, not a statistically established speedup.
The cold visit still has substantial tick/mount cost; replay reuse cannot avoid
the first recording of never-rendered content. After website review, choose the
next measured target with Zack rather than automatically broadening this pass.

Checkpoint C validation: 264 runtime unit tests and 6 scene-geometry integration
tests pass; the two normally ignored GPU tests also pass when run serially with
Metal access. Debug and release website Web builds pass, as does the Wasm chassis
check with the Piet feature enabled. Desktop and 390px browser smoke checks show
intact masks, opacity, clipping, and independently scrollable embedded Paxflix
instances, with no browser error logs observed. These are settled visual samples,
not proof that every intermediate frame is complete. API reference regeneration,
`mdbook build pax-docs/book`, and `git diff --check` pass. Physical iOS and a full
native application run were not tested in this checkpoint.

### Phase 2 checkpoint D — eager cards and single-flight images

Zack approved restoring eager card mounting and measuring/removing duplicate
image work before expanding GPU residency policy. Implemented on 2026-10-05:

- Removed Paxflix's provisional per-shelf `if loaded` gate and catalog-scroll
  inputs. Each instance mounts all 210 cards. The website still owns the explicit
  control flow that instantiates the entire showcase; no runtime lifecycle
  scheduling, lazy evaluation, or list virtualization was added.
- One browser ImageLoader per chassis coalesces concurrent requests by the
  existing logical source string. One decode publishes its pixels once through
  a batched Wasm API and wakes every current live consumer. Decoded-cache hits
  wake consumers without replacing pixels/incrementing versions. Subsequent URL
  requests detach consumers from old requests; failures release bookkeeping for retry; teardown aborts
  fetches/backoff timers and suppresses late publication into a freed chassis.
  The existing app-relative source-resolution contract is unchanged.
- Surface tracing now distinguishes request, decode, CPU-ready, GPU-upload/cache
  hit, changed image draw, and image-bearing surface submission. Image-state
  diagnostics and the event log are bounded and cleaned on retirement. The
  image-submit event means CPU-side presentation, not completed GPU execution.
  Native/Piet diagnostics keep no-op defaults; their loaders are unchanged.
- GPU preparation still uses the existing shared directional exposure and replay
  order. No independent image cache or new warming region was introduced. The
  current speculative-batch bound is **not** a calibrated millisecond work
  allowance; implementing that remains open from checkpoint B.

The 1280×720/DPR-2 debug website, with both catalogs instantiated, produced 37
unique image requests/decodes and 389 coalesced requests. The 35 catalog thumbnail
sources were already decoded before phone scrolling. Subsequent cache-hit
requests did not transfer pixels again; observed image versions remained 1.
The following short programmatic probes were captured without a concurrent
build. They diagnose stages, not a controlled before/after speedup:

| Phone-local motion | Tick p95 | Render p95 | Full frame p95 |
| --- | ---: | ---: | ---: |
| First viewport / 400ms | 35.3ms | 13.6ms | 60.0ms |
| Next viewport / 200ms | 37.0ms | 12.4ms | 59.7ms |

Neither scroll generated a request, decode, or missing-decoded-pixels event.
The first newly exposed row's tile initialized in about 0.7ms, then waited about
57ms before its first image upload and submitted about 72ms after first visible
demand. Later predicted rows submitted before becoming visible. The faster probe
reused 14 existing GPU image bindings and performed 21 uploads; retired rows
released their leases after leaving the prepared region.

Two remaining constraints are now concrete:

1. A 2496-physical-pixel grid cell spans about 1484 local pixels in the embedded
   phone. One row therefore prepares seven posters for a roughly 374-local-pixel
   viewport showing about two. The effective visible area does not yet bound the
   amount of image work within a tile. Do not simply mark a partially drawn tile
   ready: reducing this work needs smaller stable geometry or explicit partial
   validity/preparation semantics.
2. An asynchronously initialized tile waits behind another application tick
   before recording/upload/submission. The probe spent roughly 30–37ms in active
   scroll ticks and another 7–8ms processing native messages. Finer GPU work alone
   cannot recover that main-thread runway. A preparation fast path must preserve
   latest content/clip state and ordering; it is not permission to skip lifecycle
   updates or add general lazy mounting.

The observed GPU hits support the existing weak-resource lifetime policy: a
texture can be shared while any tile retains it, and can disappear after the last
binding. A longer-lived speculative image cache is not justified by this forward
scroll sample. Fast-scroll completeness and a preparation-work time budget remain
unresolved; this checkpoint must not be described as eliminating all blank tiles.

A follow-up release probe uncovered click-through from the browser trace
inspector into Pax's window-capture hit testing. Its button opened a movie detail
view underneath, so that sample was discarded. The inspector now tags its subtree
and the application input bridge excludes those events; a regression test covers
mouse, wheel, keyboard and touch input plus listener disposal.

With that correction, the release 400ms phone probe again performed no image
request/decode work. Its first cold row submitted about 93ms after visible demand;
later predicted rows submitted ahead of exposure. Phase p95 values were 38.3ms
tick, 23.3ms render, and 71.5ms full frame (18 frame samples). These small samples
do not establish a debug/release performance ordering or a sustained frame rate.

Checkpoint D validation: 101 browser tests, 15 Web chassis tests, 264 runtime unit
tests plus 6 scene-geometry integration tests, 23 ordinary GPU tests, 10 Paxflix
tests, and the opt-in Metal shared-image texture lifetime test pass. Other ignored
GPU tests were not rerun in this pass. The previous chassis unmount-fixture failure
was corrected by removing the child from its parent's control-flow lists before
unmounting; it now tests a valid tree lifecycle. The Wasm chassis check with Piet
enabled passes; an interactive Piet run and physical Apple devices were not tested.
Debug/release website builds pass; the JS interface was rebundled after the final
inspector-only correction. Desktop and 390px checks show both catalogs and working
nested vertical/horizontal scrolling after preparation, without browser error
logs. These are not an all-frames-complete guarantee. API references, canonical
image/scrolling articles, example README, and pain points are updated; mdbook and
whitespace checks pass. Full TypeScript checking retains the same 15 pre-existing
diagnostics; no new diagnostics appear in the image-loader/trace changes.

### Phase 2 checkpoint E — approved preparation granularity and execution

Zack approved proceeding with the recommended sequence: viewport-aware stable
tiles; opt-in subphase timing of the runtime tick; a guarded presentation-only
path using settled drawing records; then cost-aware preparation admission.
Eager component mounting, screen density, shared image lifetime, canvas/pixel
caps, latest-content correctness, and native/Piet behavior boundaries remain
invariants. No manifest or baked-program schema changes are planned.

The first sizing experiment opts WebGPU's explicit-coverage planner into a
one-host-viewport scrolling-axis cap, rounded up to 256 physical pixels and
bounded by the existing tile/area/backend caps. The origin stays on an absolute
content grid. Moving ancestor clips do not resize surviving cells. Native and
Piet retain their current sizing defaults pending adapter-specific validation.
Timing is observational and opt-in; it must not change tick/lifecycle counts.

The Web frame now offers a guarded pre-tick presentation pass. It queries only
the settled spatial index and atomically queues complete cached drawing programs.
Pending reactive effects/cutoffs, geometry, and native occlusion block the pass
without consuming jobs. Canvas/lighting dirtiness and removals block their layer;
dirty content is tested against selected nodes. Unrendered cold layers or an old
hot-reload layer awaiting removal must not block unrelated cached replay.
A missing record defers the job
to ordinary rendering; no partial record set is published. The renderer checks
current physical ownership when draining async completions, and reapplies cached
lighting to new surfaces. The normal frame then samples inputs and ticks as
before. Async completion can request one coalesced microtask pass; the same gate
permits at most one additional pass between ordinary frames, and disposal rejects
queued old-chassis notifications. There is no extra lifecycle tick, microtask spin, or cached copy of
mutable component state. Native callers may reuse the shared API later; only Web
invokes it now, and Piet defaults to an empty opt-in job queue.

`SurfaceWorkBudget` cooperatively bounds this extra pre-tick pass. Its initial
allowance is 4ms; the estimated job cost rises immediately and decays gradually.
At least one job may run, so an indivisible upload/encode can overrun. Later jobs
are deferred without losing their replay scopes. This is intentionally **not**
reported as the full preparation budget: ordinary rendering/visible repairs and
async allocation retain their current behavior. Cache misses still require that
path. The first experiment should establish whether bypassing the tick helps
before dividing normal render/content work into a larger scheduling system.

#### Checkpoint E validation and findings

Matched release probes used the preserved checkpoint-D build and the new build
at the same 1280×720 browser viewport. The embedded phone projected to 313×596
CSS pixels; its parent page was at scroll offset 640, so ancestor clipping was
active. Browser backings in this run were 1 physical pixel per CSS pixel (not the
earlier checkpoint-D DPR-2 setup). No build ran during the probes. Each phone
probe moved one local viewport (712 units) in a requested 400ms; reported phase
samples also include the inspector's short post-input tail.

| Release phone probe | Tick p95 | Render p95 | Full frame p95 | Image uploads / bytes during probe |
| --- | ---: | ---: | ---: | ---: |
| Preserved checkpoint D (25 frame samples) | 35.7ms | 22.9ms | 64.6ms | 29 / 26,726,400 |
| Checkpoint E (28 frame samples) | 32.4ms | 17.4ms | 59.8ms | 8 / 7,372,800 |

The three newly prepared poster-row surfaces were 512×180 instead of 2140×180;
the smaller grid also required one 315×768 phone-background tile. Both probes
performed zero new image requests or decodes. This is a short diagnostic
comparison, not a sustained-FPS or statistical speedup claim. The dominant new
tick subphase was occlusion reconciliation (26.8ms p95); ordinary node update
was 0.2ms and import-settings synchronization 4.2ms. A sibling-tablet 400ms probe
had 56.9ms full-frame p95 and 27.6ms occlusion p95. No fairness/budget caps were
expanded to obtain these results.

The first cold phone probe had cache misses and no early cached submissions.
Its first new row initialized in 0.9ms, then waited about 62ms for normal render;
prediction nevertheless submitted it about 48ms before its first visible-demand
event. A following 200ms phone probe exercised a successful cached presentation
job in 0.4ms. Cold rows and reversal still produced cache misses. Therefore the
pre-tick path is proven reachable, but **not** the source of the first-probe
improvement or a complete solution to cold-tile delay. Drawing programs are
tile-independent once recorded; never-drawn content still needs first recording.
The 4ms allowance remains initial calibration, not a demonstrated global frame
budget. Incremental occlusion and preparing first drawing records without
waiting behind another full tick remain separate next targets.

Desktop phone/tablet probes, faster phone movement/reversal, and a 390×844 mobile
phone probe plus horizontal shelf scroll settled to populated images. Readiness
snapshots had no waiting-decoded or recorded-not-submitted images; this does not
assert that every intermediate native-compositor frame was complete. Current
release runs had no browser errors. Debug hot-reload startup still emitted the
existing 100,000-effect drain limit warning during the full remount; this was
not conflated with release scrolling measurements.

Validation: 269 runtime unit tests, 78 runtime-API tests, 6 scene-geometry tests,
15 Web chassis tests, 103 browser tests, four explicit-Piet policy tests, and
the real Metal incremental-tile/mask/opacity/cache-miss regression pass. Wasm
checking with Piet enabled passes; physical Apple UI and interactive Piet were
not tested. Debug and release website builds, API regeneration, mdbook, and
`git diff --check` pass. TypeScript retains its same 15 pre-existing diagnostics.
No manifest/baked-program schema, lifecycle-mounting rule, screen-density policy,
or decoded/GPU-image lifetime was changed.

## Implementation checkpoint F — explicit runway and spatial retention

Zack reported a main-page regression after checkpoint E: native scrolling was
smooth, but even basic page movement exposed unsubmitted tiles. Reproduction at
1280×720 / DPR 2 (main scroller 1280×640) confirmed E's page cells had shrunk
from 1248 to 640 CSS pixels tall. The idle planner supplied no explicit margin,
so the first pixel crossed a cold boundary. A requested 1000ms page probe had
about 104ms between first visible demand and first submit. Ready forward tiles
were then retired about 250ms after motion stopped, and recreated on resumption.
There were 52–58 spare canvas slots and no admission denials in those probes.
The previous larger cells had masked this policy gap with incidental coverage;
they were not a guarantee of complete frames either.

The approved correction retains E's smaller cells and existing raster density:

- Shared `surface_preparation` now distinguishes actual forward trajectories,
  independent safety alternatives, and retain-only alternatives. Exposed hosts
  request half a local viewport on each scrollable side, even while stationary.
  Each hypothetical safety movement varies one ancestor/axis at a time; no
  Cartesian product of parent/child padding. Offscreen children inherit future
  exposure but cannot independently inflate their own idle axes.
- A one-viewport spatial retention margin admits compatible, already-submitted
  assignments at lower priority than visible, forward, or safety demand. Cold
  or unsubmitted tiles in that margin remain denied. Nearby submitted tiles do
  not expire with the movement signal; distance and resource pressure govern
  eviction. This is spatial hysteresis, not an unbounded lifetime extension.
- Candidate enumeration includes the union AABB, but admission tests exact
  trajectories, preserving rejection of diagonal sweep corners. All admitted
  surfaces use the existing recording/replay pipeline; this does not allocate
  blank canvases as a substitute for content preparation.
- The Web adapter samples monotonic assignment-to-observed-submission latency
  once per content revision, independent of tracing. Shared policy applies a
  fast-rise/slow-fall estimate to a 250–750ms horizon, with a two-viewport travel
  cap. This includes queue and first-recording delays plus at most the readiness
  observation interval; it is not GPU completion or asynchronous image readiness.
- Canvas count, total/optional physical pixel limits, lifecycle mounting,
  generation/lease validation, image lifetimes, and native scrolling are
  unchanged. The policy remains shared Rust with Web adoption; native rollout
  and broader frame-time budgeting remain separate work.

Focused tests cover exact boundaries, pause/reversal, scale, clipping, retained
versus cold eligibility, budget pressure, nested-axis isolation, sample identity,
bounded latency adaptation, and stable assignment timing. The debug preview's
initial 1000ms main-page probe, 400ms pause/resume, 400ms reversal, and 400ms
embedded-phone probe had no visible-unready transitions. These debug checks
overlapped release compilation and are behavioral checks, not performance data.

Final release validation, without concurrent compilation:

- Desktop 1280×720 / DPR 2: main-page initial 1000ms scroll, 400ms pause/resume,
  400ms reversal, and 400ms one-page scrolls in both phone and tablet recorded
  no visible-unready transitions. At initial rest, both next-row page cells
  were already submitted. The matched initial-scroll diagnostic sample had
  full-frame p95 71.0ms versus E's 76.6ms, and render p95 17.8ms versus 19.9ms
  (38 versus 33 samples, including the probe's 250ms tail). This is a short
  sanity comparison, not a statistically controlled throughput result.
- 390×844: main-page initial 1000ms scroll, 400ms pause/resume, 400ms phone
  scroll, and a subsequent requested 200ms phone scroll (218ms actual) likewise
  recorded no visible-unready transitions. A horizontal shelf smoke check
  showed populated imagery. The final image-readiness snapshot had no pending
  decoded or recorded-but-unsubmitted images.
- Across these sampled release traces, peak assigned canvases were 19 and peak
  charged backing pixels were 21,049,264, below the unchanged limits of 64 and
  67,108,864. Optional pixel-budget denials still occur; spare canvas slots do
  not authorize unlimited speculation. Settled screenshots and sampled traces
  do not establish that every native-compositor frame was complete, nor do
  they eliminate separate application-generated-content or network delays.

Validation: 273 runtime unit tests pass (two hardware tests remain ignored),
15 Web chassis tests, 105 browser tests, nine focused preparation tests, Wasm
checking with Piet enabled, debug/release website builds, regenerated API docs,
mdbook, and `git diff --check` pass. Full TypeScript checking retains the same
15 pre-existing diagnostics. Physical Apple UI and interactive Piet were not
tested. No manifest/baked-program schema or application content was changed.

## Implementation checkpoint G — nested occlusion and current urgency

Native wheel input exposed a blind spot in checkpoint F's programmatic probe:
the probe stops producing movement while the main thread is busy, whereas native
compositor scrolling continues. A cold phone shelf took about 72ms from observed
visible demand to image submission, including only 0.6ms of surface creation.
Occlusion consumed 23–29ms of the sampled 53–61ms frames. Free canvas slots and
already-decoded images ruled out count admission and download as that gap's cause.

- The shared occlusion reconciler now caches descendant presentation-container
  topology. Safe, clipped nested scroll islands refresh affected Frame/Scroller
  bounds and query native masks in old/new local viewport regions without a full
  scene rebuild. Structural or inherited geometry changes, escaped content,
  unclipped scrollers, and shared presentation domains retain conservative
  reconciliation. Rotated views query the conservative envelope used by native
  masking; exact clip paths are unchanged. macOS view-detachment eligibility
  remains leaf-only.
- Web initialization selects its targets after acquiring the shared-context
  mutex, comparing against current committed global urgency. A waiting layer
  that has lost priority yields through the existing requeue path. Already-issued
  GPU work remains non-preemptible and its result still requires a current lease
  identity. Replay regression coverage explicitly demotes former visible work
  and removes work no longer present in the current layout.
- Opt-in trace diagnostics record sampled native-scroll offsets and detailed
  frame phases during movement and its 250ms tail. Their clock starts at main-
  thread observation, not the compositor's first displayed movement. No tracing
  or DOM serialization is added to ordinary production frames.

The nested regression fixture has 12 framed horizontal scrollers and 1,920
leaves under a scaled, rotated parent. Forward movement and reversal perform no
full rebuild and fewer than 100 mask evaluations per tested step; native masks
and container presentation bounds match forced full reconciliation.

Lifecycle mounting, screen density, canvas/pixel caps, preparation margins,
image lifetime, and first-draw scheduling are unchanged. This pass does not
introduce deadline-driven rendering or a global preparation-time budget.

Release validation used native wheel input with no concurrent compilation, at
1280×720 / DPR 2. On the same first phone scroll (0→720 local pixels), the
preserved F build's scroll frame spent 34.6ms in occlusion and 67.3ms total.
G's occlusion peak was 3.6ms and full-frame peak 37.3ms across 14 sampled frames
including the 250ms tail. This is a short diagnostic comparison, not a sustained
FPS benchmark. The cold shelf's observed-visible-to-image-submit gap was 65.9ms
before and 68.1ms after: the remaining blank interval is not solved by this pass.
G spent 0.6ms creating that surface, then another 43.5ms before image submission.
The pre-tick pass deferred work and later encountered uncached drawing; ordinary
rendering still reached 22.5ms. Bounded first-draw preparation remains the next
candidate, not a larger admission window.

Faster phone movement/reversal, sibling-tablet scrolling, horizontal shelves,
and 390×844 mobile scrolling all settled with populated images and no browser
errors. Traces still showed visible-unready intervals during larger movements,
including about 189ms in one mobile sample. Final snapshots had no pending
decoded or recorded-but-unsubmitted images. These checks do not claim complete
intermediate compositor frames. Peak sampled desktop backing allocation was
25,628,455 pixels, below the unchanged 67,108,864 limit.

Validation: 274 runtime unit tests, 6 scene-geometry tests, 21 viewport tests,
16 Web chassis tests, and 106 browser tests pass (three existing runtime/manual
tests remain ignored). Wasm checking with Piet enabled, sequential debug/release
website builds, the Web interface bundle build, mdbook, and `git diff --check`
pass. TypeScript retains its 15 pre-existing diagnostics. A concurrent profile
build collision was discarded and the release rebuilt sequentially. Because
root-level website builds do not automatically enable libdev, the explicitly
rebuilt browser bundle was copied into both generated previews and verified by
hash before browser testing; use `--libdev` for subsequent full builds. Physical
Apple UI and interactive Piet remain untested. Human-facing scrolling and
compositing guidance is updated; no public API shape or baked-program schema
changed, so this checkpoint requires no API regeneration.

## Checkpoint H — bounded cold drawing preparation

WebGPU now opts into shared-runtime cold preparation rather than converting
surface readiness into ordinary whole-layer rendering work. Missing immutable
drawings are captured one primitive at a time, with ancestor clip/mask/opacity
hooks, without changing GPU targets. A complete physical tile is then replayed
and submitted atomically. Both stages consume the existing cooperative 4ms
allowance and existing surface replay queue. Pending work is reselected from
current visibility/intent; no second scheduling queue or larger resource cap
was added. Physical renderer identity, not its index in a partially initialized
host array, determines its current rank.

Important correctness boundaries:

- Cold targets cannot be flushed by unrelated ordinary dirty-surface work.
  Already-presented survivors still receive ordinary application updates.
- Animation continues recording current commands even when its resident tile
  is cold. The first submission must not wait for the animation to stop.
- A render-dirty node left behind by an unavailable GPU target is distinct from
  unsettled scene state. The capture path can repair it when geometry, effects,
  layer invalidations, and removals are settled. Cache-only replay on other
  backends retains the stricter clean-node requirement.
- Command-cache rejection or eviction during capture takes an atomic direct
  draw of the selected tile. This avoids capacity-induced retry loops; like a
  single expensive primitive or GPU submission, it can overrun the allowance.
- The browser coalesces after-render preparation with async-ready callbacks.
  Its default microtask function must be called with the browser receiver, not
  as a scheduler method. A regression test covers this browser-only failure.
- Eager mounting, lifecycle timing, application clock, density, warm margins,
  canvas/pixel caps, and image residency policy remain unchanged. Native and
  Piet first-draw defaults remain unchanged; the Rust implementation is opt-in.

Matched native-wheel release comparison at 1280×720 / DPR 2, with no concurrent
compilation: first phone scroll from 0 to 720 local pixels. G's observed
visible-to-image-submit gap for `layer-43-cell-0-0` was 69.7ms; H's was 28.7ms.
Peak ordinary rendering fell from 22.8ms to 14.2ms. Peak full ordinary frame was
essentially unchanged (35.5ms versus 35.6ms); H's preparation pass peaked at
2.9ms, with six cold drawings captured and the visible tile submitted in one
2.3ms pass. This is one short matched trace, not a sustained FPS benchmark.
Submission is not GPU completion and settled screenshots do not prove every
intermediate compositor frame was populated.

Fast phone movement/reversal, sibling-tablet handoff, horizontal shelves, and
390×844 mobile scrolling settle with populated images. Larger movements still
expose transient unready regions (about 140ms for one desktop handoff and 175ms
for a 1.7-page mobile jump). Final snapshots had no waiting-decoded or
recorded-but-unsubmitted images. Peak sampled desktop backing allocation was
25,502,290 pixels, below the unchanged 67,108,864 limit. Release browser checks
reported no new errors; static debug hosting predictably reports failed
designtime websocket connections.

A large main-page jump also reveals an empty feature gallery in both the
preserved G build and H; this is a pre-existing website issue, not fixed here.
Imported-settings traversal remains a separate follow-up. Remaining first-draw
and frame costs should be measured before changing warm margins or budgets.

Validation: 277 runtime unit tests, 8 scene-geometry tests, 21 viewport tests,
16 Web chassis tests, and 107 browser tests pass. The new real-Metal atomic
publication/fallback test also passes with GPU access. Three GPU/manual tests
remain ignored in the default unit run, plus one manual viewport test. Debug
and release website builds were run sequentially with `--libdev`; Wasm/Piet
checking, generated API docs, mdbook, and diff whitespace checks pass. No
compiler/manifest/baked-program representation changed. Physical Apple UI and
interactive Piet are not validated by the shared Metal renderer test.

## Checkpoint I — capacity-aware forward reserve

Zack approved using spare capacity for a deeper buffer on the active axis. The
shared policy now separates urgent velocity/latency prediction (250–750ms, up
to six local viewports) from lower-priority forward reserve (six viewports in
the last observed direction). Each exposed ancestor contributes an independent
axis trajectory; this does not multiply parent/child hypothetical padding.
Reversal changes candidate membership immediately. Moving reserves precede
recent pauses, which precede pauses older than 1.5 seconds. Idle reserve work
can finish filling opportunistically; it does not receive a completion deadline.
An unchanged sample after a long idle gap preserves remembered direction but
does not refresh urgent movement. Geometry changes/discontinuous movement still
invalidate activity history.

Admission first runs the existing globally visible-first and two-screenful
optional allowance. A second pass lets forward candidates borrow otherwise
unused capacity, leaving four surface slots and one screenful of backing pixels
out of the borrowing ceiling. The global 64-canvas / 67,108,864-pixel limits are
unchanged. Borrowing never evicts ordinary admitted safety coverage, increases
the frame-work budget, lowers density, or defers lifecycle mounting. The final
execution order is restored after the second pass. Diagnostic admission entries
now include owner/content rectangle and borrowed-pixel/ceiling data alongside
submitted readiness.

The first experimental release exposed a bad cutoff: stopping cold reserve
work at 1.5 seconds could abandon an incompletely filled buffer on slower runs.
The final policy downgrades paused work instead. Ready depth must be measured
after filling, not inferred from candidate distance or allocated canvas count.

Initial matched release spot checks, before removing that cutoff, at 1280×720,
DPR 2 with no compilation running: main page down 1.55 browser pages, phone
down 0.45/1.4/1.4 pages, pause, then phone down 3/up 2.2 pages and tablet down
1.5 pages. H recorded unready intervals of 132.8/149.5/96.0ms for the last three
moves; I recorded no gap for the forward jump, then 143.0/145.3ms for reversal
and sibling handoff. Full ordinary-frame maxima were 33.6/34.8ms, and sampled
frame p95 15.4/16.1ms (64/61 phase samples). I reached 60 assigned canvases and
58,727,709 backing pixels, versus H's 29 and 25,070,093 in that sequence.
This is an explicit memory-for-ready-coverage tradeoff, not an overall frame-time
speedup or a guarantee against cold direction changes.

Initial 390×844 / DPR 1 spot checks (phone down 0.7 then 1.7 pages) reduced the
larger jump's unready interval from 325.6 to 161.4ms, but I also had a 57.5ms gap
on the first smaller move. A later 2.5-page jump still exceeded the incompletely
filled buffer. These motivated the idle-filling correction above. These are
short native-wheel traces, not a continuous compositor capture or statistical
benchmark; submission still is not GPU completion.

Final release verification after the idle-filling correction repeated the
desktop sequence: no recorded unready interval on the three-page forward
jump, then 248.6ms on reversal and 263.6ms on tablet handoff. Those cold
transitions were worse than the H spot check, so this is not an across-the-board
scrolling improvement. Peak ordinary frame was 37.2ms (sample p95 15.8ms), and
peak admission was 60 canvases / 58,452,356 backing pixels. At 390×844 / DPR 1,
the final release recorded 40.4ms on the small phone move and 83.3ms on the
larger move, versus H's no recorded gap and 325.6ms respectively. Mobile peak
admission was 34 canvases / 6,513,916 backing pixels. All views settled with
populated images; the final release reported no browser errors.

More allocation still does not guarantee more submitted coverage. Idle traces
contain repeated settled-presentation cache misses, with some admitted forward
tiles remaining unsubmitted even when considerable capacity is unused. The
aggregate counter does not identify which first-draw/replay eligibility check
failed. Distinguishing these reasons is the next diagnostic step, together with
checking cold reversal/handoff preparation latency; raising capacity again is
not justified by these observations. The cooperative preparation allowance is
unchanged, and a single non-preemptible draw/submission can still overrun it.

Validation: 283 runtime unit tests (three ignored), 8 scene-geometry tests,
21 viewport tests (one ignored), 16 default-feature Web chassis tests, and
108 browser tests pass. Sequential debug and release website builds with
`--libdev`, Wasm checking with Piet/designtime, regenerated API reference,
mdbook, and whitespace checks pass. The designtime-enabled chassis unit-test
configuration retains an existing `PaxEngine::new_empty` harness mismatch;
only its default-feature unit configuration passed. Physical Apple UI and
interactive Piet remain untested. This checkpoint changes shared scheduling
policy and Web diagnostics, not manifest/baked-program representation or
lifecycle semantics. Human-facing scrolling guidance and authoring pain points
are updated alongside the implementation.

## Checkpoint J: failure isolation and animated clip reconciliation

The whole-page audit found a correctness failure independent of warm-region
size. The website mounts its calculator on approach with `paused=true`. The
calculator returned before initial layout, leaving its padded history Scroller
at width -28. Preparation rejected the entire snapshot, freezing gallery
admission until the calculator became visible and computed valid geometry.

Calculator pause now skips recurring work only after initial layout has settled;
resize still updates layout and its initial content while paused. Shared
preparation returns empty, diagnosed plans for malformed geometry/ancestry and
their descendants. Valid siblings continue, and the branch recovers from the
next valid observation. Rejected coverage never keeps a stale assignment alive.
Duplicate IDs or a malformed outer protocol remain transaction-level errors.
Version-2 Web admission consumes the prepared transaction directly, rather than
revalidating a second copy of the host measurements. Browser diagnostics report
branch rejection/recovery once per change.

The first release verification repeated main-page → phone Paxflix → gallery →
calculator. The gallery filled without the extra calculator-visibility scroll
that previously unblocked the page, with no preparation rejection. This did not
eliminate cold-render delays: a two-page horizontal gallery move recorded a
398.5ms visible-unready interval in that sample. A submitted tile is still not
proof of every image being present.

New opt-in `occlusion-work` counters also showed 34 full compositing rebuilds
from the hero's animated geometry clip during one startup, in addition to four
structural rebuilds. Clip membership is now indexed at structural reconciliation.
Changing an existing clip refreshes its dependent records and old/new native
overlaps, rather than traversing unrelated branches. Scroller geometry changes,
clip-boundary membership changes, and unsafe presentation domains retain the
conservative path. A 300-unrelated-native-node test checks bounded local work
and exact native-mask/presentation hashes against forced full reconciliation for
clip shrinking, empty coverage, movement, and rotation.

This checkpoint does not raise any surface/pixel/preparation allowance, change
raster density, reduce the examples, or introduce deferred lifecycle mounting.
Synchronous mounting of both 210-card Paxflix trees, asynchronous font settlement,
and application-generated content availability remain separate startup/scroll
costs. Initial timing samples were taken on a machine with varying concurrent
workload and are diagnostic observations, not an isolated benchmark.

Final release verification at 1280×720 / DPR 2 recorded four structural
rebuilds and zero animation-driven clip rebuilds during startup, versus the
four structural plus 34 clip rebuilds in the preceding release. Slow occlusion
samples (>20ms) fell from 38 to four. The first frame still took 10.64s, of
which 9.31s was `tick.proximity_effects`; removing clip rebuilds does not remove
the synchronous Paxflix-tree initialization. Those absolute times remain
machine-load-dependent observations, not a controlled performance comparison.

Main-page entry into Paxflix recorded a 307.3ms visible-unready interval;
a subsequent one-page phone scroll recorded 204.6ms. Gallery entry, two-page
horizontal turnover, and calculator entry all settled with populated visuals
without branch rejection or the former calculator-triggered global unblock.
The short ring buffer rolled over around gallery inspection, so missing gap
events there do not establish gap-free rendering. At 390×844 / DPR 1, responsive
hero, embedded phone, gallery turnover, and calculator were also exercised;
phone entry and a 0.75-page nested move recorded 94.2ms and 105.2ms unready
intervals. Browser errors/warnings and sampled branch rejection events were
absent on both viewports. Mobile entry still visibly showed cold content before
it filled. Surface submission counters do not measure image completeness or GPU
completion, and these spot checks are not a continuous scroll benchmark.

Validation: 284 runtime unit tests (three ignored), 8 scene-geometry tests,
22 viewport tests (one ignored), 39 calculator tests, 16 Web chassis tests,
and 109 browser tests pass (478 passing in total). Sequential debug and release
website builds with `--libdev`, generated API reference, bundled example-source
sync check, mdbook, and `git diff --check` pass. Physical Apple UI and interactive
Piet remain untested. No manifest/baked-program representation changed. The
release preview remains served at `http://127.0.0.1:50500/`.

## Checkpoint K: productive queue servicing and replay draining

The next audit separated admission, factory entry, GPU creation, and first
submission. In the preceding release, a jump to the calculator admitted page
tiles that did not start initialization for roughly 842ms and 1510ms, with
32 canvases assigned and 32 available. Temporary factory-entry/context-lock
traces in a debug build then showed the same class of stall: ordinary frames
continued, four initialization slots were occupied, but those futures had not
entered the factory. Once entered, the shared-context wait was immediate and
GPU surface creation took less than a millisecond in that sample. This does
not establish why the browser's microtask executor was delayed; it establishes
that admission and actual GPU creation were not the long stage in this case.

Queue servicing now polls a newly selected factory once immediately. A ready
shared-device addition is published without first reserving an asynchronous
slot; a genuine wait retains the exact future and passes it to the existing
platform executor with its live waker. Web limits immediate attempts to four
per queue service. Presentation also services pending initialization, and all
priority, cancellation, survivor, generation, and lease checks remain in place.
Native already used synchronous `block_on`; its whole-layer policy is unchanged.
Temporary console and factory/context-lock probes were removed after diagnosis.

Settled preparation no longer stops at its first selected batch snapshot.
After each complete submission it reselects against current priorities while
the existing 4ms cooperative allowance permits. Blocked layers are held aside
until the next pass, including when a newly initialized surface requeues one;
siblings can progress without repeatedly retrying the blocked head. The
allowance still governs capture/submission, not GPU creation or lifecycle work,
and an atomic unit can overrun it. No canvas/pixel/warm budget, density policy,
or component mounting gate changed.

In the debug calculator-jump comparison, visible-unready coverage fell from
677ms to 142ms. The selected new page and graph surfaces started initialization
within about 1ms of admission; their first submissions followed approximately
54ms later. This removes the reproduced pre-factory stall, not all cold drawing
or section-mounting latency. A later debug probe overlapped the release compiler
and had substantial frame hitches; it is not a clean performance baseline.

Four added tests cover repeated replay batches in one pass, current-priority
reselection with a blocked sibling, the unchanged time allowance, and retaining
a pending initializer across the immediate poll/executor handoff. Runtime unit,
scene-geometry, viewport, Web chassis, and browser-contract suites currently
pass: 443 tests, four ignored. Generated API docs, bundled example-source
checks, and mdbook also pass.

Release spot checks at 1280×720 / DPR 2 recorded no visible-unready transition
during a 400ms main-page probe, 400ms and 200ms embedded-phone probes, or a
subsequent two-page phone jump. These are short observations, not a guarantee
for arbitrary flings; the timed probes drive offsets through requestAnimationFrame
rather than reproducing native compositor momentum. A cold two-page horizontal
Paxflix jump recorded 57.8ms unready, and the calculator-entry jump recorded
130.6ms. The sampled page/graph surfaces started within 0.3/0.8ms of admission,
then reached first submission roughly 48–51ms later. Gallery turnover settled
with populated cards; sampled preparation passes submitted two batches apiece.

At 390×844, cold entry into the embedded demos visibly began empty and recorded
64.8ms unready; a horizontal phone-shelf move recorded 33.5ms. The phone's
400ms vertical probe ended with populated cards, but its start had already
left the trace ring by snapshot time, so that sample cannot establish gap-free
coverage for the entire probe. Browser warnings/errors were absent on both
viewport checks.

Frame pacing remains uneven (many probe intervals above 16ms). The desktop
startup sample reached its first frame at 3.48s, including about 3.00s in
`tick.proximity_effects`; synchronous section initialization and font/layout
settlement remain separate work. These machine-load-dependent runs are not a
controlled comparison with the preceding checkpoint. Readiness here means
submission, not GPU completion or proof of every image being present. Neither
canvas capacity nor warm-region size was increased to obtain these results.

Sequential debug/release website builds with `--libdev` pass. Physical Apple
UI and interactive Piet remain untested; no manifest/baked-program contract
changed. The release preview remains at `http://127.0.0.1:50500/`.
