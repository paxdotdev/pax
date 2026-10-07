# engine::surface_work
<!-- summary: Cooperative admission for drawing capture and surface submission. A drawing -->
<!-- tags: api, pax-runtime -->

Cooperative admission for drawing capture and surface submission. A drawing
capture or GPU submission is atomic; the allowance limits starting more work.

## Structs
### `SurfaceWorkBudget`
CPU-time allowance for preparing settled drawings between application ticks.
The cost estimate adapts across passes; at least one eligible job can make progress.
