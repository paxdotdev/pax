# engine::surface_preparation
<!-- summary: Clip-aware, time-coupled exposure planning. All rectangles use presentation -->
<!-- tags: api, pax-runtime -->

Clip-aware, time-coupled exposure planning. All rectangles use presentation
coordinates until the final inverse mapping into owner content space.

## Structs
### `PreparationHost`
A host and the conservative clip between it and its nearest scrolling parent.
The intermediate clip moves with the parent's content, not its viewport.

#### Properties
##### `geometry`
Type: [`HostObservation`](../../../../api/internal/pax-runtime/engine/surface_scheduling.md#hostobservation)

##### `content`
Type: [`f64`; 2]

##### `intermediate_clip`
Type: `Option`<[`CoverageRect`](../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect)>

---

### `PreparationLatency`
Conservative end-to-end preparation latency, sampled once per current
assignment. Hosts supply monotonic durations from assignment to observed
submission, including queueing and first drawing, not just GPU allocation.

---

### `PreparationPolicy`
Bounded prediction, independent of chassis event types. The horizon is an
initial calibration value, not a guarantee of content preparation latency.

#### Properties
##### `horizon_ms`
Type: `f64`

##### `max_viewports`
Type: `f64`

##### `safety_viewports`
Type: `f64`

Independent first-interaction coverage on currently exposed scroll axes.

##### `reversal_viewports`
Type: `f64`

Cold-capable coverage opposite the last movement on exposed axes.
Admission reserves this before forward speculation; execution remains
visible-first, then predicted incoming work.

##### `retention_viewports`
Type: `f64`

Spatial hysteresis for already submitted assignments; never starts work.

##### `reserve_viewports`
Type: `f64`

Opportunistic forward distance, admitted only when capacity permits.

##### `reserve_tail_ms`
Type: `f64`

Recent pauses precede older reserve work; this is not an expiry deadline.

---

### `PreparedHost`
Current and anticipated exposure in owner content coordinates. The private
clip trajectory lets admission reject corners introduced by a swept AABB.

#### Properties
##### `unavailable_reason`
Type: `Option`<&'`static` `str`>

A malformed host (or ancestor) is unavailable for this observation only.
Its empty plan must not preserve stale coverage or block valid siblings.

##### `visible`
Type: [`CoverageRect`](../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect)

##### `required`
Type: [`CoverageRect`](../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect)

##### `prepared`
Type: [`CoverageRect`](../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect)

Candidate enumeration bounds, including retain-only space. Admission
must call `rank`; this AABB alone does not authorize cold preparation.

#### Implementations
##### `deadline`
<pre><code class="api-signature language-rust ignore">pub fn deadline(&amp;self, projected_tile: <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect">CoverageRect</a>) -&gt; Option&lt;f64&gt;</code></pre>

Earliest predicted intersection, in milliseconds; `None` is outside the
trajectory. A tile already intersecting the current clips has deadline 0.

##### `rank`
<pre><code class="api-signature language-rust ignore">pub fn rank(&amp;self, tile: <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect">CoverageRect</a>, retained_ready: bool) -&gt; (<a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coverage">Coverage</a>, f64)</code></pre>

Exact trajectory admission, not the enclosing candidate AABB. Retention
is a lower-priority reuse opportunity, never permission to initialize a
cold tile or to displace required/forward/safety coverage.

## Functions
### `prepare_hosts`
<pre><code class="api-signature language-rust ignore">pub fn prepare_hosts(hosts: &amp;BTreeMap&lt;u32, <a href="../../../../api/internal/pax-runtime/engine/surface_preparation.md#preparationhost">PreparationHost</a>&gt;, viewport: <a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#coveragerect">CoverageRect</a>, activity: &amp;<a href="../../../../api/internal/pax-runtime/engine/surface_scheduling.md#scrollactivitytracker">ScrollActivityTracker</a>, now: f64, policy: <a href="../../../../api/internal/pax-runtime/engine/surface_preparation.md#preparationpolicy">PreparationPolicy</a>) -&gt; Result&lt;BTreeMap&lt;u32, <a href="../../../../api/internal/pax-runtime/engine/surface_preparation.md#preparedhost">PreparedHost</a>&gt;, &amp;&#39;static str&gt;</code></pre>

Plan exposure along one shared time axis. Actual ancestor/child movement is
composed. Safety and retention are separate one-axis alternatives for each
currently exposed host, not a Cartesian product of nested warm windows.
An offscreen shelf inherits parent exposure, not independent horizontal pad.
Invalid geometry or ancestry quarantines only that branch for the current
observation. Valid siblings continue; corrected observations recover normally.
