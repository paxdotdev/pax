# PAX-968 Runtime Revision Coordination

Status: implemented and validated on web, macOS, and iOS Simulator
Last revised: 2026-07-17

<!-- summary: Debug-only revision coordination for Pax templates, compiled logic artifacts, and resilient design-server reconnects. -->
<!-- tags: designtime, hot-reload, runtime, websocket, web, macos, ios -->

## Summary

Keep the last working cartridge running when the design server disconnects,
restarts, or publishes an incomplete replacement. The core rule is that a
runtime may only consume program metadata belonging to the logic revision it is
executing.

PAX-968 now enforces that rule in one shared revision protocol rather than in
web-specific socket code. A program revision is identified by:

- `logic_revision_id`, which names compiled or interpreted application logic;
- `template_version`, a monotonic sequence of Pax-only updates within that
  logic revision.

The design server owns a `DebugRevisionCoordinator`. Each cartridge owns a
small `RevisionGate`. Websocket and filesystem code only transport generic
prepare, activate, manifest-snapshot, and template-update messages. Web and
macOS hosts remain responsible for loading their platform artifacts.

These structures are debug-only orchestration. They are not serialized into a
`PaxManifest`, `ProgramIR`, binary cartridge, generated descriptor table, or
other release-cartridge representation.

## Reload Policy

Debug sessions expose the shared coordinator through two independently gated
semantic lanes: `pax` and `logic`. `HotReloadMode` combines them as `all`,
`pax`, `logic`, or `off`. The logic name deliberately describes application
behavior rather than Rust or dynamic linking: compiled Rust artifacts and
future interpreted modules can use different chassis adapters behind the same
policy boundary.

The effective mode is selected by `pax-cli run --hot-reload`, then
`PAX_HOT_RELOAD`, then `[package.metadata.pax.dev].hot_reload`, then the `all`
debug default. A disabled lane continues accepting source writes but cannot
mutate the mounted revision or schedule its build path. The server emits one
restart-required notice per suppressed lane. `off` retains the designtime
server for inspection; stopping designtime entirely remains a separate
concern.

Web and macOS implement both lanes. iOS and iPadOS implement the Pax lane only,
so an explicit `logic`-only request is rejected and `all` reports that logic
edits require rebuilding. Release builds force `off` independently of all
debug configuration.

## Motivating Failure

The `path-drawing` session reported this sequence after the design server went
away during development:

- the privileged-agent websocket closed;
- the host repeatedly failed to load a staged `pax-cartridge.js`;
- an image promise rejected with `InvalidStateError` during decode;
- the runtime tried to instantiate `FontComparisonRow` from a manifest that no
  longer matched its compiled component descriptors;
- Wasm panicked, after which wasm-bindgen reported repeated recursive mutable
  borrows and the tab stopped making progress.

The websocket close was not itself fatal. The fatal transition was a
cross-generation manifest update: the design server replaced its globally
visible manifest as soon as compilation finished, before the host had loaded
and activated the corresponding application logic. The manifest was dynamic,
but component, property, and handler descriptors were compiled statically.

The later wasm-bindgen aliasing errors were fallout from that first panic, not a
separate websocket defect.

## Shared State Machine

The coordinator has one active revision and at most one staged candidate. Logic
activation is a two-phase transaction:

1. The active revision continues serving full snapshots and Pax-only updates.
2. A logic build stages a candidate manifest and emits `PrepareAppRevision`.
   Staging never replaces the active manifest.
3. The host validates and loads the artifact described by the generic prepare
   envelope, but keeps the old runtime mounted.
4. The candidate sends `RequestAppRevisionActivation`. The server replays the
   normalized Pax journal, validates the resulting descriptor capability, and
   reserves that exact candidate without changing the active revision.
5. The server returns `AppRevisionActivationPrepared` with the exact manifest
   the candidate would run. The candidate installs it transactionally, then
   sends `ActivateAppRevision` as the final commit request.
6. Final commit performs no parsing or other fallible source replay. Before a
   new revision becomes active, the server atomically publishes a durable debug
   restart record containing the exact revision stamp, manifest, artifact
   envelope, and survivor-adoption authority. It then promotes only the
   matching reservation and returns the committed full snapshot. A failed
   restart-record write leaves the previous revision active; stale IDs are
   rejected.
7. Only after the candidate observes that committed snapshot does the host swap
   runtimes and dispose the previous cartridge.

A host may cancel during prepare and keep the old runtime. Once final commit is
in flight it cannot safely infer failure from a timeout: the request and its
matching acknowledgement are retried across reconnects until server authority
is known. If a Pax mutation lands after the candidate was reserved, the server
commits the frozen validated candidate and immediately queues a follow-up logic
build; it never introduces a fallible operation into final commit.

The prepare envelope separates execution policy from transport:

- `execution_mode` is currently `compiled-artifact`; `interpreted-module` is
  reserved so a future JavaScript or other interpreted logic host can use the
  same coordinator without pretending to be a dylib;
- `artifact.kind` and `artifact.location` are adapter data, currently
  `web-cartridge` or `macos-dylib`.

This is intentionally only the seam for future interpreters. Module lifetime,
state migration, evaluator APIs, and language-specific caching are not designed
by PAX-968.

## Pax-Only Updates

A `.pax` edit is not a logic-artifact activation:

1. The server parses the source against the active manifest.
2. It compares the runtime descriptor capability before and after the edit.
3. A compatible edit commits to the active manifest, increments only
   `template_version`, and streams an `UpdateTemplateRequest`.
4. Source edits and dynamic authoring mutations are recorded in one normalized
   replay journal. Every logic build records the journal generation immediately
   before compilation; preflight applies only mutations newer than that
   baseline. The journal groups every component backed by the same source file,
   while whole-manifest authoring serialization is restricted to dynamic fields.
   This closes the race where template state changes after a Rust build took its
   source snapshot without replaying lifetime-old entries that a later build
   intentionally deleted or renamed, and without allowing replay to overwrite
   candidate logic or compiler metadata.
5. An edit that introduces a component or generated handler capability the
   current cartridge does not have is escalated to a logic build instead of
   being mislabeled as template-only.

The runtime accepts exactly the next template version for its active logic
revision. A gap requests one full snapshot; duplicates, stale versions, and
updates for another logic revision are ignored.

All server-side source mutations and active-client promotion pass through the
same source transaction barrier. Each source path also carries a mutation
generation, and commits compare both that generation and the full revision
stamp observed before parsing or serialization. A delayed watcher parse can
therefore neither overwrite a newer authoring edit to the same file nor erase a
concurrent edit to another file. Source changes still commit while no socket is
connected; the active socket is only a delivery route.

Watcher admission is path- and content-aware. Unsupported files and transient
editor backups are rejected before any read. Server-authored writes register
their exact canonical path and resulting contents, so their filesystem echo is
suppressed without creating a global interval in which unrelated edits can be
lost. Repeated notifications with unchanged contents are coalesced. The logic
worker separately marks when compilation has actually started: changes during
the initial debounce are captured by that build, while a changed source
observed after compilation begins queues one follow-up build.

## Runtime Capability Guard

The embedded manifest establishes the executable cartridge's descriptor
capability. Before installing a server snapshot or template update, the runtime
checks the parts generated statically into the cartridge:

- component type IDs and primitive/struct-only classification;
- primitive instance import paths;
- the canonical type table and property definitions;
- generated event-handler names and event argument types, following the
  runtime's first-match lookup rule. Reordering distinct names and repeating an
  identical signature are harmless; a later duplicate with a conflicting
  signature is incompatible and must trigger a logic rebuild.

Templates, settings that do not alter generated handlers, timelines, source
module paths, asset directories, and other editable metadata remain dynamic.
Every non-control-flow template node must still reference a component present
in the manifest, and the main component must exist.

The guard is transactional: rejected data leaves the current manifest, tree,
and render session untouched.

## Disconnect and Restart Semantics

Socket connectivity is deliberately absent from the revision state machine.
Disconnecting invalidates trust in the last server snapshot, but it does not
invalidate or stop the mounted cartridge. The transport reconnects with capped
exponential backoff while the engine continues ticking and rendering.

On reconnect, the runtime sends its full active revision stamp and requests a
full snapshot. A queued activation preflight or final commit is replayed before
an ordinary snapshot request, so a standby candidate cannot accidentally be
adopted as the active client. The server atomically restores its debug restart
record before accepting clients. That record preserves the exact committed
revision stamp, active manifest, retained artifact envelope, and the authority
needed to reconcile a surviving executable. This is not runtime-side
re-labeling of old code as a newly built revision: the executable remains the
authority for `logic_revision_id`, and the returned snapshot must still pass
the embedded cartridge capability guard before the runtime installs it.

After restoring the record, the server rereads every active `.pax` source from
disk through the normal source transaction. This recovers edits that landed
after the last snapshot or while the server was down. Web and macOS also queue
one recovery logic build after restoration, which captures Rust/source edits
that cannot be reconstructed from the Pax journal and any edit that arrived
after a candidate's activation preflight. iOS has no dynamic native-logic lane:
it reconciles compatible Pax-only edits, while changed application logic still
requires rebuilding and relaunching the cartridge.

An unactivated websocket `PrepareAppRevision` is process-local server state and
is discarded at disconnect. A surviving or restarted server must replay its
current candidate before sending the snapshot.

## Chassis Adapters

### Web

Web logic builds publish JavaScript, Wasm, and the wasm-bindgen snippets tree by
atomic directory rename. The server stages the resulting `web-cartridge`
candidate. JavaScript imports it with cache-busted, capped retries and ignores
superseded in-flight builds. A fresh candidate socket remains on standby while
its revision gate runs preflight and final commit; it cannot evict the active
socket merely by connecting. The current chassis keeps rendering until the
candidate reports `Committed`, at which point the host attaches the candidate
and disposes the old chassis.

The browser transport is not considered alive until its websocket reports
`Opened`. Calls made while the browser socket is still `CONNECTING` remain in
the revision gate for replay instead of reaching `WebSocket.send` and throwing
an `InvalidStateError`.

Once activated, the artifact envelope is retained so a fresh page that starts
from the root cartridge can advance to the current debug revision.

### macOS

The design server sends the same prepare envelope through the existing dev
session directory with a `macos-dylib` artifact. The DEBUG host:

1. opens and resolves the candidate dylib without changing the current API;
2. initializes and primes the candidate engine with its revision ID;
3. asks the candidate's shared revision gate to run preflight while polling only
   designtime transport state, not application logic or rendering;
4. sends final commit after the prepared manifest is admitted;
5. preserves the old engine and API until the candidate observes `Committed`;
6. swaps APIs, disposes the old engine with its matching deallocator, and
   reports the revision-tagged result to the coordinator.

The compiler does not publish the candidate manifest as active before that
activation. Failure closes the candidate and resumes the previous engine. The
server waits for both the host result and activation acknowledgement before
allowing another native candidate to supersede it. The restart record is
published before a newly activated native revision is committed. If that
durable publication fails, the host receives a rejection and keeps the old
engine mounted. A duplicate final request for an already active revision
remains idempotent and is acknowledged even if an opportunistic republish
fails; a disk problem cannot invalidate the revision both sides have already
committed.

The CLI launches the debug app through LaunchServices with explicit
`open --env` arguments for the dev-session paths and design-server address. Setting
those values only on the `open` helper process is insufficient because
LaunchServices does not otherwise forward arbitrary environment variables into
the app bundle.

### iOS and iPadOS

The Apple mobile cartridge uses the shared revision gate, manifest capability
guard, disconnect handling, and reconnect backoff. For designtime simulator and
device runs, the CLI starts a template server and injects
`PAX_DESIGN_SERVER_ADDR` into the launched application. Simulator launches use
loopback; physical-device launches advertise a LAN-reachable host, with
`PAX_DESIGN_SERVER_ADVERTISE_HOST` available for VPN and multi-interface
machines. The designtime staging plist receives the local-network usage
description before signing.

Mobile deliberately omits the native-session argument used by macOS, so a Rust
edit does not advertise or attempt a dylib replacement. Pax-only edits and
hangup recovery work through the shared websocket lane; application-logic
changes still require rebuilding and relaunching the app. With no injected
address, the chassis uses an explicit offline transport instead of silently
probing `localhost:8080` forever.

## Release Boundary

Production builds do not participate in this protocol:

- the compiler checks the exact normal-edge Cargo graph for the requested
  target and feature set, and rejects a direct or transitive designtime
  dependency while ignoring dev-only, build/proc-macro, inactive optional, and
  off-target edges;
- generated application code also has a compile-time backstop that rejects
  designtime expansion when the compiler declares `PAX_BUILD_DESIGNTIME=0`;
- macOS dylib prepare/swap code and symbol resolution are guarded by `DEBUG`;
- revision priming and activation C symbols are absent without the designtime
  feature;
- release web cartridges never receive prepare messages;
- no revision or artifact envelope is baked into the descriptor table or
  release binary representation.

The durable restart record also lives only under the debug `.pax/dev` working
state. It is neither an input to cartridge generation nor a fallback source of
runtime metadata in release mode.

Release cartridges therefore support neither dynamic linking nor `.pax` live
reload. The active/candidate model exists only around debug sessions.

## Independent Failure Boundaries

Two adjacent defenses from the original failure remain intentionally separate
from revision coordination:

- Web reload artifacts are exposed only after JavaScript, Wasm, and snippets
  are all present. A missing required file removes the pending directory and
  never exposes the final URL.
- Image loading retries the entire fetch, blob decode, bitmap creation, and
  canvas-readback operation. Promise failures are handled in JavaScript, and
  ownership is rechecked after awaits so decoded pixels cannot be delivered to
  a disposed cartridge.

## Validation Expectations

Automated coverage exercises revision staging and two-phase matching activation,
template-version gaps, active template updates while a candidate is prepared,
strict native priming, disconnect/reconnect replay, restarted-server adoption,
incompatible ABI rejection (including conflicting duplicate handler
signatures), watcher processing without an active socket, complete web artifact
publication, native request/response serialization, LaunchServices environment
forwarding, and mobile template-server provisioning.

Manual acceptance uses the animated `path-drawing` example on web, macOS, and
iOS Simulator. For each chassis, the application must render and animate before
the server is killed, remain alive and animate after the hard hangup, reconnect
to a compatible server without relaunching, and avoid missing-component panics,
Wasm traps, recursive-borrow errors, and unhandled image failures. Each run also
distinguishes a `.pax` edit from a logic edit: web and macOS activate a staged
compiled artifact only for the latter, while iOS reports that native logic
requires a rebuild and keeps its baked cartridge. Incompatible snapshots are
covered by the transactional capability-guard tests so they cannot replace the
last working manifest.

## Validation Results

The final `path-drawing` fault drill passed on all three supported chassis:

- Web: a Pax-only blue-to-red edit changed the running template without a new
  compiled artifact; a Rust edit activated a staged web cartridge. After the
  design-server process was hard-killed, the red cartridge kept animating for
  more than seven seconds. Restoring both sources while the server was down and
  restarting it on the same port reconciled the template back to blue and
  produced one recovery cartridge without reloading the page. Disconnect
  retries were bounded, and the earlier `CONNECTING`-state send error did not
  recur.
- macOS: the Pax-only edit changed the running app without publishing a dylib;
  the Rust edit activated a staged dylib. The app remained red and animated
  through a hard server kill, then reconnected on the same port and dev-session
  directory, returned to blue, and activated exactly one recovery dylib without
  relaunching. A separate failed-build drill saved invalid Rust, confirmed the
  mounted app and dev screenshot channel stayed responsive, then corrected the
  source and observed one watcher admission and one newly staged dylib.
- iOS Simulator: the Pax-only edit reloaded through the template lane, while a
  Rust edit produced no native artifact or logic-revision change. The app
  remained alive and animated after a hard kill, then reconciled the restored
  blue template after the same server endpoint returned, without relaunching.
  An initially misleading blue capture was a later fixture exposed by scroll
  position; the edited first waveform was separately confirmed red.

The same failed-build drill passed on web: invalid Rust left the active
cartridge and dev screenshot channel responsive, while the corrected save
produced one watcher admission and one replacement web artifact.

The final automated pass completed 50 `pax-designtime` tests, 131
`pax-compiler` tests, the release-feature-boundary fixture, compiler all-target
and designtime wasm32 checks, docs validation, macOS Swift parsing, Rust format
checking, and whitespace checking without failures.

PAX-889 remains the owner of broader transactional mounting: requiring a newly
committed cartridge to attach and produce a healthy first frame before the old
host is disposed, state migration between executable logic revisions, and
long-lived reload-artifact pruning.
