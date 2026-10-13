# engine::startup
<!-- summary: Experimental, host-serviced initial mounting. A continuation owns identities -->
<!-- tags: api, pax-runtime -->

Experimental, host-serviced initial mounting. A continuation owns identities
and phase cursors only; no property evaluation or borrow crosses a host yield.

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

##### `units`
Type: `usize`

##### `mounted`
Type: `usize`

Mount continuations attempted, including stale entries discarded on service.

##### `effects`
Type: `usize`

##### `elapsed_ms`
Type: `f64`

##### `longest_unit_ms`
Type: `f64`

##### `longest_unit`
Type: &'`static` `str`
