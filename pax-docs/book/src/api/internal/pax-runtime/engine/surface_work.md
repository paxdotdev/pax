# engine::surface_work
<!-- summary: Cooperative admission for drawing capture and surface submission. A drawing -->
<!-- tags: api, pax-runtime -->

Cooperative admission for drawing capture and surface submission. A drawing
capture or GPU submission is atomic; the allowance limits starting more work.

## Structs
### `PreparationDiagnostics`
Fixed-stage, opt-in CPU attribution for cold preparation. Image replay is a
subphase of replay; neither submission nor presentation measures GPU completion.

#### Properties
##### `stages`
Type: `BTreeMap`<&'`static` `str`, `PreparationStageTiming`>

##### `targeted_surfaces`
Type: `usize`

##### `additional_surfaces`
Type: `usize`

Already-presented dirty surfaces included alongside the selected targets.

##### `image_uploads`
Type: `usize`

##### `image_upload_bytes`
Type: `usize`

---

### `RenderWorkReport`
One synchronous service result. `pending` requests another host task;
`blocked` waits for a normal settlement/completion opportunity, not a busy loop.
CPU times include atomic overruns and do not measure GPU completion.

#### Properties
##### `pending`
Type: `bool`

##### `blocked`
Type: `bool`

##### `blocker`
Type: `Option`<&'`static` `str`>

##### `allowance_exhausted`
Type: `bool`

##### `ordinary_layers`
Type: `usize`

##### `ordinary_drawn_layers`
Type: `usize`

##### `ordinary_flush_only_layers`
Type: `usize`

##### `atomic_fallbacks`
Type: `usize`

##### `units`
Type: `usize`

##### `elapsed_ms`
Type: `f64`

##### `setup_ms`
Type: `f64`

Setup and ordinary drawing are measured, not charged to preparation.

##### `ordinary_ms`
Type: `f64`

##### `ordinary_drawn_ms`
Type: `f64`

##### `ordinary_flush_only_ms`
Type: `f64`

##### `max_unit_ms`
Type: `f64`

##### `preparation`
Type: [`SurfaceWorkReport`](../../../../api/internal/pax-runtime/engine/surface_work.md#surfaceworkreport)

##### `layer_reconciliation`
Type: `Vec`<(`usize`, `bool`, `usize`)>

Opt-in setup attribution: (layer, dirty before reconciliation, pending nodes).
At most 32 layers per service; empty when runtime diagnostics are disabled.

##### `layer_draws`
Type: `Vec`<(`usize`, `Option`<`i32`>, `usize`, `usize`)>

Opt-in draw attribution: (layer, current priority, selected nodes, still dirty).
At most 32 units per service, without retaining scene nodes between calls.

---

### `RenderWorkState`
Per-display-frame accounting for cooperative rendering. This is not a scene
snapshot or work queue: pending nodes, layers and surfaces remain authoritative
in the runtime/backend and are selected again at every service boundary.

#### Implementations
##### `begin`
<pre><code class="api-signature language-rust ignore">pub fn begin(&amp;mut self, epoch: u32)</code></pre>

Begin drawing after a settled tick. Multiple ticks/completions in one
display epoch share preparation accounting, including its first-job grant.

##### `continue_epoch`
<pre><code class="api-signature language-rust ignore">pub fn continue_epoch(&amp;mut self, epoch: u32)</code></pre>

Continue the same settled scene in a new display epoch. Preparation gets
a fresh allowance, but expensive completed setup must not restart merely
because it consumed an entire previous slice.

---

### `SurfaceWorkBudget`
CPU-time allowance for preparing settled drawings between application ticks.
Actual elapsed CPU time limits further work; at least one eligible job can
make progress. Unrelated expensive jobs do not predict the next job's cost.

---

### `SurfaceWorkReport`
#### Properties
##### `urgent_units`
Type: `usize`

Visible initialization/submission units serviced in this display epoch.
Diagnostic only: starting initialization is not presented content.

##### `diagnostics`
Type: `Option`<[`PreparationDiagnostics`](../../../../api/internal/pax-runtime/engine/surface_work.md#preparationdiagnostics)>

Absent unless runtime diagnostics are enabled. Fixed-size stage aggregates,
never per-node or per-command logs; no effect on admission or readiness.
