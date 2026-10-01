# PAX-1013 — Application lifecycle and async integration

Status: **Implemented; macOS debug/release UI and sandboxed TCP checks passed; mobile lifecycle qualification pending.** Authoring date: 2026-09-30.

This proposal starts with application setup hooks and ends with supported
Tokio integration on Pax's native targets, with a matching browser async
workflow. It also specifies a new `examples/src/async-workbench` fixture.
The user approved implementation after selecting the shared/local property split.

Implementation checkpoint (2026-10-01): shared/local storage and owner graph
projections, application hooks and services, guarded scopes, bounded UI delivery,
optional Tokio/browser adapters, asynchronous managed shutdown, host wake routes,
and the Async Workbench scenarios are implemented. Debug regression tests,
browser interaction checks and macOS native UI scenarios pass. The macOS host
also recovers from injected display-clock startup failures. Typed Apple Info.plist
and entitlement overlays now support arbitrary keys, native source files,
iPadOS inheritance and managed-key diagnostics. The fixture explicitly opts into
macOS network client/server entitlements; its signed release TCP exchange passes.
Release-baked browser/native cartridge checks, iOS simulator debug/release builds,
documentation and cost measurements are recorded in the fixture README; this is not yet a blanket claim
that every target acceptance gate below has passed.

Selected architectural direction: `Property<T>` is the thread-safe default for
transferable application state; `LocalProperty<T>` owns thread-affine values and
computations. Both feed one UI-owned reactive graph. `ExpandedNode`,
`NodeContext`, and the live runtime remain local. Application setup hooks and
task lifetimes build on this boundary; an explicit public `async_scope` is not
required merely to publish a property value. Section 5 defines the contract and
the remaining prototype gates.

There are no external users to migrate, and breaking API changes are acceptable.
Do not add compatibility shims or migration work for these changes.

The existing [investigation and executable probes](../../../../tests/src/tokio-support-probe/README.md)
are the evidence baseline. Five headless checks and the property-thread
reproduction passed in debug and release on Apple Silicon macOS. Those results
are not an end-to-end application proof.

## 1. Outcome and support contract

A builder should be able to initialize services before the first component
mount, start asynchronous work from ordinary synchronous handlers, update
reactive state when results arrive, and clean up without owning the process's
`main` function. Choosing Tokio must not make Tokio a dependency of every Pax
application.

“Full Tokio support” in this ticket means:

- An application can own or borrow a native Tokio runtime, configure its
  workers/drivers, and use its supported timer, I/O, channel and task APIs.
- Workers can publish shared application properties directly. Reactive
  evaluation and UI effects run on the UI thread without blocking it on async
  work or checking channels from application `@tick` handlers. Lossless streams
  and callbacks into local state additionally use bounded delivery primitives.
- Cancellation, unmount, replacement requests, shutdown and logic reload have
  explicit behavior, including late completions and non-abortable work.
- A multithread runtime is the standard adapter. A continuously driven
  current-thread runtime on a dedicated service thread is also proven. A
  `LocalSet` may own non-Send service state on that thread and publish shared
  properties; it cannot access UI-owned `LocalProperty` values or nodes.
- The same Pax result-delivery contract works with browser futures. This does
  not mean desktop Tokio networking, filesystem, process or timer drivers work
  in browser Wasm. Native I/O capabilities remain subject to platform limits.
- Debug and release-baked programs implement the same ownership contract.
  Existing synchronous apps are updated for intentional property API changes;
  template reactivity and supported reload modes continue to work.

A Tokio `LocalSet` running UI-affine futures on the Apple main thread is a
separate executor/run-loop integration and is not part of this support claim.
No `block_on` or periodic executor polling will be inserted into Pax handlers.
An optional user-owned Rust `main`/embedding runner remains a later consumer of
the lifecycle contract, not a prerequisite or deliverable of this ticket.

## 2. Three lifetimes

| Lifetime | Owner and boundaries | Examples |
| --- | --- | --- |
| Host | Browser page/mount host or Apple native shell | Logging facilities, window/view integration, frame scheduler |
| Application instance | One active cartridge instance and its services | Tokio runtime, data clients, application work scope |
| Mounted node | One node attachment generation inside that application | A pending search, screen subscription, progress stream |

An application instance has a unique ID and revision identity. Component
remounts do not repeat application setup. A logic replacement creates a new
application instance; services are not silently transferred across dylib/Wasm
or Rust type boundaries. Multiple mounted applications have separate services
and scopes even when they share one process.

## 3. Application setup hooks

### Proposed authoring surface

All new API names and examples in this document are proposed. The initial
implementation should preserve these semantics even if review adjusts names.
Use an explicit Rust attribute on the root rather than discovering specially
named methods or requiring a trait implementation on every existing app:

```rust,ignore
#[pax]
#[main]
#[application(WorkbenchApplication)]
#[file("lib.pax")]
pub struct Workbench {
    pub clicks: Property<usize>,
}

pub struct WorkbenchApplication;

impl Application for WorkbenchApplication {
    fn configure(app: &mut AppBuilder) -> AppResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?;
            app.provide_managed(pax_tokio::TokioService::owned(runtime)?)?;
        }
        app.provide(WorkbenchConfig::default())?;
        Ok(())
    }

    fn started(app: &AppContext) {
        // Optional: start work through an application-bound executor helper.
    }

    fn stopping(app: &AppContext, reason: StopReason) {
        // Optional, synchronous notification. No waiting for workers here.
    }
}
```

`configure` is fallible; `started` and `stopping` default to no-ops. Hooks are
ordinary synchronous Rust and run on the application's UI thread. State lives
in registered services, so hooks do not need a second mutable root component.
Omitting `#[application(...)]` selects an empty application configuration.
A dependency's annotated main component must never configure another app when
used as an ordinary component.

### Startup order

1. Establish the host's minimal error-reporting/panic boundary and application
   identity, but do not yet construct root state or install a default logger.
2. Call `configure` once. Construct services and configure optional logging.
   This hook has target/launch information, not a `NodeContext` or mounted UI.
3. Install Pax's default logging only if the application has not supplied its
   own compatible setup. Host/process-global facilities must be idempotent
   across multiple app instances and reload; logger replacement is not promised.
4. Freeze the service registry, construct the engine and root state, and mount
   with the registry already available to `on_mount` handlers.
5. Commit activation, call `started` once, then release any scoped work staged
   during mounting and permit its UI deliveries. Present the first frame.

`started` is not a guarantee that all asynchronous layout, fonts, images or
child measurements are complete. A slow operation starts through a service and
shows loading state; `configure` must not perform blocking network/database
startup. Async preparation before the first UI is not a new implicit startup
executor in this proposal.

If configuration fails, report a structured startup error through the host
(web startup rejection/diagnostic surface; native error result handled by the
shell). Do not unwind through C ABI, mount a partial root, or run `started`.
Previously registered resources still receive cleanup. Errors/panics during
later hooks enter the host's fatal-error boundary with best-effort teardown;
recovery from an aborting panic or arbitrary external side effects is not
promised.

### Reload preparation

Current logic reload can construct a candidate before activation. Therefore
configuration and mounting must support a **Preparing** application state:
registration is allowed, but scoped task execution and UI deliveries remain
staged until activation. `started` is only called for the activated candidate.
Discarding a candidate cancels staged work and shuts down its services.

`configure` should prepare dormant services; externally observable work belongs
in `started` or scoped tasks. Pax cannot roll back arbitrary side effects in
user constructors. Exclusive resources need an application-specific handoff or
must wait until activation; this proposal does not promise transactional
replacement of arbitrary databases, ports or native SDK state.

## 4. Services and ownership

Add an application registry alongside the existing lexical local stores:

| Proposed API | Contract |
| --- | --- |
| `AppBuilder::provide<T: 'static>(T)` | Own a plain service for the application lifetime |
| `AppBuilder::provide_managed<T: ManagedService>(T)` | Also participate in staged shutdown |
| `AppContext::service<T>() -> Result<Rc<T>, ServiceError>` | Get a UI-thread service handle |
| `NodeContext::application() -> AppContext` | Reach the owning application |

Duplicate concrete types fail configuration instead of silently replacing a
service. Use purpose-specific newtypes for several instances of one underlying
type. The registry is immutable after configuration; services use their own
well-defined interior mutability. Services are not Pax properties, expressions,
serializable manifest values, or automatically persisted application state.

Keep `provide_store`/`with_store` for component-owned subtree state; application
services have a separate, application-wide lifetime.
Application services do not shadow or fall back through those stores. Reuse
internal type-erasure machinery where useful, but keep these different lifetime
and lookup contracts explicit.

`AppContext`, service `Rc` handles, `NodeContext`, local scope controllers and
`LocalProperty` handles remain UI-thread-bound. Shared `Property` handles and
the explicitly transferable producers/tokens described below can cross threads.
A service intentionally exposes cloneable `Send` clients/handles for workers
when appropriate. Do not capture a service `Rc` or `NodeContext` inside
`tokio::spawn`; shared property state does not make a whole component Send.

Cloned `AppContext` values reference the owner weakly and cannot keep an app
mounted or accepting work. An explicitly retained service `Rc` can extend its
allocation lifetime beyond registry release; it does not extend active managed
resources past shutdown. Managed services report Closed once shutdown begins.
Keep lifecycle tokens separate from service allocations to avoid ownership
cycles through callbacks, contexts and the service registry.

Plain services have synchronous UI-thread destruction. Resources whose
shutdown can wait must use managed registration. `ManagedService` begins
shutdown synchronously and returns an executor-neutral `ShutdownTicket`:
completion/failure is reported by a signal that wakes the host, not by polling
in a component tick. The exact ticket representation is internal initially.

## 5. Shared properties and a UI-owned reactive graph

### Ownership boundary

Adopt the following split. The prototype validates its semantics and cost; it
does not reopen a repository-wide `Rc`-to-`Arc` conversion as the default plan.

| Type or machinery | Ownership and contract |
| --- | --- |
| `Property<T>` | Thread-safe shared application value; clone, read, publish and drop safely on workers for supported transferable `T` |
| `LocalProperty<T>` | Explicitly non-Send/non-Sync property for owner-thread values and computations, including `Rc<ExpandedNode>` and local evaluator captures |
| Reactive graph | Dependencies, evaluators, subscriptions, transitions and effects remain on their owning UI thread; shared values enter through local graph projections |
| `ExpandedNode`, `NodeContext`, runtime and service registry | Keep local `Rc`/`Weak`, `RefCell` and `Cell` ownership where appropriate; never become worker-accessible through a shared property |

The initial shared-value bound is the existing value requirements plus
`Send + Sync + 'static`, allowing immutable shared snapshots. Phase 0 may relax
the `Sync` bound only with a storage/read API that proves it unnecessary. Do not
require these bounds of `LocalProperty` or of every component, factory or event
handler. Ordinary component state uses `Property`; local values opt into
`LocalProperty`, which is available to builders as well as engine internals.
Support that distinction in macro/type recognition rather than forcing local
values into an unsafe erased shared representation.

`Arc` belongs in shared value storage and transferable endpoint ownership. It
does not make a contained `RefCell`, erased local value or callback thread-safe.
Making `ExpandedNode` concurrent would additionally require synchronization of
parent/child relationships, mounting, caches, lifecycle callbacks and rendering
across the connected runtime. That work is outside this ticket. Future parallel
layout or rendering preparation can use immutable snapshots and versioned
results without sharing the mutable live tree.

### One graph, two property representations

Keep one dependency/evaluation system. Shared application inputs must feed local
computed properties, template expressions, repeat/conditional child lists,
layout and effects through the existing graph semantics. Do not fork two
disconnected reactive systems or duplicate every graph algorithm.

A shared handle owns synchronized value storage with stable identity and a
revision, not a bare key into the current thread's property table. Its graph
attachment holds the local dependency/evaluator state and a transferable wake
route. Shared storage must never own a local evaluator, `Rc<ExpandedNode>`,
`NodeContext`, or callback whose final drop could occur on a worker.

Local graph projections subscribe to published revisions. A worker publishes
a value and schedules invalidation; only the UI owner traverses dependencies,
evaluates formulas, changes children or invokes subscriptions. Generated
bindings and explicitly declared local dependency views read the projection
for the current graph settlement, so a concurrent writer cannot change a
source halfway through that settlement. A later revision schedules another
settlement. No global graph lock is held across evaluation or user callbacks.

### Read and publication contract

The starting contract for the phase 0 prototype is explicit about the change
from today's lazy `Property::get`:

| Operation | Proposed meaning |
| --- | --- |
| Shared source `get` / `read` | Read the latest published snapshot; never execute a UI evaluator or wait for UI processing. `read` invokes its callback against a retained snapshot outside storage locks. |
| Shared source `set` | Atomically publish a new value/revision, then request graph work; return does not mean layout or rendering has completed. |
| Shared source `update` | Atomic single-property read/modify/write; the user closure runs once, not silently replayed after contention. |
| Shared source `set_if_neq` | Comparison and conditional publication form one atomic operation. |
| Local computed `get` | May resolve dirty dependencies on the owner thread using local graph views. It cannot run from a worker. |
| Exported computed snapshot | `Published<T>` reads only the most recently published computed result; the producer/evaluator remains local. |

Shared `get` has the same freshness contract on every thread. A same-handler
source read observes its completed write unless another writer has subsequently
published. Reading a derived value from a worker does not promise that the UI
has processed the latest inputs. Do not overload shared `get` to synchronously
evaluate only when called on the UI thread. An explicit owner-thread local view
provides lazy evaluation when needed; exported computed snapshots cannot be
written by arbitrary workers.

Direct shared reads in arbitrary Rust code observe published state. Custom
computed closures must declare and read local dependency views to participate
in graph settlement; a shared snapshot read alone does not establish a reactive
dependency. Adapt code generation and programmatic computed-property APIs to
make this distinction usable without changes to declarative PAXEL syntax.

Single-cell atomicity does not promise transactions across unrelated
properties. Publish a single struct property when fields must change together.
Intermediate worker publications may coalesce before graph settlement: a
property represents current state, not a lossless event stream. Use the bounded
channels in section 6 when each item must be observed. UI-originated writes must
still settle before rendering, and local read-after-write behavior must be
specified and tested at handler/evaluation boundaries.

Before implementation proceeds beyond phase 0, finalize the local-view API,
snapshot/version algorithm, initial computed-result availability, and how
expression-bound component fields expose shared reads versus owner-only
rebinding. Specify atomic `update` reentrancy/panic behavior: no lost updates,
silent callback replay, or undiagnosed recursive-lock deadlock. Short
per-property synchronization may be needed; never wait for an executor, UI
callback or another graph while holding it. Do not carry storage guards across
`.await`. Audit `replace_with`, transitions, cutoffs, subscriptions, two-way
bindings and serialization against the split: graph rebinding and animation
scheduling remain owner-thread operations even when their values are shared.

The read-only export is `Property::published() -> Published<T>`. This does not
turn an expression-bound component field into a read-only field: regular
`Property<T>` retains mutation for existing two-way binding behavior. A reader
receives only the `Published<T>` wrapper; its local proxy cannot mutate the
source. Render backends receive plain resolved stroke/material records captured
from the UI graph, rather than reading shared nested fields during rendering.

### Attachment, disposal and lifetime

Property allocation lifetime is separate from view attachment and task
lifetime. Shared state can outlive a view. Detaching a view removes its local
subscriptions and invalidates its wake route; it need not destroy a retained
shared value. Worker publication racing detach must not access a dead graph.
Application/revision and attachment generations prevent stale notifications
from reaching a new app or a reused graph slot.

A retained raw shared property remains data, not authority to mount a node,
keep a service active or mutate a replacement application's new property.
Unscoped writes to retained shared state are not automatically cancelled by
unmount. Section 6 supplies guarded publication for operations that require
cancellation or replacement to revoke write authority. Shared allocations and
their value destructors must also participate in native cartridge code-lifetime
safety when they outlive the application; a thread-safe value may still have
drop code in a retired library.

### Safety and prototype gates

Make `LocalProperty` and every untyped handle into the local graph explicitly
non-Send/non-Sync. The current `UntypedProperty` is just a slot-map key whose
clone/get/drop consult thread-local storage; it must not become the shared
handle's backing representation. If a transferable erased value handle is
needed, give it a distinct representation that enforces shared-value bounds.
Neither `unsafe impl Send` nor wrapping the current handle in a mutex fixes
thread-local lookup.

Required proof before higher-level async APIs:

- Positive native `Send + Sync`, worker clone/get/set/update/drop checks for
  shared properties, including concurrent writers and retained value teardown.
- Compile-fail transfers of local typed/untyped handles, local contexts and
  node pointers through `thread::spawn`, Tokio `spawn`, and `Arc<Mutex<_>>`;
  erasure and conversion must not bypass the restriction.
- Shared source -> local computed value -> child/layout/effect propagation,
  with evaluator and callback thread assertions and stable settlement views.
- Publication during evaluation, disposal and app replacement; queued stale
  notifications, wake disarming races, and progress without periodic ticks.
- Debug/release checks for property aliases, cutoffs, subscriptions, transitions,
  two-way bindings and serialized-value reconstruction in the selected model.
- Replace the current wrong-thread diagnostic with successful publication
  coverage for the new shared representation and rejection coverage for local
  handles; incorrect values must never become expected behavior.

Measure UI reads/updates, graph fan-out, memory per property, worker bursts and
frame responsiveness against the current implementation. Coalesce pending
property invalidations by attachment rather than allocate one message per
write. No performance conclusion has been measured yet. If the prototype cannot
meet correctness or acceptable cost, bring that evidence back for a design
decision instead of silently making nodes concurrent or reverting to mandatory
callback-based state updates.

## 6. Task lifetimes and delivery into local state

### Default authoring and guarded publication

Ordinary application-lifetime data can be published through a cloned shared
`Property` from any executor. It does not need a completion callback or an
explicit public scope merely to set a value. Application/component-bound spawn
helpers register lifetime ownership internally; application work may outlive a
view, while component work is cancelled at its node's actual unmount.

Cancellation alone cannot prevent a worker that is already running from
calling raw `Property::set`. For replaceable or cancellable operations, expose
a guarded publisher created for the target shared property and the operation's
lifetime. It checks generation and commits atomically with respect to
invalidation; a check followed by an independent `set` is insufficient.

A guarded publisher has the shared value and transferable revocation state,
never the node, local scope controller or callback. After invalidation its
publication returns `Closed(value)`. A publication that committed before
invalidation remains a real write; cancellation cannot roll it back. UI cancel
or replacement invalidates the old operation before publishing its new state.
No later old-operation commit can overwrite that state. Direct raw property
clones intentionally do not provide this guarantee; do not claim that merely
spawning their owner in a scoped task revokes them.

The ergonomic helper supplies this guard automatically for result publication.
Explicit operation groups, completion callbacks and event channels remain
available for advanced use. Public names such as `async_scope` are provisional;
scope identity is required internally, but explicit scope construction is not
the default example for ordinary async property updates.

### Low-level completion and channel primitives

For work that must call UI-local code, introduce core APIs without a Tokio
dependency. This is the advanced callback path, not a prerequisite for shared
property publication:

```rust,ignore
let scope = ctx.async_scope();
let local_model: LocalProperty<LocalModel> = self.local_model.clone();
let completed = scope.completion(move |text: String| {
    local_model.update(|model| model.accept(text));
})?;

// Transfer only completed and owned request data to any worker/callback API.
// complete consumes the endpoint; it never runs the UI callback inline.
completed.complete("Ready".to_owned())?;
```

`NodeContext::async_scope()` attaches to `ctx.expanded_node`, matching the
existing context's node identity. It does not guess the lifetime of `&mut self`.
For component lifetime, obtain/retain a scope in that component's `on_mount`;
inline handlers may describe a child control. `AppContext::async_scope()`
explicitly requests application lifetime. Reusable child operation scopes
support replacing one request without cancelling unrelated work.

A completion endpoint is `Send` when its payload is `Send + 'static`. Its
callback is `FnOnce(T) + 'static` and remains in a UI-owned registry. Completion
consumes the producer, queues at most one result, and reports `Closed(T)` if
already invalidated. Success means accepted, not already delivered; unmount
may invalidate accepted work before its callback runs. Dropping an unused
producer signals closure so the UI registry releases its callback on the UI
thread. No shared producer object may own the UI closure or captured local
properties. A guarded publisher may own a shared property as described above.

For repeated events, a scope can create a bounded channel with a local
`FnMut(T)` callback and a subscription guard. Provide `try_send` with explicit
`Full(T)`/`Closed(T)`, and an executor-neutral async `send` that waits for
capacity. No blocking send on the UI thread and no silent loss. Dropping the
subscription or invalidating its scope closes the channel, wakes waiting
producers, and discards buffered deliveries. FIFO is guaranteed in accepted
queue order; concurrent producers do not gain an artificial total order before
enqueue. Shared properties already provide latest-value state; a separate
coalescing event-channel API is unnecessary for this ticket.

Bound resources explicitly: proposed initial defaults are 1,024 active
endpoints per application and a default channel capacity of 32. One-shot
endpoints reserve their single result slot when registered; registration fails
at the endpoint limit. Channel capacities have a per-app aggregate limit of
4,096 message slots. These are count limits, not byte limits on arbitrary Rust
values; services must bound large payloads. Expose configuration and errors.
Confirm/tune the defaults with the fixture before documenting them as stable.
Task registrations and guarded publishers also require bounded registration;
they cannot bypass resource accounting merely because they have no callback
endpoint. Phase 2 must set and test their configurable limits.

### Dispatch, reentrancy and wakeups

Local-state delivery messages carry application/revision identity, mount
generation, operation identity and payload. Property notifications carry their
graph attachment identity and published revision; unscoped application state
need not have a node/operation owner. Worker-facing objects may own shared
properties and transferable producers, never an engine pointer, node `Rc`,
UI callback, local property, or direct access to `pax_interrupt`.

At a safe update boundary:

1. Import a bounded snapshot of pending property revisions and settle
   already-pending structural invalidation, including unmounts. Property-driven
   removal must invalidate the removed node before its callback can run.
2. Snapshot eligible deliveries, then invoke live callbacks on the UI thread.
   Release inbox/registry locks and tree/application borrows before calling user
   code. Validate scope identity immediately before each callback.
3. Import that callback's UI writes and settle its effects before invoking
   another callback whose target might have been removed by it. Then continue
   normal lifecycle, layout/native-message and rendering work.
4. Deliveries enqueued by callbacks are deferred to a later drain. Concurrent
   worker publications beyond the imported revision remain pending for a later
   settlement; a busy producer cannot force an unbounded settle-until-idle loop.
   Keep the existing clock/lifecycle contract; no new timer clock is introduced.

Start with a configurable drain budget of 64 callbacks or 2 ms, whichever
comes first. Time is checked between callbacks; a single slow callback cannot
be preempted and remains application responsibility. Remaining work schedules
another host turn so input/rendering are not starved.
Schedule ready channels round-robin, taking one item per channel per rotation;
ready one-shot endpoints join the same scheduler. A single busy producer must
not consume every drain ahead of other ready endpoints. Coalesced property
imports also need a measured work budget and fair scheduling alongside message
delivery, so property bursts cannot starve input or lossless channels. Their
pending storage is bounded by live attachments, not publication count.

An unwinding UI callback panic is an application-fatal error: stop further
delivery and enter host teardown rather than continue with partially updated
state or unwind through native FFI. As elsewhere, aborting panics are not
recoverable. Adapter task panics use the separate result contract below.

Every transition from no pending property or delivery work to pending work
requests a chassis wakeup. Use a coalesced pending bit/generation with a recheck
when disarming to avoid lost wakes. Do not rely on continuous display ticks
for correctness.
This is compatible with the separate [demand-driven frame scheduling exploration](demand-driven-frame-scheduling.md)
without making an idle-scheduler rewrite part of PAX-1013.

- **Apple:** register a host-owned wake endpoint at startup. A worker signal
  schedules main-queue processing; the main-thread host resolves the current
  application generation and runs a safe update. Never send an engine pointer
  to a worker or reconstruct the engine there.
- **Web:** a property publication or local completion requests a microtask/frame
  flush through the chassis. Respect frame-in-progress guards and suspension.
  A later resume drains retained valid work. Background tab timing remains
  browser-controlled; no delivery deadline is promised while suspended.
- **Inactive/destroyed host:** wake signals become no-ops; endpoints and graph
  attachments are closed. Retained unscoped shared data does not keep the host
  alive. A queued wake validates identity before touching any engine state.

`NativeInterrupt` remains the platform-input protocol. Share its host scheduling
and serialized execution boundary where appropriate; do not encode arbitrary
Rust values/closures in JSON/flexbuffers or pretend a custom event name is a
cross-thread transport. Existing `dispatch_event` behavior need not change;
a UI completion callback may invoke ordinary application code or dispatch a
named event once on the correct thread.
Registered `async fn` handlers remain unsupported. Add an actionable diagnostic
directing builders to bound task helpers rather than silently discarding a returned
future; do not implicitly await or reinterpret handler return values.

### Cancellation contract

Invalidation, publication and task cancellation are distinct:

- Invalidating a scope immediately prevents future callback starts, clears its
  UI callbacks on the owner thread, closes its producers and revokes guarded
  publishers. It does not revoke arbitrary raw shared property clones.
- A cancellation request additionally asks tracked background work to stop.
  It cannot undo an already-running callback or preempt arbitrary blocking code.
- Cancelling an operation while its callback result is queued suppresses that
  result. An already-started UI callback finishes. A guarded shared publication
  that has already committed is not undone, even if its UI projection is pending.
- Replacing a request creates a new operation identity and revokes the old
  guarded publisher before writing replacement state. A prior guarded request
  cannot overwrite the replacement even if it finishes later.
- Scope lifetime ends when the node actually unmounts. An exit-retained node
  remains mounted until removal; applications can explicitly cancel sooner
  when their semantic “leave screen” action occurs.

## 7. Tokio adapter and alternative executors

Create an optional native adapter crate, provisionally `pax-tokio`, depending
on Pax's shared-property and core lifetime/delivery interfaces.
`pax-runtime`/`pax-runtime-api` do not depend on Tokio; `pax-kit` does not enable
it by default. The adapter enables only its required Tokio features;
applications enable additional I/O features.

`TokioService::owned(Runtime)` participates in shutdown and checks that the
supplied runtime has the multithread flavor; an undriven current-thread runtime
returns a configuration error. A borrowed `Handle` mode leaves runtime lifetime
and continuous driving to the external owner. Both use explicit handles; Pax
does not implicitly enter a Tokio context around every UI callback. The adapter
does not infer that a borrowed current-thread runtime is being driven merely
because it can obtain its handle.

Provide bound task helpers so the ordinary authoring path does not construct
an explicit scope. For application-owned data with intentionally unscoped
publication, a helper can monitor a task that captures a shared property:

```rust,ignore
let app = ctx.application();
let service = app.service::<pax_tokio::TokioService>()?;
let status = self.status.clone();

let task = service.for_application(&app)?.spawn(async move {
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    status.set("Ready".to_owned());
})?;
```

For node-owned, cancellable results, the helper supplies a guarded publisher:

```rust,ignore
// ctx identifies the intended owner node, e.g. obtained in its on_mount.
let service = ctx.application().service::<pax_tokio::TokioService>()?;
let task = service.for_node(ctx)?.spawn_into(
    &self.request_state,
    async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        RequestState::Ready("Ready".to_owned())
    },
)?;
```

`spawn_into` does not give its future an unguarded target clone. It publishes
the returned value only through the registered operation guard. Explicit cancel
and replacement revoke that guard before requesting task abortion. A replacement
helper/operation slot performs revoke, replacement-state publication and new
registration in that order; builders must not need a racy check-then-set pattern.
Retain a component-bound helper acquired during `on_mount` when later handler
contexts identify child controls. `for_node` uses the supplied context's node,
not an inferred owner of the handler's `&mut self`.

Native futures are `Send + 'static`; `spawn_into` values meet shared property
bounds. A separate `spawn_with_completion` helper accepts `Send + 'static`
results and a local callback for manipulating local state; that callback may
capture `LocalProperty` or a local context. Both paths register lifetime and
publication/delivery authority before execution, monitor the task, and release
tracking on finish. Prepared candidates stage both paths until activation.

Task controls expose observable completion/error/cancellation status, and the
application error sink observes unhandled task panics even if the caller drops
the control. The callback path's `TaskOutcome<T>` distinguishes completed `T`
(including user `Result<T, E>`) from an unwinding panic. `spawn_into` publishes a
normal returned value; a panic reports task failure and does not invent a value
of the application's type. The fixture observes task status to show that
failure instead of remaining silently in Loading. Cancellation suppresses
uncommitted guarded publication and unstarted callbacks; the initiating UI
action sets its cancelled state after revocation.

Dropping a task control does not detach work from its owning lifetime, which
continues to track it. Application-lifetime work is selected deliberately. A
raw Tokio handle remains available for advanced code; shared property updates
still work from it, but tasks spawned outside the adapter are not automatically
tracked, cancelled or staged for candidate activation. The documentation must
say so, including that direct raw-property writes remain unguarded.

For latest-value progress, a task uses a guarded property publisher. For
lossless streaming work it uses the bounded UI channel; closing the lifetime
wakes a producer waiting for capacity. Recoverable panic reporting requires
unwinding; `panic=abort` remains process termination.

A second proof uses a dedicated service thread that continuously drives a
current-thread runtime. If it needs `LocalSet`, construct and retain that set
on that service thread. Shared properties, guarded publishers and Send message
data may cross to the UI; non-Send objects created inside the service never do.
This is a supported service pattern, not a second UI executor.
Register that dedicated owner as a managed service before the borrowed-handle
adapter so scoped work stops before its runtime driver. LocalSet-specific
commands create/access local state on the service thread and publish shared
values or return data through core completions. Native task helpers do not
accept non-Send UI captures in their futures.

The browser adapter uses its local executor and the same shared-property,
guarded-publication and lifetime contracts. Its future need not be Send because
it is polled on that UI thread; this does not make a local handle transferable
to a native worker or promise browser Web Worker/shared-memory support.

An executor-neutral scoped-registration primitive lets other executor adapters
supply cancellation and completion signals. Prove it with one ordinary Rust
thread/callback producer; do not implement adapters for every async ecosystem.
For CPU/blocking tasks, use bounded concurrency and cooperative cancellation.
A started Tokio `spawn_blocking` operation cannot be forcibly aborted.

## 8. Shutdown and hot reload

Use a state machine: `Preparing -> Active -> Closing -> Closed`, with failed
preparation also entering `Closing`. Host suspension is independent of this
state; it is not automatic destruction of the app.

On close, replacement, or discarded preparation:

1. Enter Closing; reject new work. Invalidate app/node/operation delivery
   scopes, guarded publishers and graph wake attachments before teardown can
   race incoming results. Retained unscoped properties may remain usable as
   data, but cannot reactivate this app or reach its replacement.
2. Unmount the tree and notify `stopping` once if configuration succeeded.
   Existing services are readable during these callbacks, but closed scopes
   cannot launch new application work.
3. Begin managed-service shutdown in reverse registration order, awaiting each
   ticket asynchronously before shutting down its earlier dependencies; collect
   errors. Register dependencies before their dependents. Then release services
   and callbacks on their documented owning threads. A timed-out service retains
   its required dependencies; the UI deadline does not justify dropping them.
4. Finish host disposal after acknowledgements, or report a configurable
   shutdown deadline. Native default proposal: two seconds. Waiting occurs
   through host wakeups, never a join/block_on on the UI thread.

The owned Tokio adapter transfers waiting/dropping its runtime to a shutdown
worker, cancels tracked tasks and reports when they actually stop. Deadline
expiry means “still running”, not successful cancellation. Retain any code and
resources still needed by outstanding tasks. In native logic reload, never
`dlclose` a cartridge while its futures, callbacks, shared-value allocations,
payload destructors or shutdown work can still execute; preserve the existing
retired-library safety until true quiescence can be established. A borrowed
runtime is not shut down; only the adapter's tracked work, publishers and
endpoints are closed.

A failed candidate leaves the old app active. On successful logic activation,
revoke old delivery scopes, guarded publishers and graph attachments before new
UI delivery is enabled; no service or completion identity is reused. New app
state is not implicitly aliased to old shared allocations. The old shutdown may
finish after new activation. Pax-only reload preserves application services
and retained shared state but cancels scopes and removes local subscriptions
for nodes that actually unmount. Retained nodes retain their live scopes.
Existing web/macOS logic-reload opt-in and iOS/iPadOS restart requirements remain.

An abrupt process kill, browser tab close or mobile suspension cannot guarantee
asynchronous cleanup. Do not use shutdown hooks as the only persistence path.
Backgrounding follows OS/browser execution constraints; resume must safely
handle pending results without promising work continued while suspended.
In particular, native mobile support does not imply desktop process APIs or
permission to keep arbitrary network tasks running in the background.

## 9. New proving fixture: Async Workbench

Create **`examples/src/async-workbench`** during implementation phase 1, with
`#[main]`, the proposed application hook, and the normal generated chassis.
It must not supply a custom `main`, use app-level channel polling, or require
external services, credentials, network accounts or an additional process.

### Concept and spatial design

A compact laboratory for seeing work leave and return to the UI. Use an ink
background, warm neutral panels and restrained cyan/amber state accents.
Typography separates the title, readable request state, and small diagnostic
numbers. Bundle/reuse repository assets; do not make the proof depend on a
remote font. Motion comes from a small timeline-driven activity indicator and
progress transitions, not a bespoke async polling loop.

Textual wireframe:

```text
Async Workbench                  [platform / executor] [app generation]
[Interact: +1]  Clicks 12         [responsive animation]

[Request panel mounted ✓]        Lifetime / bounded event history
  [Run delayed result] [Fail]      started #14
  [Cancel] [Replace request]       cancelled #13
  [Stream progress] [Stop]         result #14 accepted
  [Publish shared state]          shared -> local computed -> layout
  [Burst updates / events]        coalesced state / lossless channel
  [Run local I/O]                 [Clear history]
  Loading… / Result / Error
  progress bar

[Unmount request panel] [Remount]   active work / pending / stale count
```

At >=900 px use two columns; below 900 px stack the request and history panels.
At narrow phone widths keep primary actions full-width and the history below
the result. The interaction counter and mount controls remain outside the
conditional request panel, so they can prove responsiveness and remount it.
Reserve stable space for loading/error/result text to avoid layout jumps. Use
native Button controls and touch-appropriate sizing. At rest, stop the optional
activity animation for the wakeup test so it cannot conceal missing wakes.

### Planned source layout

```text
examples/src/async-workbench/
  Cargo.toml
  src/lib.rs, src/lib.pax          root and independent interaction counter
  src/application.rs              setup hook and service registration
  src/request_panel.rs/.pax        node-bound requests and local computed views
  src/shared_state.rs              shared values and coherent result records
  src/work_service.rs              common request/result types and interface
  src/work_service/native.rs       Tokio execution and local I/O proof
  src/work_service/web.rs          browser futures and same-origin fetch
  public/sample.json               deterministic browser I/O payload
  README.md                       run commands, expected scenarios and limits
```

Follow current example Cargo/build conventions, including `.nvmrc` tooling
when needed. Keep native Tokio dependencies under target-specific Cargo
sections. Start with two native worker threads. Web uses browser timer/fetch
futures and the same property/lifetime/channel semantics; its backend label says
“Browser futures”, never “Tokio”.

### Scenarios

| Scenario | Required observable result |
| --- | --- |
| Setup | Setup count is one per app instance and precedes root mount; remounting the request panel does not create another runtime |
| Direct shared publication | A native worker captures an application-owned `Property` clone and sets it without an explicit scope or UI callback; a local computed value and layout/children update on the UI thread |
| Delayed success/error | A bounded 600 ms async timer returns a typed result through `spawn_into`; clicking the independent counter works before completion; task panics show an observed failure |
| Replace | Start slow A, then fast B through an operation slot; B remains visible after A finishes; a retained old publisher is rejected even when A ignores cancellation |
| Explicit cancel | Revoke authority before publishing Cancelled; a queued callback or later guarded write cannot overwrite it; a previously committed value is not described as rolled back |
| Unmount/remount | Hide the panel mid-request; old guarded publication and callbacks are rejected; application-owned shared data survives and a new panel attaches without old subscriptions |
| Latest-value progress | Guarded property updates coalesce under a burst while the eventual latest value appears and input stays responsive; no claim that every revision rendered |
| Lossless stream | A bounded channel delivers every accepted item in order; a controlled burst proves Full/backpressure, producer closure and input fairness |
| Concurrent updates and snapshots | Two controlled workers atomically increment one property without lost updates; a struct-valued result stays coherent; diagnostics distinguish published and UI-observed revisions |
| Native I/O | Tokio binds an ephemeral loopback TCP listener, serves a small fixed response and reads it with timeouts; sockets close after the scenario |
| Browser I/O | Fetch `sample.json`, served from `public/sample.json` at the app's base URL, with a controlled browser-timer delay and error case |
| Blocking work | Bounded native work demonstrates cooperative stop and safe suppression when work cannot stop immediately |
| Plain thread / local callback | An ordinary Rust thread proves both shared property publication and a completion that updates `LocalProperty`; only the latter executes a UI callback |
| Host close/reload | Old-generation callbacks, guarded writes and graph notifications are rejected; retained raw data cannot reach the replacement app; cooperative tasks/services quiesce and setup occurs once for the new app |

The event history is bounded (latest 100 records) and contains app, mount and
operation identities, property publication/observation revisions and monotonic
timestamps. It is a diagnostic fixture, so thread/executor/lifetime facts belong
here. Do not add these internals to
ordinary product templates. Fixture counters supplement regression assertions;
visual logs alone do not prove correct teardown.

Native-only scenarios are explicitly unavailable on the browser with a brief
reason. iOS tests use the native backend, with UI and app-lifecycle behavior
checked on the actual target; a macOS pass is not an iOS pass.

## 10. Implementation phases and gates

| Phase | Work | Exit gate |
| --- | --- | --- |
| 0 — Property boundary | Implement/prove shared `Property`, local `LocalProperty` and one graph; finalize reads, atomic mutation, graph projections and owner-only APIs; audit generated types/baking | Shared transfers succeed; local transfers fail; mixed graph and disposal races pass in debug/release; correctness and performance evidence supports the cut |
| 1 — Application hooks and services | Attribute/codegen, setup ordering, application registry, staged lifecycle/error contract; create fixture shell with setup/remount and mixed-property checks | Repository synchronous examples still work after deliberate API updates; native and web fixture mount with one setup per instance; root dependencies do not run hooks |
| 2 — Publication and native Tokio | Chassis property wakeup, guarded publishers, bound task helpers, local one-shot endpoints, owned/borrowed adapter and task monitoring | macOS fixture proves direct worker writes, local graph effects, responsive input, replace, cancel, unmount and ordinary-thread publication/callback delivery |
| 3 — Browser and streaming | Browser wake backend, bounded channels, guarded progress publication, browser service, fairness/backpressure | Web fixture proves shared properties/lifetimes and fetch; latest-value and lossless semantics differ as specified; no native Tokio features in Wasm |
| 4 — Lifecycle completion | Managed shutdown, prepared candidates, logic reload, retained shared allocation/code lifetimes, blocking-task limits; dedicated current-thread service proof | No stale callbacks, guarded commits or graph access after invalidation; cooperative cleanup completes; timeouts accurately report unfinished work |
| 5 — Target/release qualification | Native I/O, iOS/iPadOS validation, debug/release-baked coverage, public docs and API generation | Acceptance matrix passes for each claimed target; remaining target limits named explicitly |

Keep phases individually reviewable. Do not broaden into a generic dependency
injection framework, alternate renderer, async template language, arbitrary
async event-handler syntax, demand-driven scheduler, concurrent `ExpandedNode`
tree, repository-wide `Arc` conversion, or custom-main API.
Introduce the fixture early and extend it at each phase rather than waiting
until the end for the first visible proof.

## 11. Regression and platform acceptance matrix

Prefer deterministic barriers, controlled clocks and queue inspection for race
checks. A sleep followed by “nothing happened” is insufficient lifetime proof.
Use small unit/compile tests before any full app build.

| Area | Required checks |
| --- | --- |
| Property ownership | Shared typed/erased representations uphold their Send/Sync bounds; local typed/untyped handles, node pointers and contexts reject transfer; worker clone/drop never touches UI TLS |
| Publication | Concurrent `set`/`update`/`set_if_neq`, read freshness, update panic/reentrancy contract, same-handler reads, coherent struct values, immutable snapshots, bounded coalescing |
| Mixed graph | Shared source -> local computed -> template/layout/children/effect; owner-thread evaluation/drop; publications during settlement defer safely; cutoff/alias/transition/two-way-binding semantics |
| Guarded writes | Invalidation races publication at a defined commit point; blocked/non-cooperative old worker cannot overwrite replacement; prior committed writes are not rolled back; raw property clones remain explicitly unguarded |
| Hook/codegen | No-hook compatibility; explicit root hook; dependency-main suppression; setup before root defaults/mount; duplicate/missing services; failure unwinding and cleanup; candidate discard; release hook reachability |
| Endpoint | One-shot at-most-once; closed producer; unused producer drop; bounded registration; callback/thread ownership; callbacks may enqueue without reentrancy |
| Lifetime | Unmount before send, after enqueue and before dispatch; remount ID reuse; replacement requests; callback A unmounts callback B's target; retained exit node policy |
| Wakeup | Property and message producers race host disarm; signal during frame/close; paused periodic scheduler resumes from either wake path independently; multiple apps route correctly |
| Streams | FIFO, capacity/full, fairness across producers, waiting send wakes on space/close; cancellation drops buffer without callbacks; aggregate capacity limits |
| Executors | Owned and borrowed multithread Tokio; timer and loopback I/O; continuously driven current-thread service/LocalSet; normal-thread producer; browser futures |
| Shutdown | Cooperative finish, panic/error observation, already-started blocking job, expired deadline, borrowed runtime remains usable, retained shared values have safe destructor/code lifetimes, resources reclaimed when owners release them |
| Reload | Invalid candidate leaves old live; discarded prepared runtime shuts down; old accepted result/notification cannot hit new state; Pax-only remount invalidates affected scopes/attachments while retained shared state survives |
| Release | Native and web fixture execute with baked program data; release disables both reload lanes; ordinary synchronous fixture still runs |

Run macOS debug first, then web debug; reuse caches within this worktree and
build targets sequentially. Add optimized release checks after behavior is
stable. Exercise iOS/iPadOS using an existing available destination; do not
create new VMs or duplicate simulator caches merely to fill the matrix.
Unexecuted destinations remain explicitly unqualified, not presumed passing.

Use Pax dev screenshots/interaction/inspection where the target supports them.
During a held request, click the independent counter and verify its changed
state before releasing the request barrier. Separately stop periodic frame
scheduling in a test harness and prove both a direct property publication and
a local completion independently wake and update the UI.
A continuously animated screenshot does not prove the wake mechanism.

Provide the running target preview for each fixture iteration. Record memory,
shared allocations, local graph attachments, pending endpoints and task counts
over repeated cancel/remount runs to detect unbounded growth. Performance claims
require measurement; no universal frame
rate or completion-latency promise is part of this spec.

## 12. Compiler, baking, and implementation map

| Area | Expected changes |
| --- | --- |
| `pax-runtime-api` | Shared `Property`, non-Send `LocalProperty`/local erased handles, local graph views, guarded publishers and executor-neutral lifecycle/channel types |
| `pax-runtime` | Keep node/runtime ownership local; mixed graph attachments, app context/services, publication import and callback phase, generation invalidation, managed teardown |
| `pax-macro` | Recognize both property forms and generated bounds; explicit application attribute, active-root ownership, hook invocation in generated native/web startup |
| `pax-compiler` | Property binding/factory codegen, host lifecycle/wake endpoints, diagnostics and fixture packaging; cartridge/bootstrap coordination |
| `pax-chassis-common`, Apple Swift hosts | Main-queue wake routing, startup errors, app shutdown and candidate/revision ownership |
| `pax-chassis-web`, web TypeScript host | Browser wake routing, non-reentrancy/suspension, app disposal and reload |
| New optional `pax-tokio` | Native bound task helpers, guarded result publication, local completion monitoring, owned/borrowed runtime shutdown |
| `examples/src/async-workbench` | Progressive proving fixture and local data |

Prefer direct Rust hook references generated from the root attribute. Services,
executors, endpoints, property allocation/attachment identities, synchronization
state and pending tasks are process state and must not be baked or serialized.
Serialize supported property values and required declarative binding metadata;
reconstruct fresh shared storage and local graph attachments when loading. Do
not add a runtime manifest field just to rediscover a Rust hook that codegen can
call directly.

Nevertheless audit `pax-manifest`'s `program_ir`, `binary`, `rust_manifest`,
cartridge descriptors/templates and release roundtrips: startup now crosses the
compiler/runtime boundary even if the binary schema stays unchanged. The
property split also touches generated component/type descriptors, erased
conversions and expression bindings. Audit both rich/debug and baked/release
factory paths; prove literals, computed bindings and nested component values
reconstruct with the correct ownership. If implementation needs hook or property
kind metadata in the manifest, classify source-only vs execution-required data
explicitly and update the release path accordingly.
New native host callbacks/exports need ABI/version/capability coordination;
stale ejected interfaces must get a diagnostic/re-ejection instruction rather
than silently running without wake/lifecycle support.

## 13. Documentation and handoff

No new public book chapter is proposed. Update existing canonical sections as
the corresponding behavior is implemented:

- **Events and Rust:** synchronous handlers, launching work, receiving results,
  task errors, bound task helpers, guarded cancellation and unmount semantics;
  explicit completion callbacks for local state.
- **State and Properties:** shared `Property` versus `LocalProperty`, snapshots
  versus local computed reads, atomic updates, coherent struct publication,
  graph settlement and the distinction between state and lossless events.
- **Components and Composition:** application services versus lexical stores.
- **How Pax Runs:** application lifecycle, UI-owned graph and tree, property
  import/wake/delivery order, attachment ownership and teardown.
- **Targets, Build and Deployment:** native/browser capabilities and adapter
  dependency configuration; ejected-host compatibility if relevant.
- **Developer Workflow:** reload scope behavior and the fixture validation flow.
- **Getting Started / generated builder instructions:** a short link to setup
  hooks and async services; keep the default app free of a mandatory executor.

Update `///` source comments and regenerate affected API pages through
`pax-docs` tooling. Keep documented snippets and the fixture aligned; compile
snippets as part of fixture builds. Run `mdbook build pax-docs/book` and link
checks. Record additional authoring pain points as encountered. Adding the
fixture to bundled `pax-cli create --example` choices is a final opt-in release
step after it becomes a supported example; rebuild the required source bundle
artifacts then. Documentation publication is separately authorized work.

This revision is design-only: do not teach the new API as available in those
articles until implementation and its checks establish the behavior. The
investigation README links to this spec as the current direction; its original
local-property delivery sketch remains historical evidence.

The handoff records passed/failed/unrun matrix entries, dependency and binary
format impact, an appropriate running preview, known target restrictions and
remaining work. Keep PAX-1013 In Progress during implementation, use In Review
when the agreed support contract is ready, and leave commits/closure to Zack.

## 14. Design decisions and remaining prototype work

1. Adopt explicit `#[application(Type)]` with synchronous configure/started/
   stopping hooks; make per-application services distinct from lexical stores.
2. Keep Tokio in an optional adapter; support owned and borrowed runtimes.
3. Make shared `Property` the default for transferable application state and
   `LocalProperty` the explicit cut for local values/computations. Retain one
   UI-owned graph and local `ExpandedNode`/runtime ownership; no broad `Arc`
   refactor. Prove this boundary before higher-level async APIs.
4. Make direct shared publication and bound task helpers the ordinary authoring
   path. Keep task lifetime separate from property lifetime; guarded publishers
   enforce replace/cancel authority, and explicit completion/channel APIs cover
   local callbacks and lossless events. Public `async_scope` is optional advanced
   surface, not required boilerplate.
5. Make macOS the first Tokio proof, then web parity, then native mobile and
   release qualification; create Async Workbench in phase 1.
6. Define “full” using section 1's contract: native Tokio plus safe publication and
   lifetime integration, with browser futures and explicit platform limits.
7. Accept breaking changes and update repository consumers directly; no
   compatibility shims or migration work.

The ownership direction and APIs are implemented. The local-view/read API,
atomic-update behavior and snapshot/settlement algorithm are covered by tests;
measured costs are recorded in the probe README. The fixture README records
which platform acceptance checks have run and which still require interactive
native verification. API spelling and queue limits can be refined
without weakening ownership, boundedness, wakeup or lifetime guarantees. Named
template event semantics remain separate from cross-thread state publication.

## Primary references

- [Rust Arc thread-safety contract](https://doc.rust-lang.org/std/sync/struct.Arc.html#thread-safety)
- [Tokio: bridging synchronous and async code](https://tokio.rs/tokio/topics/bridging)
- [Tokio 1.53.1 runtime ownership and shutdown](https://docs.rs/tokio/1.53.1/tokio/runtime/struct.Runtime.html)
- [Tokio LocalSet](https://docs.rs/tokio/1.53.1/tokio/task/struct.LocalSet.html)
- [Tokio blocking-task cancellation limits](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)
- [Tokio Wasm support](https://docs.rs/tokio/1.53.1/tokio/#wasm-support)
- [Browser-local futures](https://wasm-bindgen.github.io/wasm-bindgen/api/wasm_bindgen_futures/fn.spawn_local.html)

Primary APIs were checked during the investigation. The design's APIs now have
implementations; use Async Workbench and the canonical authoring/API docs for
buildable examples. The fixture README records the checks actually performed
and the remaining platform qualification limits.
