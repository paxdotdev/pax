# PAX-968 Runtime Resilience During Web Reloads

Status: implemented
Last revised: 2026-07-13

<!-- summary: Generation-safe web hot reload, atomic artifact publication, and isolated image decoding failures. -->
<!-- tags: designtime, hot-reload, runtime, wasm, websocket, web -->

## Summary

Keep the last working web cartridge running when the design server disconnects,
restarts, or publishes an incomplete replacement. A cartridge may consume
template and manifest updates only after the JavaScript host has activated the
matching build generation.

The implementation adds four defenses:

1. A build-id handshake freezes manifest/template mutation between a
   `ReloadAppRequest` and activation of that cartridge.
2. A cartridge ABI identity rejects manifests that cannot be executed by the
   current static descriptor registry.
3. Web reload artifacts are published by an atomic directory rename only after
   JavaScript glue, Wasm, and the wasm-bindgen snippets tree are present.
4. Image fetch/decode failures are retried, reported, and kept outside the Wasm
   interrupt boundary; work belonging to a disposed cartridge is abandoned.

These changes are designtime/web-host behavior. They do not change the release
cartridge representation or binary baking format.

## Motivating Failure

The `path-drawing` session reported this sequence after the design server went
away during development:

- the privileged-agent websocket closed;
- the host repeatedly failed to load a staged `pax-cartridge.js`;
- an image promise rejected with `InvalidStateError` during decode;
- the runtime tried to instantiate `FontComparisonRow` from a manifest that no
  longer contained a matching component generation;
- Wasm panicked, after which wasm-bindgen reported repeated recursive mutable
  borrows and the tab stopped making progress.

The websocket close was not itself fatal. Existing reconnect behavior allowed
the old chassis to keep ticking. The fatal transition was a cross-generation
manifest update: the design server installed its newly compiled manifest in
shared server state, and the still-running old cartridge could receive that
manifest before JavaScript drained the reload request and mounted the new
cartridge. The manifest was dynamic, but the component descriptor registry was
compiled statically into Wasm. Mixing them made a normal lookup panic.

The later wasm-bindgen aliasing errors were fallout from that first panic, not a
separate websocket defect.

## Runtime Invariants

The hardened flow preserves these invariants:

- A live cartridge may apply template-only changes from its own component ABI.
- A `ReloadAppRequest` for build `B` puts that websocket client into
  `AwaitingCartridge(B)` before any later manifest/template message is applied.
- While awaiting `B`, manifest responses are discarded and template updates
  are ignored. The server remains the source of truth; a fresh manifest is
  requested after activation instead of replaying potentially stale deltas.
- Only an acknowledgement from build `B` releases the gate. An acknowledgement
  for an older or unrelated build is ignored.
- A newer reload request supersedes the build currently being awaited.
- A websocket disconnect clears the process-local wait state. On reconnect, a
  surviving server replays its latest reload envelope before answering the
  manifest request. A restarted server has no envelope, so its manifest is
  admitted only through the ABI compatibility guard.
- Each connection remains quarantined after reconnect until a complete,
  ABI-compatible manifest is accepted. A rejected manifest cannot be followed
  by an otherwise-valid template delta that mutates the old cartridge's tree.
- Rejected or malformed server data leaves the current manifest, tree, and
  render session untouched.

Reconnect attempts use capped exponential backoff. A stopped server therefore
does not flood the browser console or spin a new websocket twice per second,
while the first recovery attempt still happens after 500 milliseconds.

## Activation Handshake

For a successful Rust rebuild, the sequence is:

1. The compiler finishes a web cartridge and atomically publishes build `B`.
2. The design server records and sends `ReloadAppRequest(B)`.
3. The old chassis enters `AwaitingCartridge(B)` and forwards the request to the
   JavaScript host without applying newer manifest/template messages.
4. JavaScript imports and initializes build `B`. If loading fails, the old
   chassis remains mounted and the host retries with capped backoff and a
   cache-busted artifact URL. Replayed requests matching either the pending or
   in-flight build are ignored.
5. The new cartridge opens its websocket. The design server replays
   `ReloadAppRequest(B)` to that client.
6. JavaScript sees that the current artifact URL already names build `B` and
   acknowledges `B` through the new chassis.
7. The new chassis requests a fresh manifest, validates it, and resumes normal
   template hot reload.

Comparing normalized artifact URLs makes the replay idempotent even when one
side uses a relative URL and the other resolves it against the document base.

## ABI Compatibility Guard

The embedded manifest establishes the executing cartridge's ABI identity. A
server manifest must match:

- the set of component type IDs;
- primitive and struct-only component classification;
- primitive instance import paths; and
- the canonicalized type table, including component property definitions.

Templates, settings, timelines, source module paths, asset directories, and
other editable/source metadata are deliberately excluded. They may change
without changing the compiled descriptor registry.

Before any server manifest is installed, every non-control-flow template node
must also reference a component present in that manifest, and the main
component must exist. The same reference check is applied transactionally to a
single `ReplaceTemplateRequest`. This converts the former runtime panic into a
warning while retaining the last known good tree.

## Atomic Web Artifact Publication

Each reload build is first copied into a hidden sibling directory. Publication
requires nonempty copies of:

- `pax-cartridge.js`
- `pax-cartridge_bg.wasm`
- the wasm-bindgen `snippets` dependency tree imported by the JavaScript glue

Optional declaration and package metadata are copied when available. Only
after the required set succeeds is the hidden directory renamed to the public
`__reloads__/<build-id>` path. A failed copy removes the hidden directory and
never exposes the final URL, so a reload request cannot race a partially copied
JS/Wasm pair.

The JavaScript loader also treats a non-successful Wasm HTTP response as a load
failure rather than passing an error page to WebAssembly initialization.

## Image Failure Boundary

Image loading now retries the entire fetch, blob decode, bitmap creation, and
canvas-readback operation. This matters because a successful HTTP response can
still contain incomplete or undecodable bytes while a dev server is restarting.

After the bounded retry window, the host logs one contextual warning instead of
leaving an unhandled promise rejection. Before calling `image_loaded` or
delivering decoded bytes, the native-element pool verifies that it still owns
the same chassis. An image decode that finishes after a cartridge swap therefore
cannot call into freed Wasm.

## Scope Boundary

PAX-968 keeps the currently mounted session alive across websocket loss,
incompatible server state, missing reload artifacts, and image failures. It
also prevents those failures from poisoning the Wasm borrow boundary.

PAX-889 remains the owner of the broader transactional mount work: proving that
a newly instantiated cartridge has attached and produced a healthy first frame
before disposing the last known good chassis, pruning long-lived reload
artifacts, and adding full browser smoke coverage. This change preserves the
old chassis through import/fetch/initialization failure, but it does not claim a
general rollback after attachment has begun.

## Validation

Regression coverage includes:

- incompatible component sets and changed type tables do not replace the live
  cartridge manifest;
- internally inconsistent manifests are rejected;
- template updates cannot introduce a missing component reference;
- reload requests freeze following manifest updates until the matching build
  is acknowledged;
- unrelated acknowledgements do not release the gate;
- disconnects discard orphaned generation wait state, while incompatible
  reconnect manifests keep template deltas quarantined;
- compatible full manifests release ordinary template hot reload;
- web reload directories publish only when the full required artifact set
  exists; and
- a missing Wasm file or snippet tree leaves no public or pending reload
  directory.

A browser failure drill stopped the design server under the running
`path-drawing` example. The mounted scene and stroke animation continued, no
missing-component, Wasm trap, recursive-borrow, or image-decode error appeared,
and reconnect attempts reached the five-second delay cap. A fresh web build
also verified that the emitted cartridge, Wasm binary, interface bundle, and
wasm-bindgen snippet import compile together.
