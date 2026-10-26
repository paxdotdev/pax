# Deferred whole-component probe

This web-only [PAX-1034](https://linear.app/paxdev/issue/PAX-1034) fixture loads a
complete small counter component into an already running Pax scene. Its deferred
WASM contains the template, expressions, selector settings, event/lifecycle
bindings, property schema/defaults, handler descriptors, and Rust state/event
logic. The equivalent `monolithic` control embeds the same package and logic in
the base application. Research results and design proposals live in the ticket.

The base retains the shared Pax runtime, standard primitives, a fixture adapter,
and the parent's typed import contract (`count`, `step`, `warm`). It has no
counter template, counter labels, local-property schema, or counter-specific
handler routes. This supersedes the earlier logic-only version of this fixture.
It is an executable experiment, not a public package ABI or production loader.

## How the package works

`component/counter.pax` is compiled by the existing Pax parser into the existing
binary manifest format. `component/descriptor.json` supplies the prototype's
explicit scalar property and handler ABI. `packer` checks the manifest's binary
roundtrip, including expressions, settings, and event routes. `child.rs` embeds
both metadata payloads in its WASM alongside `logic.rs`; they require no separate
network requests. There are no assets in this component.

The loader fetches/instantiates the module and copies its definition into host
memory once. The adapter validates the fixture ABI, import contract, property
schema, handler routes, and available primitive dependencies. Prefetch registers
the definition without creating any component instances. Activation installs the
child's dispatch function and makes the parent's conditional outlet mountable.
The outlet builds a normal `ComponentInstance` through
`DefinitionToInstanceTraverser`, inside the existing scene and property graph.
Pax performs expression evaluation, bindings, slot projection, layout, and
rendering. The child's WASM owns its Rust instance state and event logic.

The generated cartridge registry is private. For this experiment, the adapter
collects public primitive descriptors from the host's existing instance tree.
Consequently, a primitive must already appear in the base template to be
available to a package. Production support needs an explicit generated registry
and dependency/capability contract; this discovery mechanism is fixture-only.
No production runtime, compiler, or chassis code is changed.

The parent uses `count=bind:count` to share a writable property; `step` and `warm`
are ordinary input bindings. The adapter creates local properties from the loaded
schema and exposes scalar handles to the child. Handles and module instance
tokens are revoked on unmount. Remounting creates fresh child state and retains
parent state. The module and its definition remain loaded for the page lifetime.

Select **Direct WASM imports** or **JS property bridge** before loading. Both
execute synchronously. The direct path calls through the host's function table;
the child imports raw host WASM functions for property lookup, reads, and writes.
The comparison path wraps those imports in JavaScript. JavaScript performs
loading and registration, including the one-time descriptor JSON decode. No
per-event path uses workers, `postMessage`, serialization, or promises.
Rust allocations, pointers, `Rc`, `Any`, and `Property` objects never cross the
separate linear memories.

The host uses `wasm_bindgen::function_table()` to retain/expose its function table
and `--growable-table` for late registration. Merely asking the Rust linker to
export its table is insufficient after wasm-bindgen processing.

## Build and run

Requirements: the repository's Rust/WASM toolchain, `wasm-pack`, Python 3, and a
web interface built from the same repository revision. For example, build
`examples/increment` with `pax-cli build --path examples/increment --target web
--release`, then use its `.pax/build/release/web` directory as `--interface`.
The fixture bypasses CLI packaging to keep experimental linker flags and loader
code local to this directory.

From the repository root:

```sh
export CARGO_TARGET_DIR=/tmp/pax-module-probe-target
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0
python3 tests/src/deferred-cartridge-probe/build.py \
  --output /tmp/pax-module-probe/split \
  --interface examples/increment/.pax/build/release/web
python3 tests/src/deferred-cartridge-probe/build.py \
  --mode monolithic --output /tmp/pax-module-probe/monolithic \
  --interface examples/increment/.pax/build/release/web
python3 -m http.server 18751 --bind 127.0.0.1 --directory /tmp/pax-module-probe
```

Open `http://127.0.0.1:18751/split/`. The parent works before the child is
requested. Load the child, change its increment through the parent, reset the
parent value, change the child's color, and unmount/remount it. Projected text
remains owned by the parent; the `Frame` and native buttons use the same scene.

To rebuild only a component package, use `--child-only` with an existing output
directory, optionally supplying `--template` and `--descriptor`. This leaves
the base WASM and generated cartridge glue unchanged. The fixture web files and
interface are refreshed. Changing the public import contract requires a
compatible host; arbitrary ABI compatibility is not implied.

## Checks

Provide Playwright through the local Node installation or `NODE_PATH`:

```sh
source ~/.zshrc
nvm use
node tests/src/deferred-cartridge-probe/check.cjs \
  http://127.0.0.1:18751 /tmp/pax-module-probe/check.json
python3 tests/src/deferred-cartridge-probe/package-check.py \
  --output /tmp/pax-module-probe \
  --interface examples/increment/.pax/build/release/web
node tests/src/deferred-cartridge-probe/package-check.cjs \
  http://127.0.0.1:18751 /tmp/pax-module-probe/package-check.json
```

`check.cjs` covers no initial child request, prefetch without mounting, parent and
child updates, slots, unmount/remount, stale tokens, duplicate load/activation,
ABI/capability rejection, missing/corrupt downloads, pre-aborted requests, and
retry. `package-check.py` rebuilds a child with a changed template, an additional
local property, and a renamed handler. It asserts byte-identical base WASM and
absence of child-specific strings from that base. `package-check.cjs` exercises
the alternate component, stale property handles, rejection of invalid import
contracts/handler routes/truncated definitions, retry, and absence of separate
metadata requests. Screenshots record the shared scene; these are not pixel-level
clipping tests. `CHROME` overrides the default macOS Chrome executable.

For debug validation, build with `--debug` into a separate directory. Set
`PROBE_MODES=wasm,js` when no monolithic build is beside it. Debug uses baked
definitions with designtime disabled; this does not test template hot reload.

Optional `bench.cjs` rotates all three modes through twelve trials in one page,
using separate same-origin frames. Arithmetic loops compare a non-inlined local
function with opcode dispatch and a function-table crossing. Event loops include
two property reads and two writes (parent count and local click count). These
measure synchronous work, not event-to-paint latency or full-frame performance.
Run without concurrent builds. Results from the earlier logic-only probe are
not directly comparable because the property path and event work changed.

Both controls retain package decoding and probe instrumentation. Their size
difference isolates deferring this package; it does not measure infrastructure
cost against a normal generated cartridge. Compare that base cost separately.
This tiny component is not representative of a demo's savings.

## Experimental limits

This package contains one custom component using existing standard primitives.
The explicit schema supports only `u32` and `bool` properties, at most 32 fields,
and eight handler descriptors. Rust state supports eight live instances. Event
payloads and `NodeContext` services are not marshalled; handlers receive their
module token. Arbitrary Rust structs, nested custom components, asset bundles,
self-property settings/timelines, and automatic partitioning/code generation are
not implemented. The host adapter is not a general replacement for generated
Rust component descriptors. Metadata is authored explicitly for this fixture.

The scalar contract assumes trusted modules built together, one thread, and no
reentrant child dispatch. The bounded generation token is not a production
allocator. Host handle IDs are not reused; released entries retain empty slots.
The existing runtime descriptor API requires static references, so a single
successfully staged package retains a bounded descriptor allocation for the page
lifetime. Registration rejects further packages. There is no executable unload.
The loader validates a fixture ABI and capability mask, not content hashes or
arbitrary WASM function signatures; signature mismatches trap on invocation.

Cancellation belongs to the first prefetch request. Shared-consumer cancellation,
mid-compilation cancellation tests, simultaneous consumers, whole-application
teardown, device-loss recovery, runtime feature modules, native targets, browser
qualification, and production resumable mounting remain outside this fixture.
