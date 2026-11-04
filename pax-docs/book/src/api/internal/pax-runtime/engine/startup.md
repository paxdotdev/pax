# engine::startup
<!-- summary: Experimental, host-serviced initial mounting. A continuation owns identities -->
<!-- tags: api, pax-runtime -->

Experimental, host-serviced initial mounting. A continuation owns identities
and phase cursors only; no property evaluation or borrow crosses a host yield.

## Submodules
- [engine::startup::plan](startup/plan.md)

## Structs
### `StartupInput`
Frozen recipients for application input captured while startup is pending.
A remount of the same node ID is a different input lifetime.

---

### `StartupInputGuard`
Restores the input dispatch context even if a callback unwinds.

---

### `StartupReport`
CPU-side startup service counters. A unit may exceed the requested budget:
arbitrary Rust callbacks and structural dependency groups are indivisible.

#### Properties
##### `pending`
Type: `bool`

##### `ready_to_present`
Type: `bool`

##### `deferred_roots`
Type: `usize`

##### `units`
Type: `usize`

##### `mounted`
Type: `usize`

Mount continuations attempted, including deferred or discarded stale entries.

##### `effects`
Type: `usize`

Queued reactive evaluations, including both effects and cutoff propagation.

##### `peak_queued_effects`
Type: `usize`

Largest queued effect/cutoff count observed during this service call.
Includes entries already settled by synchronous reads but not yet popped.

##### `elapsed_ms`
Type: `f64`

##### `longest_unit_ms`
Type: `f64`

##### `longest_unit`
Type: &'`static` `str`
