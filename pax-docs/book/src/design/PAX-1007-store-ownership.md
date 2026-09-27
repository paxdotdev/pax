# PAX-1007 — Component-local store ownership

Status: implemented and checkpointed; public documentation approved for drafting
on 2026-09-27. The proposal below records the original design discussion against
base `090348859` on 2026-09-25. The approved decisions use `provide_store` /
`with_store` without compatibility aliases, and clear local state and providers
at final unmount. See the current
[public store guide](../components-composition.md#shared-state-farther-down-the-tree)
and API reference for the implemented contract.
Issue: [PAX-1007](https://linear.app/paxdev/issue/PAX-1007/define-and-implement-component-local-store-ownership-and-isolation).

## Recommendation

Give each mounted runtime node ownership of the stores it explicitly publishes.
Resolve stores through logical template ancestry, independently of expression
scope and rendering ancestry. A component's own mount handler publishes on that
component instance; primitives such as Path use the same mechanism.

For caller-supplied content, the receiving component is a provider ancestor.
Expressions and handler `self` keep their existing caller scope. Private
wrappers around a slot do not become provider ancestors of the projected child.
This gives Table its intended context behavior without making slot layout an
implicit state-sharing mechanism.

Ordinary fields remain per-instance. Sharing requires `bind:`, a deliberately
cloned `Property`, or publishing a store. Store registration exports only the
supplied value, not all fields on the component. This is an ownership convention,
not a security boundary against Rust code with explicit handles.

## Evidence on this base

The issue is In Progress, with Design/Bug labels, in Pax Launch. It has no
comments and relates to PAX-869. The website checkpoint
`0b5c68be105278829330da65f5e32be741826899` was inspected read-only.

The failure is in the runtime contract:

- `api.rs:120` inserts into `NodeContext.local_stack_frame`;
  `expanded_node.rs:2889` initializes that field from the node's incoming stack.
- `properties.rs:1577` replaces the entry keyed by Rust `TypeId` in that frame.
- `expanded_node.
- rs:2415` runs user mount handlers before the instance's mount;
  `component.rs:89` creates the private template expression frame afterward.
- `component.rs:60` creates received content using the caller's expression
  environment, but sets its `template_parent` to the receiving component.
- `PathInstance::handle_mount` already isolates its child store by explicitly
  pushing a fresh frame. Table and ComboBox use the unsafe mount registration.

A temporary test in `cartridge/initialization_tests.rs` mounted two copies of
one component template with a shared incoming frame. Each mount published its
own `Property<f64>`. After setting their values independently:

```text
provider fields:                  (1, 2)
descendant store reads:            (2, 2)  [expected (1, 2)]
parent store read:                 Some(2) [expected absent]
first child after second unmount:  Some(2) [expected 1]
```

Command: `cargo test -p pax-runtime --lib pax_1007_sibling_mount_store_isolation_reproduction --offline`.
The isolation assertion failed as expected. The temporary test was removed;
the unmodified initialization suite then passed all 9 tests. This establishes
sibling overwrite, upward leakage, and stale registration after provider
unmount, independently of rendering. It does not validate a proposed fix.

## Alternatives

| Model | Benefit | Cost or semantic problem |
| --- | --- | --- |
| Add an empty expression frame per component | Small initial patch; sibling registrations separate | Couples stores to expression-frame creation; projection, primitives, and teardown still need special rules |
| Separate stores, follow lexical component ownership | Stable under projection | A caller-authored Row cannot see its receiving Table without a second explicit projection/provider API |
| Separate stores, follow render parents | A provider surrounding a slot can affect its rendered contents | Layout wrappers and slot moves change context; mount-time cloned handles would require rebinding on moves |
| **Separate stores, follow logical template ancestry** | Matches authored nesting; supports Table, Path, nested providers, and stable slot moves | A provider hidden around a slot cannot implicitly inject into caller content |
| Require an explicit provider element for every store | Very visible sharing boundary | Adds a new authoring construct and compiler/baked obligations; still needs a projection rule |

Recommend the fourth model. The existing immutable `template_parent` edge is a
candidate implementation of this relation, not a reason to select its semantics.
Tests must establish those semantics for forwarding, control flow, and primitives.
An explicit provider component can already be authored with a mount method and
slots; no new DSL node is needed.

## Proposed contract and API

1. A registration belongs to the exact runtime node identified by its
   `NodeContext`, for that mount generation. It never writes an incoming frame.
   Component authors register in their own `@mount`. A handler's Rust `self`
   and event-node context can differ; the context determines provider ownership.
2. Lookup starts at that node, then walks logical parents. The nearest matching
   Rust type wins; ancestors without that type are skipped. Siblings and
   descendants are never searched. The provider can read its own registration.
3. A published store is visible to both private template descendants and
   authored/received descendants. It crosses component boundaries only because
   publication was explicit. Use a distinct newtype for each purpose.
4. Lookup returns an error when missing, when the context's mount has ended,
   or when the requested store is already mutably borrowed. Keep closures short;
   clone property handles before triggering work that may look up stores again.
5. Publishing the same type again on the **same owner** replaces its entry.
   Future lookups see the replacement. Previously cloned handles keep referring
   to their original state; replacement is not a reactive retargeting operation.
   Prefer updating properties inside a stable store. No automatic subscription
   to the provider map is added.

Proposed names: `provide_store(store)` and `with_store(|store: &mut T| ...)`.
Keep `push_local_store` and `peek_local_store` as documented compatibility aliases
with the corrected semantics. There is no legacy fallback to expression stores:
such a fallback would preserve sibling leakage. New API errors should distinguish
missing, expired-context, and borrow-conflict cases; legacy lookup can map them
to its existing `Result<V, String>`. Registration through an expired context
must fail rather than attach to a later mount. Exact error type/signatures can
be settled with the API naming decision.

Illustrative proposed API (not available yet):

```rust
pub struct SelectionStore(pub Property<usize>);
impl Store for SelectionStore {}

// In Picker's own mount handler; selected is a field on that Picker.
ctx.provide_store(SelectionStore(self.selected.clone()))
    .expect("Picker is mounting");

// In a descendant's handler.
let selected = ctx.with_store(|s: &mut SelectionStore| s.0.clone())?;
selected.set(2);
```

```pax
<Picker> <Choice/> </Picker>
<Picker> <Choice/> </Picker>
```

Each Choice finds its own Picker. An inner Picker shadows the outer Picker only
within the inner subtree. To synchronize two Pickers intentionally, bind both
`selected` inputs to the same property; stores then expose those explicit aliases.

Projection example:

```pax
// Caller template
<Table rows=3 columns=2>
    <Row y={self.row_index}/>
</Table>
```

| Question at Row | Answer |
| --- | --- |
| Where does `self.row_index` resolve? | Caller expression scope |
| Where does `TableContext` resolve? | Row → this Table → caller's logical ancestors |
| Does a private provider wrapped around Table's `slot()` override it? | No |
| Does moving that slot between private Groups change the answer? | No |

Forwarding received content through another component's slots preserves its
original provider ancestry. If a wrapper wants to expose an inner provider's
state to its received content, it must publish that state on its own boundary
or pass explicit properties. This tradeoff needs agreement before implementation.

## Identity, disposal, and reload

| Operation | Proposed result |
| --- | --- |
| Keyed repeat reorder | Reused instance keeps its store and property handles; lookup is unaffected by index changes |
| Unkeyed repeat reuse | State follows the reused position, matching existing component identity |
| New repeat key / new conditional instance | New provider generation and registration |
| Exit retention and rescue | Store lives until actual unmount; rescue before unmount preserves it |
| Slot move / render reparent | Provider ancestry stays fixed, even if the new visual parent has a store of the same type |
| Move to a different logical owner | Remount under that owner; no implicit transfer of provider identity |
| Final unmount | Descendant and provider unmount handlers can still use the old scope; then close it and clear its registry |
| Remount, including reuse of an ExpandedNode allocation | Fresh generation; old contexts cannot resolve into it |
| Same-instance template/property rebind | Preserve registry and stable handles; if a rebind changes a published property alias, refresh/remount that provider and affected consumers rather than leave a stale alias |
| Template subtree/tree recreation | Dispose and re-register providers in the recreated region; unaffected owners retain theirs |
| Successful logic reload | Fresh stores in the new application revision; do not carry `Any`, Rust `TypeId`, closures, or property handles across the module boundary |
| Failed logic build/activation | Existing mounted revision and its stores continue according to the current reload coordinator |

The partial-rebind alias case needs dedicated implementation coverage: current
`recreate_with_new_data` can rebind properties without rerunning mount. This
proposal does not promise automatic migration of arbitrary store internals.
If retaining opaque published aliases cannot be proved safe for a reload class,
conservatively remount the affected provider subtree for that class and document
the state reset. Preserve in-place reload for changes proved to retain aliases.

Engine-owned provider links must be weak or otherwise acyclic. Cached contexts
carry a mount-generation identity; they must not keep old provider registries
alive or silently fall through an expired boundary to a different provider.
Already cloned `Property` handles remain valid Rust handles after unmount; they
are not revocable. They may keep old state alive intentionally, but cannot
discover or mutate a fresh provider unless the author explicitly shared that
same handle. Drop registry entries and node-owned subscriptions at final unmount.

## Runtime cost and release parity

Use a sparse per-node registry (allocate its map only on publication) and an
iterative logical-ancestor walk: expected O(1) insertion and O(h) lookup for
logical depth h. Do not copy ancestor maps or cache lookup results initially.
This avoids invalidation walks on shadowing/replacement and O(nodes × types)
retained caches. Store lookup does no expression evaluation. Existing consumers
can clone handles at mount and perform ordinary property updates afterward.

Generation checks must also cover stale contexts and detached retained nodes;
an ancestor remount must not silently reconnect an old child. Prefer using
existing weak logical-parent links plus explicit generation tracking over a
second strong ownership tree. Measure empty-node overhead and lookup hop counts
before considering provider-only shortcuts.

No syntax, manifest field, serialized store value, or new node kind is proposed.
This should be shared runtime behavior, without a wire-format version change.
The audit must still cover `program_ir`, `binary`, `rust_manifest`, generated
component/handler descriptors, and cartridge templates. On this base the cartridge
template selects a Rust manifest expression or JSON construction and uses shared
runtime descriptors; do not assume a serializer unit test alone exercises the
release launch path. Run equivalent debug/release web fixtures and rich/binary
initialization coverage. Release continues to disable reload.

## Migration and implementation plan after agreement

- Remove runtime store ownership from `RuntimePropertiesStackFrame`; retain its
  expression role. Deprecate the misleading `NodeContext.local_stack_frame`
  store use. Low-level `insert_stack_local_store` callers must migrate to an
  explicit node owner; an expression frame cannot infer one safely.
- Keep ComboBox's store for this task and verify two simultaneous dropdowns.
  Keep Table's contextual composition and verify different sibling dimensions,
  nested tables, and projected Row/Col/Cell/Span consumers.
- Move PathContext registration to its Path node before child mount. Its
  current explicit frame is already isolated; preserve that behavior with
  sibling/nested path tests instead of treating it as another broken caller.
- Audit router-playground and the contextual-components/custom-events fixtures.
  Older test-project imports may need scoped repairs before those projects run.
- This base predates the ExampleHost fix. Bring over only its direct `bind:`
  change/removal of SelectedSourceStore and the focused sibling/binding tests
  from the handoff checkpoint. Do not reintroduce a selection store or copy
  unrelated website work.
- Add a focused web fixture with sibling/nested providers, projected consumers,
  keyed items, and remove/remount controls. Include actual ComboBox, Table, and
  Path checks, plus a direct-binding ExampleHost check.
- Update `components-composition.md` as the canonical explanation; add short
  links/corrections in `state-properties.md`, `primitives.md`, and
  `developer-workflow.md` where warranted. Update public API comments and
  regenerate affected references through the docs tooling. Record pain points.
  No new learning chapter or published docs snapshot is proposed.

## Test matrix

| Area | Required assertions |
| --- | --- |
| State baseline | Shared component template creates independent fields; explicit `bind:` reverse writes reach only the chosen owner |
| Siblings and nesting | Same-type sibling providers cannot overwrite; no upward leakage; nearest nested provider shadows; missing ancestors are skipped |
| Errors and replacement | Missing type, expired context, reentrant borrow; same-owner replacement affects fresh lookups but not cloned handles |
| Projection | Caller expressions/handlers remain caller-owned; received child finds receiver; private slot wrapper does not inject; forwarding and dynamic slots preserve scope |
| Repeats | Several providers in one iteration; keyed reorder/insert/remove; unkeyed positional reuse; nested repeats |
| Lifecycle | Exit rescue, final teardown/drop counters, remount with old context retained; render reparent; no implicit lookup through expired ancestry |
| Reload | In-place stable-handle rebind, changed `bind:` alias, subtree/tree remount, opt-in logic success/failure, no stale provider entries |
| Consumers | Independent ComboBoxes; sibling/nested Tables and helpers; sibling/nested Paths; router store; direct-bound ExampleHosts |
| Parity and cost | Debug and optimized runtime tests; designtime reload tests; rich/binary roundtrips; runnable debug/release web fixture; bounded hops and registry release |

Only the diagnostic reproduction and baseline initialization suite have run for
this proposal. No web, native, release, or hot-reload fix has been validated yet.

## Decisions for review

The following were the initial proposal's decisions. The exploration below adds
an alternative public API direction; neither direction has been approved.

1. Adopt logical nesting for lookup: received children see their receiver's
   provider, while private slot wrappers and render reparenting do not change it?
2. Keep same-owner replacement with explicit snapshot-handle semantics, and
   add `provide_store` / `with_store` with compatibility aliases?
3. Reset providers on real remount/logic reload, allowing a targeted remount
   when template rebinding would otherwise retain stale published aliases?

These are recommendations, not approved semantics. Production implementation
waits for the proposal/review milestone required by PAX-1007.

## Review exploration — typed node access and messages

Zack proposed a blessed node/property traversal API and/or message passing via
NodeContext. Both are viable in Rust and could share one managed addressing
foundation. They address different parts of the store problem: finding a
particular owner, exposing state deliberately, and requesting mutations.

Existing ingredients, verified on this base:

- `NodeContext::get_nodes_by_id` returns all matching mounted nodes globally;
  its implementation scans the node cache, without a scoped or ordered-repeat
  contract (`properties.rs:1271`).
- `NodeInterface` exposes render/template parents, mounted children, and
  `with_properties<T>` for typed access (`engine/node_interface.rs`). It owns a
  strong `Rc<ExpandedNode>` and does not make access conditional on a live mount.
  Its typed access uses mutable RefCell borrowing and can panic on reentrancy.
- `NodeContext::dispatch_event` queues a custom name to a handler bound on the
  emitting component's invocation. It has no payload or arbitrary target.
- `get_node_interface` returns the containing component; `is_descendant_of`'s
  comment says template ancestry while its implementation walks render ancestry.
  A new public navigation API must define these relations explicitly.

### Dynamic structure does not require untyped properties

Use a checked `NodeRef<T>` (or a ref to an explicitly exposed interface).
Selecting by name/key/index is dynamic; accessing T's public interface is typed.
Rust's existing checked downcast pattern is sufficient for recovering the
concrete type. Arbitrary field-name reflection would require Pax descriptors
or generated getters; `Any` alone does not provide it.

Generate optional and collection accessors per component/template site if we
want completion-friendly names. An `if` yields an optional ref, a repeat yields
a collection of refs, and unlike-typed branches can use an explicit enum or a
checked typed query. No enumeration of every concrete render-tree permutation
is necessary. Exact dotted field access is an ergonomic codegen choice, not a
requirement of typed addressing. Runtime absence still needs a fallible API.

Illustrative shape only:

```rust
let list = ctx.refs().child::<List>("list")?;
let row = list.refs().repeat::<Row>("rows").by_key(&item_id)?;
let value = row.read(|row| row.public_value.get())?;
ctx.send(&list, ListMessage::Remove { item_id })?;
```

Define `refs()` as the component's authored instance scope, with separate
received-content traversal. Crossing into a child's private template requires
an explicitly exposed ref. Keep render-tree inspection separately named.
Repeated `.get(6)` means current logical position; `.by_key(key)` means identity
within that repeat, and nested repeats need their full owner/key path.
Queries fail on absence, ambiguity, or type mismatch rather than taking the
first result from the current global lookup. A resolved handle follows its
instance through reorder and expires on unmount/remount. A live query is a
different object that may resolve a replacement; do not conflate the two.

Queries are snapshots unless explicitly reactive. A reactive query must track
structural membership/order changes as well as the selected Property; a tree
lookup inside a computed closure does not by itself declare those dependencies.
Also, parent user mount runs before its private children mount today. Lazy refs
can be created then, but dereferencing them needs a child-ready phase or a
structural subscription. Do not reorder lifecycle callbacks merely to make
an apparently synchronous child field access work.

Prefer weak, generation-checked instance handles. Weak ownership avoids
retaining a removed node; the mount generation detects remount even if the
allocation is reused. Return a borrow-conflict error for direct typed access.
Keep scoped ID/type/key indices incremental if profiling justifies them;
resolved handles should avoid repeated whole-tree scans.

### Typed messages preserve the owner's mutation boundary

An owned Rust enum is sufficient for messages within one runtime; neither
JSON nor cross-thread synchronization is required. The receiver owns its
state and handles the message after the sender's current borrow has ended.
Use direct instance targets or an explicitly published typed sender. Ancestor
endpoint discovery can be convenient, but still needs the provider ancestry
contract; a global message bus would recreate the isolation ambiguity.

Specify FIFO delivery in a bounded event phase, with messages produced during
delivery deferred to the next phase/tick. `send` reports enqueue success, not
completed mutation. Include the sender's identity and event-time payload;
repeat keys and index snapshots must be distinguished. A list reordered before
delivery must not turn a command for item A into a command for the new item at
A's old index. Expired target generations cannot receive or redirect messages.
Pending messages cannot keep removed targets alive. Define queue limits and
overflow behavior rather than allowing an unbounded feedback loop.

### What this changes about stores

| Need | Good primitive |
| --- | --- |
| Inspect or control a particular child | Scoped typed node/interface ref |
| Let an owner validate a requested state change | Typed message to that owner |
| Read shared reactive data throughout a subtree | Published reactive model/handle |
| Find a model or endpoint without coupling to a component's concrete type | Scoped provider lookup |

An exposed interface can combine read-only reactive values and a typed command
sender. A provider then publishes that interface. Descendants get a stable
contract without arbitrary mutation access to the owner's entire Rust struct.
Read-only wrappers must not expose the mutable Property handle through their
public API. Deliberately writable bindings remain useful where that is the
component contract.

This could make stores a convenience for discovering owned interfaces, while
direct child references and typed messages cover many current store uses.
Messages alone do not provide reactive reads (Table geometry is an example),
and explicit node IDs alone do not fix store ownership or disposal. A model
may also outlive any view, so its owner need not always be a visual component.

A bounded design experiment would compare ComboBox selection via typed messages
and Table geometry via a published reactive interface, backed by the same
scoped, generation-checked handles. It would establish whether both mechanisms
belong in PAX-1007 or whether the public communication API merits a linked task.
Generated exposure metadata or message handler descriptors require a fresh
compiler/baked audit; a Rust-only runtime queue need not introduce a serialized
message format. No such implementation or scope expansion is approved yet.
