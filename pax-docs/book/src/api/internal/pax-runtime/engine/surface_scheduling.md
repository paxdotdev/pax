# engine::surface_scheduling
<!-- summary: Platform-independent surface admission and observed scroll activity. -->
<!-- tags: api, pax-runtime -->

Platform-independent surface admission and observed scroll activity.

Chassis supply presentation observations, physical costs, and an explicit
monotonic clock. This module neither allocates surfaces nor owns GPU leases.

## Structs
### `CoverageRect`
Axis-aligned conservative bounds in one explicitly chosen coordinate space.

#### Properties
##### `left`
Type: `f64`

##### `top`
Type: `f64`

##### `right`
Type: `f64`

##### `bottom`
Type: `f64`

---

### `HostObservation`
A chassis observation, not a request to allocate a surface for this host.

#### Properties
##### `id`
Type: `u32`

##### `parent`
Type: `Option`<`u32`>

##### `bounds`
Type: [`CoverageRect`](../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect)

##### `visible`
Type: [`CoverageRect`](../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect)

##### `viewport`
Type: [`f64`; 2]

##### `offset`
Type: [`f64`; 2]

##### `basis`
Type: [`f64`; 4]

##### `active`
Type: `bool`

---

### `ResidencyPolicy`
Physical residency policy. The viewport allowance limits speculation, not
mandatory composition. Chassis report unreleased backings/reservations that
are not already represented by candidates (including unknown preserved plans).

#### Properties
##### `backing_pixels`
Type: `u64`

##### `optional_viewports`
Type: `f64`

---

### `ScrollActivity`
Movement in local content units per millisecond. Direction is unsmoothed.

#### Properties
##### `direction`
Type: [`f64`; 2]

##### `speed`
Type: [`f64`; 2]

---

### `ScrollActivityTracker`
Per-engine activity history, independent of input devices and platform focus.

#### Implementations
##### `observe`
<pre><code class="api-signature language-rust ignore">pub fn observe(&amp;mut self, id: u32, offset: [f64; 2], now: f64, geometry: &amp;str)</code></pre>

Geometry changes and movement across long sampling gaps are not velocity
evidence. An unchanged idle observation preserves the last direction for
opportunistic coverage, without refreshing its movement timestamp.

##### `recent`
<pre><code class="api-signature language-rust ignore">pub fn recent(&amp;self, id: u32, now: f64, tail_ms: f64) -&gt; <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#scrollactivity">ScrollActivity</a></code></pre>

Last observed local direction within a bounded tail. A longer tail may
retain opportunistic coverage, but must not extend urgent scroll intent.

---

### `SurfaceBudget`
Available capacity after the adapter has accounted for unreleased resources.

#### Properties
##### `surfaces`
Type: `usize`

##### `backing_pixels`
Type: `u64`

##### `optional_pixels`
Type: `u64`

##### `borrowing_limit`
Type: `Option`<(`usize`, `u64`)>

Optional second-pass ceiling (total surfaces, total backing pixels).
Bounded reverse coverage borrows before forward coverage, after ordinary
admission finishes. This ceiling preserves headroom for visible demand.

#### Implementations
##### `count_only`
<pre><code class="api-signature language-rust ignore">pub fn count_only(surfaces: usize) -&gt; Self</code></pre>

Compatibility policy for adopting the shared scheduler before calibration.

---

### `SurfaceCandidate`
#### Properties
##### `distance`
Type: `f64`

Exposure time for directional preparation, distance for stationary work.

---

### `SurfaceSchedule`
#### Properties
##### `admitted`
Type: `Vec`<`usize`>

Indices into the immutable input snapshot, in preparation order.

##### `borrowed_reverse_pixels`
Type: `u64`

Reverse coverage admitted above the ordinary optional allowance.

## Enums
### `Coverage`
Lexicographic coverage classes; these are not visual z-order.

#### Variants
##### `Visible`
##### `MovingAhead`
##### `ReversalNear`
One-axis reverse coverage, reserved before forward speculation but
prepared after current incoming work. Cold tiles are eligible too.

##### `ReadyTrailing`
##### `StationaryNear`
##### `ForwardReserve`
Deeper coverage on a recently moving axis; yields to all near coverage.

##### `FartherPreparation`
##### `OutsidePreparation`
## Functions
### `project_surface`
<pre><code class="api-signature language-rust ignore">pub fn project_surface(rect: <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect">CoverageRect</a>, host: &amp;<a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#hostobservation">HostObservation</a>) -&gt; <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect">CoverageRect</a></code></pre>

Project a content-local rectangle using its affine basis and measured host
bounds. Browser scroll translation never becomes a scale estimate.

---

### `projected_density`
<pre><code class="api-signature language-rust ignore">pub fn projected_density(basis: [f64; 4], display_scale: f64) -&gt; Option&lt;[f64; 2]&gt;</code></pre>

Raster density for the local affine basis, not the rotated bounding box.
Orthogonal axes retain their separate scales; shear uses a conservative
singular-value bound. Pure presentation rotation cannot change the result.

---

### `rank_surface`
<pre><code class="api-signature language-rust ignore">pub fn rank_surface(candidate: &amp;mut <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#surfacecandidate">SurfaceCandidate</a>, rect: <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect">CoverageRect</a>, owner: Option&lt;u32&gt;, hosts: &amp;BTreeMap&lt;u32, <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#hostobservation">HostObservation</a>&gt;, activity: &amp;<a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#scrollactivitytracker">ScrollActivityTracker</a>, now: f64)</code></pre>

Rank inherited exposure using actual ancestor movement. Child screen motion
caused by its parent must not claim independent child-axis scroll intent.

---

### `schedule_surfaces`
<pre><code class="api-signature language-rust ignore">pub fn schedule_surfaces(candidates: &amp;[<a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#surfacecandidate">SurfaceCandidate</a>], budget: <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#surfacebudget">SurfaceBudget</a>) -&gt; <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#surfaceschedule">SurfaceSchedule</a></code></pre>

Visible coverage precedes all speculation. Reverse-buffer residency is
funded next, before forward work can consume the optional allowance.
Execution still follows coverage/deadline order. Equal-ranked branches
interleave; visible survivors win genuine overload without rotating evictions.
