# Debug Hot Logic Reloading
<!-- summary: Spec for debug-only hot swapping of user logic without restarting the host chassis. -->
<!-- tags: architecture, designtime, hot-reload, dynamic-linking, debug -->

## Problem Statement

Pax already supports designtime template reloads while the chassis stays alive.
That is enough for `.pax` edits, but it is not enough for Rust logic edits.

The goal of this spec is to define a debug-only path where:

- the chassis keeps running in watch mode
- `.pax` edits continue to patch in immediately
- Rust logic edits trigger a background rebuild
- a successful rebuild swaps in new logic without closing the running host
- durable application state survives the swap when possible

This is explicitly a designtime feature. Release builds do not participate in
this flow.

## Current Baseline

The current codebase already gives us part of the needed shape:

- Pax has an explicit runtime-kernel vs cartridge split.
- The recent release-cartridge refactor already established a meaningful split
  between program representation and logic module.
- The design server watches project files and forwards `.pax` edits into the
  running app.
- Designtime reload currently operates on manifest-driven `tree`, `subtree`, and
  `node` reload scopes.
- Apple targets already package the cartridge as a dynamic library.

There are also hard limits in the current implementation:

- Rust file changes are only surfaced as generic project-file-changed
  notifications; they do not trigger a logic reload path.
- Full userland reload recreates expanded nodes and does not define a durable
  state handoff contract.
- The release split is architectural and codegen-visible, but it is not yet a
  standalone host/plugin ABI that debug can hot-swap directly.
- On Apple targets, the app currently imports `PaxCartridge` directly and calls
  `pax_init`, so the loaded framework is still the cartridge itself rather than
  a stable host plus a swappable logic plugin.
- On the web, core Wasm modules are well supported, but component-model execution
  in browsers is still a transpiled/polyfilled path rather than a native browser
  loading path at the time of writing.

## Implementation Status

As of April 2026, Phase 1 exists on two chassis:

- **macOS**
  - a stable native host stays alive
  - Rust edits rebuild a fresh logic dylib under a unique path
  - the host remounts userland against the new cartridge image
- **web**
  - a stable JavaScript host stays alive
  - Rust edits rebuild a fresh JS + Wasm cartridge bundle under a unique served
    path
  - the host remounts a fresh Wasm app instance in the same tab

The important point is that the semantic reload boundary is shared even though
the artifact mechanics differ:

- the host survives
- `.pax` edits continue using the lighter designtime manifest-update path
- logic edits produce a fresh artifact
- the host remounts userland from that artifact

What still does **not** exist is typed state transfer. Both chassis are Phase 1
remount flows today.

Web Phase 1 still has hardening follow-up work tracked in `PAX-889`:

- preserve the last known good mounted app if a new reload artifact fails during
  attach/init
- prune staged `__reloads__` artifacts during long sessions
- add automated smoke coverage for web hot reload

## Decision Summary

The recommended direction is:

1. Reuse the release-cartridge conceptual split rather than inventing a second
   debug-only model.
2. Keep the chassis and runtime kernel alive as a stable debug host.
3. Split the current debug cartridge into:
   - a host-loaded logic module
   - stable host-side ABI shims that do not change on every user edit
4. Make hot logic reload a host-driven remount operation:
   - quiesce the frame loop
   - snapshot durable state
   - load the new logic module
   - create a fresh traverser from the new module
   - remount userland
   - restore compatible state
   - resume ticking
5. Ship this on one chassis first.

Recommended first chassis: `macOS` designtime.

## Goals

- Keep the window, render surfaces, dev session, and host process alive during
  Rust edits.
- Preserve the design-time/watch-mode workflow already used for `.pax` edits.
- Keep the hot-reload boundary compatible with future non-Rust logic runtimes.
- Prefer explicit, typed state transfer over ad hoc object reuse.
- Keep the last known good logic module running when rebuilds fail.

## Non-Goals

- Release-build hot swapping.
- Perfect recovery from every panic, infinite loop, or UB case in v1.
- Simultaneous first-class support for all chassis.
- Reusing the currently loaded monolithic cartridge image in place.
- Native browser component-model loading as a requirement for the first version.

## Proposed Architecture

### 1. Stable Debug Host

The running process should be divided into two layers:

- **Debug host**
  - owns the chassis window/tab
  - owns the runtime kernel
  - owns render surfaces and dev tooling connections
  - owns the file-watch and background-build loop
  - owns the currently active logic module handle
- **Logic module**
  - owns generated component descriptors
  - owns handler implementations
  - owns the definition-to-instance traverser implementation
  - may change on every userland Rust edit

The important design point is that the host must not require relinking when the
user edits application logic.

### Relationship to Release Cartridge

The release-cartridge refactor already provides the right semantic model:

- program representation is distinct from logic module
- descriptor-driven instantiation already exists
- future multi-language logic can target the same runtime kernel shape

What release does **not** yet provide is a ready-made hot-reload ABI:

- the final shipped artifact is still built as one loadable module per target
- manifest construction and traverser construction are separate concepts, but
  they are still compiled into the same final binary image

So the debug hot-reload work should:

- reuse the release boundary as the source of truth
- avoid inventing a second incompatible notion of "logic module"
- avoid changing release behavior unless shared compiler/runtime schema work
  truly requires it

In practice, release should mostly need auditing rather than new behavior. If we
add descriptor tables, state schemas, or other compiler/runtime boundary data
for debug hot reload, we should check whether the same schema belongs in
`program_ir`, `binary`, or `rust_manifest` for release. The answer will often be
"no" for designtime-only metadata.

### 2. New Debug ABI

The current generated cartridge already exposes descriptor tables and a
`DefinitionToInstanceTraverser`. Hot swapping needs that surface to become a
stable plugin ABI.

Use a versioned C ABI from day one.

That has some cost:

- more manual ownership and lifetime bookkeeping
- less ergonomic type signatures than a Rust-only boundary
- explicit versioning discipline

But those costs are acceptable here, because the long-term target is
multi-language logic modules and a Rust-only ABI would become migration debt
almost immediately.

The host-facing ABI should expose at least:

- logic module version / build id
- manifest or program representation handle
- component descriptor registry
- traverser factory
- optional debug metadata and diagnostics hooks
- state schema descriptors

For Rust-generated logic modules, this ABI can be emitted by codegen. Future
TypeScript or Python logic runtimes should be able to provide the same logical
contract without requiring the host to know their implementation details.

### 3. State Transfer Contract

The host should treat reload as a remount with state handoff, not as pointer
reuse across dynamic library boundaries.

Durable state is:

- `Property<T>`-backed application state that is intentionally part of the
  reactive model
- any state mirrored into explicit runtime properties before reload

Non-durable state is:

- raw Rust object identity
- hidden state inside native controls that is not mirrored into `Property`
- in-flight handler stacks
- transient render caches

To support durable handoff, codegen should emit per-component state descriptors
for reloadable fields. Each descriptor should include:

- a stable field name
- a way to read the current value into a portable payload
- a way to write a restored value back
- the field's default behavior when no prior value is available

The portable payload should use Pax-owned value types, not Rust-specific memory
layouts. `PaxValue` is the natural starting point.

Timeline and animation continuity should fit the same mechanism. Where timeline
playheads or transition playheads are already represented as `Property` state,
they should be restorable via the same typed snapshot path rather than treated
as a special case. That said, full-fidelity animation continuity belongs in
Phase 2 with the rest of state transfer.

### 4. Identity Matching Rules

State restore needs a stable notion of "the same place" across reloads.

The recommended identity key is:

- containing component type id
- template-node identity / expansion path
- component state field name

On restore:

- if the identity still exists and the type is compatible, restore the value
- if the field exists but the type changed, fall back to the new default
- if the field no longer exists, drop the prior value
- if a new field appears, initialize from its default

This makes tree-shape drift explicit and predictable instead of relying on
incidental object reuse.

### 5. Hot Reload Loop

For Rust edits in watch mode:

1. The design server detects a Rust change.
2. The design server debounces changes and starts a background build.
3. If the build fails:
   - keep running the last known good module
   - surface diagnostics in the dev console / dev session
4. If the build succeeds:
   - the design server publishes reload-ready artifact metadata
   - stage the new logic artifact under a unique build-id path
   - pause or quiesce ticking
   - snapshot durable state
   - load the new logic module
   - instantiate a new traverser from that module
   - remount userland from the new traverser
   - restore compatible state
   - resume ticking

Never overwrite the currently loaded dynamic library in place. Always load a new
uniquely named artifact so the operating system loader does not hand back the
previous image.

The design server should remain the build authority because it already owns
local filesystem access. That avoids introducing a privileged-agent roundtrip
just to watch files or invoke builds. Artifact delivery can vary by chassis:

- native targets can receive a local artifact path or equivalent metadata
- web targets can receive a served URL backed by the design server

The source of truth should stay on disk; transport to the chassis can be path,
URL, or other lightweight metadata depending on target.

### 6. Failure Model

V1 should be resilient in the common cases:

- **Compile error**
  - current app keeps running
  - error is reported out-of-band
- **Load/link error**
  - keep the previous logic module active
  - report reload failure
- **Restore mismatch**
  - restore what is compatible
  - default the rest

Some failures need a later phase:

- infinite loops in the main UI thread
- panics that cross ABI boundaries without containment
- logic that corrupts shared host state

The long-term answer for those cases is stronger isolation, potentially a worker
thread or subprocess boundary. That is not required for the first chassis.

## Chassis Strategy

### macOS: First Chassis

macOS is the best first chassis because:

- the current debug build already produces a dynamic library
- the current design workflow already has a persistent native host window
- file watching and dev-session plumbing already exist
- dynamic loading is practical in a local desktop dev environment

However, the current packaging is still too monolithic. Today the Swift host
imports `PaxCartridge` and calls `pax_init` directly. For hot logic reload, that
should be refactored into:

- a stable host-side framework or shim loaded once
- a separately versioned logic dylib loaded by path at runtime

The host shim should own the runtime kernel and hold function pointers or a
vtable returned by the logic dylib.

### Web: Phase 1 Follow-On Chassis

Web no longer blocks the first implementation; it now has its own Phase 1 path.

At the time of writing:

- browsers natively run core Wasm modules well
- browser-side component-model execution is still a transpiled or polyfilled
  workflow
- Pax web hot reload is implemented today as a stable JS host remounting a
  freshly built JS + Wasm app artifact

So the web strategy should remain:

- keep current `.pax` designtime reload behavior for template-only edits
- use the stable JS host as the durable boundary for Rust/code edits
- treat future state restore as Phase 2 on top of that host boundary
- evaluate component-model or richer linking experiments later, not as a Phase 1
  prerequisite

The shipped web Phase 1 should continue to avoid native browser component-model
support as a dependency.

### iOS: Later

iOS should reuse the same host/logic ABI design, but it should not drive the
first rollout. Platform policy, code-signing, and device-vs-simulator behavior
make it a worse first proving ground than macOS.

## Compiler and Codegen Changes

Hot logic reload requires deliberate compiler output changes:

- emit a stable debug ABI for logic modules
- emit reloadable state descriptors for component fields
- stage logic artifacts under unique per-build paths
- separate host-shim artifacts from swap-target artifacts
- share boundary definitions with the release-cartridge architecture where
  possible, while keeping designtime-only metadata out of release payloads by
  default

For web, the "swap-target artifact" is currently a rebuilt JS + Wasm cartridge
served under a unique URL, not a lower-level dynamically linked Wasm side
module. That is acceptable for Phase 1 as long as the host-side reload protocol
remains artifact-oriented rather than hard-coding native dylib assumptions.

This is debug-only metadata and should not leak into release artifacts unless
the runtime truly needs it there.

## Designtime Host and Kernel Integration

Most of the orchestration for hot logic reload is a designtime concern, not a
general runtime concern. We should keep as much of it as possible in the
designtime host layer for footprint reasons.

Designtime-side responsibilities:

- snapshot reloadable state from the mounted userland tree
- coordinate remount from a newly loaded traverser
- restore state after remount
- invalidate non-durable caches cleanly across reload boundaries
- own reload bookkeeping, error handling, and rollback policy

The runtime kernel should expose only the narrow hooks needed to support that:

- mount and remount userland roots
- expose enough traversal/state surface for snapshot and restore
- invalidate runtime caches safely after a remount

The current `full_reload_userland` path is a useful control point, but it is not
yet sufficient as the final mechanism because it assumes an in-process manifest
reload, not a module swap with durable state handoff.

## Design-Server Changes

The design server should evolve from "template patch transport" into a broader
debug coordinator.

New responsibilities:

- debounce Rust edits
- launch background builds
- surface build diagnostics without killing the running host
- publish reload-ready artifact metadata to the host
- keep the dev session registered during failed and successful reload attempts
- remain the authoritative filesystem-facing coordinator for watch and build
  behavior

The design-server-to-host payload should stay artifact-oriented. Today that
means build id + artifact kind + artifact location. Future work may need to add
versioning, integrity, or richer capability metadata, but that should extend the
same envelope instead of forking native and web into unrelated protocols.

`.pax` template edits should continue using the lighter-weight manifest reload
path when possible. Hot logic reload is the slower path used only when the logic
artifact changes.

## Phased Rollout

### Phase 1

- macOS only
- debug-only
- background rebuilds for Rust edits
- load new logic dylib from unique path
- full userland remount
- no durable state restore requirement

This proves the host/logic split and the loader mechanics.

### Phase 2

- add typed state snapshot and restore
- preserve userland `Property` state across compatible reloads
- preserve compatible timeline / animation playhead state where it is expressed
  through reloadable properties
- improve diagnostics and rollback behavior

### Phase 3

- stronger fault isolation
- web-specific state restore and hardening
- additional language runtimes that implement the same ABI

## Settled Decisions

- Phase 1 does not require durable state restore. A successful in-process logic
  swap is sufficient for the first milestone.
- Use a versioned C ABI from day one.
- Timeline and animation continuity should be treated as part of the general
  durable-state problem and land in Phase 2.
- The design server should own the watch/build loop because it already has
  direct filesystem access.

## Cross-Runtime Notes

This work moves Pax closer to multi-runtime support in one specific way: the
reload coordinator now thinks in terms of "fresh app artifact for a stable host"
instead of only "replace this native dylib path."

That is useful, but it is not the same thing as a language-neutral runtime ABI.
Future JS/TS or Python work still needs the real logic-module contract:

- component descriptor registry contract
- traverser factory contract
- handler dispatch contract
- state snapshot / restore schema
- value marshaling format, likely centered on Pax-owned value types

The web path is especially important for future agents to read correctly:

- the durable boundary on native is the host process plus stable chassis/kernel
  around a swappable cartridge image
- the durable boundary on web is the JS host, not the live Wasm instance
- those are different artifact mechanics, but they should keep one reload state
  machine and one host-facing protocol

So future cross-runtime work should prefer extending the shared artifact/reload
contract rather than introducing separate native-vs-web concepts of reload.

## Recommendation

Proceed with a macOS-first debug implementation that splits the current
monolithic debug cartridge into a stable host shim plus a swappable logic dylib,
reusing the release-cartridge logic-module boundary as the architectural source
of truth. Keep the reload coordinator in designtime, expose a C ABI from the
start, and defer full state continuity to Phase 2.
