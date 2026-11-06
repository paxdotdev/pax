# work_profile
<!-- summary: Explicit-window attribution for reactive effects, callbacks, and retained rendering. -->
<!-- tags: api, pax-runtime -->

Explicit-window attribution for reactive effects, callbacks, and retained rendering.

## Structs
### `LifecycleWork`
Inclusive CPU work for one lifecycle target and phase. Context
construction, handler execution, and teardown are disjoint; dependency reads
belong to their caller.

#### Properties
##### `calls`
Type: `usize`

##### `context_ms`
Type: `f64`

##### `handler_ms`
Type: `f64`

##### `teardown_ms`
Type: `f64`

##### `max_ms`
Type: `f64`

---

### `LifecycleWorkRow`
A callback target, with source identity when it is still mounted at capture end.

#### Properties
##### `node`
Type: `u32`

##### `phase`
Type: &'`static` `str`

##### `identity`
Type: `Option`<[`NodeInspection`](../../../api/internal/pax-runtime/node_inspection.md#nodeinspection)>

##### `work`
Type: [`LifecycleWork`](../../../api/internal/pax-runtime/work_profile.md#lifecyclework)

---

### `RenderWorkCounts`
Aggregated backend work within an explicit capture. Pixel counts estimate
processed area, not elapsed GPU time; nested spans must not be summed.

#### Properties
##### `count`
Type: `u64`

##### `pixels`
Type: `u64`

##### `cpu_ms`
Type: `f64`

---

### `RenderWorkRow`
Backend work attributed to a physical surface and, when known, an engine node.

#### Properties
##### `layer`
Type: `usize`

##### `surface`
Type: `usize`

##### `node`
Type: `Option`<`u32`>

##### `operation`
Type: &'`static` `str`

##### `work`
Type: [`RenderWorkCounts`](../../../api/internal/pax-runtime/work_profile.md#renderworkcounts)

---

### `ScopedWork`
Inclusive timing for a named runtime operation, including nested dependency reads.

#### Properties
##### `calls`
Type: `usize`

##### `inclusive_ms`
Type: `f64`

---

### `WorkProfileReport`
Explicit-window wall-clock attribution. These inclusive spans overlap tick
phase timings and must not be added to them. Effects are queued evaluations;
nested dependency reads are included but not counted separately.

#### Properties
##### `reactive_totals`
Type: `ReactiveWorkCounts`

##### `effect_categories`
Type: `Vec`<(`String`, `ReactiveWorkCounts`)>

##### `slowest_properties`
Type: `Vec`<(`String`, `ReactiveWorkCounts`)>

Up to 128 property labels ranked by inclusive evaluator time.

##### `omitted_properties`
Type: `usize`

##### `lifecycle`
Type: `Vec`<[`LifecycleWorkRow`](../../../api/internal/pax-runtime/work_profile.md#lifecycleworkrow)>

All lifecycle targets, ranked by context, handler, and teardown time.

##### `scopes`
Type: `Vec`<(`u32`, &'`static` `str`, [`ScopedWork`](../../../api/internal/pax-runtime/work_profile.md#scopedwork))>

Nested operation timings; these overlap queued effects and lifecycle spans.

##### `rendering`
Type: `Vec`<[`RenderWorkRow`](../../../api/internal/pax-runtime/work_profile.md#renderworkrow)>

Geometry, capture reuse, surface encoding and lighting invalidations.

##### `nodes`
Type: `Vec`<[`NodeInspection`](../../../api/internal/pax-runtime/node_inspection.md#nodeinspection)>

Source identities for named scopes and backend observations. Missing
entries have unmounted before the capture ended.

---

### `WorkProfileScope`
An opt-in timing scope. Dropping it records work only while capture is active.
