# engine::surface_work
<!-- summary: Cooperative admission for drawing capture and surface submission. A drawing -->
<!-- tags: api, pax-runtime -->

Cooperative admission for drawing capture and surface submission. A drawing
capture or GPU submission is atomic; the allowance limits starting more work.

## Structs
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

##### `atomic_fallbacks`
Type: `usize`

##### `units`
Type: `usize`

##### `elapsed_ms`
Type: `f64`

##### `max_unit_ms`
Type: `f64`

##### `preparation`
Type: `SurfaceWorkReport`

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

---

### `SurfaceWorkBudget`
CPU-time allowance for preparing settled drawings between application ticks.
The cost estimate adapts across passes; at least one eligible job can make progress.
