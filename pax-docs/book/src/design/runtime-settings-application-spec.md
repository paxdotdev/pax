# Runtime Settings Application (Draft)

Authoring Date: 2026-04-22

<!-- summary: Long-term design for runtime-applied selector settings and ImportSettings. -->
<!-- tags: runtime, settings, selectors, theming -->

## Problem

Pax currently treats `@settings` as a compile-time merge step. During node construction, the runtime asks the manifest for `get_inline_properties(...)`, which has already folded selector blocks into a flat property map for the host node. That works for local styling, but it breaks the longer-term theming/plugin model in PAX-864:

- selector provenance is erased too early
- `id` and `class` do not survive as first-class runtime selector metadata
- designtime owns its own selector parser/matcher instead of sharing one runtime path
- settings reuse cannot be driven by runtime control flow or ordinary component instantiation

The long-term direction is to make settings application a runtime behavior, not a manifest expansion trick.

## Goals

- Keep selector blocks authored in `@settings` intact through manifest generation, cartridge generation, and runtime loading.
- Make selector matching a runtime capability that designtime calls into, not a designtime-only implementation.
- Introduce `ImportSettings` as a first-class primitive for mounting ordinary components as non-rendering settings providers.
- Preserve `id`, `class`, and type selectors as first-class runtime metadata.
- Allow settings providers to use normal component parameters, Rust state, control flow, PAXEL, and lifecycle handlers.
- Keep compile-time checks at the syntax and PAXEL boundary; make runtime matching the sole
  authority for class membership and receiver compatibility.

## Non-goals

- Reintroducing the compile-time `@import` v0.
- Designing a general CSS cascade or complex selector language in the first pass.
- Allowing settings to leak across component boundaries by default.
- Making settings-provider nodes render or receive ordinary hit-tested input.

## Proposed Authoring Model

`@settings` remains the authoring surface for selector blocks.

`ImportSettings` becomes the mechanism for dynamic composition:

```pax
<ImportSettings>
    if active_theme == Theme::Foo {
        <FooTheme tint={accent} />
    } else {
        <BarTheme tint={accent} />
    }

    <CommonTheme />
</ImportSettings>
```

Components instantiated beneath `ImportSettings` are mounted as non-rendering provider instances. Their exported selector blocks participate in settings resolution for the containing host component instance.

This is intentionally runtime-first:

- the provider is an ordinary component instance
- parameters are checked with ordinary component instantiation rules
- expressions inside provider settings evaluate against provider state, not host state
- control flow can add, remove, or swap providers at runtime

## Scope

Settings application remains component-local by default.

The host scope for an `ImportSettings` instance is:

- the expanded nodes whose containing component instance is the same host component instance that owns the `ImportSettings` node
- excluding the non-rendering provider subtree itself

That keeps the behavior aligned with existing `@settings`, which style nodes defined in the local component template rather than reaching through nested child component templates.

## Precedence

There are two axes of precedence:

1. Selector specificity inside a single provider layer
2. Provider order across multiple active providers

Within a single provider layer, precedence is:

- type selector
- class selector
- id selector
- inline node settings

Across providers, later providers win over earlier providers. "Later" should follow the same mental model as element order and z-order in Pax:

- providers contributed by earlier `ImportSettings` instances lose to providers contributed by later ones
- within one `ImportSettings`, earlier provider instances lose to later siblings higher in template order

The host component's own `@settings` block is the base provider layer. Imported providers stack on top of that base layer. Inline settings on the host node remain the highest-priority authoring surface overall.

## Provider Semantics

`ImportSettings` owns an ordinary child subtree. Those children should be expanded, mounted, updated, unmounted, and inspected like any other subtree; the difference is semantic, not structural.

Concretely, the primitive should mark its descendants as non-rendering settings providers. That means:

- they mount, update, and unmount like ordinary components
- they run lifecycle handlers and other non-input logic normally
- they support local state, control flow, nested composition, and parameterization
- they remain visible to structural inspection/debug tooling as ordinary descendants

They should not:

- emit renderables
- participate in hit-testing
- receive standard pointer/keyboard input through their own non-rendering subtree

This deliberately does not canonize the current `sidecar_children` mechanism as the long-term model. A temporary implementation may borrow nearby machinery, but the architectural target is "ordinary children with defined semantics." `Mask` can eventually move in the same direction: two ordinary children with defined semantic order, rather than one render child plus one special sidecar.

## Exported Settings Model

When a component instance appears under `ImportSettings`, it exports the fully resolved settings surface of that component instance to the enclosing host scope. That export surface includes:

- its own `@settings` selector blocks
- its own timeline selector blocks
- any transitive providers introduced by nested `ImportSettings` inside that provider component

In other words, provider export is transitive. A provider component first resolves its own component-local provider stack, then that resolved stack is exported upward as part of the parent host's layering process.

That means the runtime needs a provider-facing representation of authored settings that preserves:

- selector text
- selector kind (`type`, `.class`, `#id`)
- property/event/timeline payloads
- the provider stack frame used to evaluate expressions

A useful conceptual boundary is:

- `AuthoredSettingsBlock`: manifest-time serialized form
- `ResolvedSettingsProvider`: runtime provider instance plus bound stack frame
- `MatchedSettingsLayer`: selector blocks from one provider that match one host node

The important behavior is that provider-authored expressions stay bound to provider scope even when the resulting values are applied to host-node properties.

## Manifest Changes

The manifest should stop pretending selector information is disposable.

### 1. Preserve authored selector blocks

`ComponentDefinition.settings` remains serialized in authored form. No compile-time expansion should erase selector boundaries or imported-provider provenance.

### 2. Normalize node selector metadata

Each `TemplateNodeDefinition` should carry normalized selector metadata in addition to its existing settings list. A concrete shape could be:

```rust
pub struct TemplateNodeSelectorInfo {
    pub source_location: Option<LocationInfo>,
    pub id: Option<Token>,
    pub class_binding: Option<ValueDefinition>,
}
```

`class` is removed from the ordinary inline settings list, so this is the
single persisted representation rather than a second static token index. It
makes selector identity explicit and durable across:

- runtime builds
- designtime builds
- future release descriptor formats that may not want to rescan authored settings text

### 3. Add `ImportSettings` as a real primitive

`ImportSettings` should be represented as an actual template node / primitive type, not lowered away into manifest expansion.

### 4. Normalize selector syntax representation

The manifest/runtime boundary should not hard-code selector semantics to only today's three cases. It should preserve raw authored tokens and also normalize them into an extensible selector AST.

```rust
pub enum SelectorExpr {
    Type(String),
    Class(String),
    Id(String),
    // future: compound selectors, pseudo-like forms, ancestry, etc.
}
```

The first shipped evaluator only needs to implement the current subset. The important design constraint is that selector data structures should be able to grow without another manifest/runtime redesign.

## Runtime Changes

### Shared selector engine

Selector parsing and matching should move out of `designtime_support.rs` into a runtime module that both runtime and designtime call.

Initial selector support should stay intentionally narrow:

- `#id`
- `.class`
- element type names

No new selector language is needed for the first implementation, but the parser/matcher should still be structured around an extensible selector AST rather than a hard-coded id/class/type special case. Selector syntax is likely to grow.

### Runtime selector metadata

Each `ExpandedNode` should expose selector metadata directly, instead of reconstructing class membership from the manifest at query time.

A concrete direction:

```rust
pub struct RuntimeSelectorMetadata {
    pub type_id: TypeId,
    pub id: Property<Option<String>>,
    pub classes: Property<Vec<String>>,
}
```

`id` is already partly runtime today via `CommonProperties`. `class` is not. The long-term design should make both available through one selector-oriented runtime surface.

`classes` is a reactive runtime property populated from a literal or expression
binding. Expressions may evaluate to `String` or `Vec<String>`. This buys two
things:

- selector queries, devtools, and runtime matching all observe one live source of truth
- future selector-syntax extensions do not require another runtime metadata redesign

Class values are normalized as selector identifiers. Empty strings are omitted,
invalid names warn and are ignored, and duplicates collapse so the last
occurrence determines both membership and cascade order. Strings are never
split on whitespace.

Runtime behavior should be:

- node expansion creates `id` / `classes` properties from selector metadata
- selector indices and matched-selector caches subscribe to those properties
- when either property changes, runtime settings are rematerialized for that host node without
  remounting it or rebuilding its subtree

The compiler does not predict class membership or validate selector properties
against potential receivers. Dynamic membership, imported providers, and hot
reload make such checking partial by construction. Incompatible matched
properties are skipped with deduplicated runtime diagnostics.

### Component-local settings scope

Each expanded component instance should own a `SettingsScope` that contains:

- the base local provider for that component's own `@settings`
- the active imported providers contributed by `ImportSettings` descendants
- a monotonically increasing version or invalidation token

The runtime should resolve effective node settings from:

- host node selector metadata
- active provider ordering
- matched selector blocks
- inline settings

This resolution should happen in runtime, not in manifest construction.

### Provider registration

When an `ImportSettings` node mounts:

- it expands its children into an ordinary provider subtree marked non-rendering
- each provider instance registers itself with the enclosing host component's `SettingsScope`
- registration order determines layer precedence

When it updates or unmounts:

- removed providers unregister
- newly mounted providers register
- the host scope invalidates effective settings for affected nodes

### Applying properties, timelines, and transitions

The runtime currently builds node properties from a single flattened `BTreeMap<String, ValueDefinition>`. That abstraction is too lossy for provider-backed settings, because values may come from different stack frames.

The runtime settings path should instead work with something like:

```rust
pub struct ResolvedSettingValue {
    pub source_scope: Rc<RuntimePropertiesStackFrame>,
    pub source_node: ExpandedNodeIdentifier,
    pub selector: Token,
    pub value: ValueDefinition,
}
```

After selector matching and precedence resolution, the host node receives a merged map of resolved values. Property factories then compile those values against their source scope instead of assuming "all settings came from the host node."

The same model should apply to:

- common properties
- transitions
- timeline selector tracks

Ordinary settings continue to follow live classes during a lifecycle
transition. Selector membership for the transition's own timeline tracks is
captured when that transition run starts, so changing a class mid-flight does
not splice tracks into or out of the active run.

### Cross-scope reactive data flow

Imported settings should be exported by reactive reference through the Property DAG, not by snapshot copy.

If a provider-authored setting expression reads provider-owned state, receives a parameter from the host, or participates in `bind:`-style data flow, the resulting host-node property should retain those dependencies as ordinary DAG edges. The runtime should not "copy values out of the child" except for fully static literals after constant folding.

When the host property factory materializes a `ResolvedSettingValue`, it should compile the value against `source_scope`. That means:

- provider expressions continue to evaluate against provider state
- changes in provider state propagate reactively into host-node properties
- explicit `bind:` edges remain shared/bidirectional property relationships rather than cloned state

This does intentionally expose provider state to the host through the imported settings surface. That is acceptable and should be treated as part of the primitive's power, not as an accidental leak.

Cycles can occur when host state flows into a provider and exported settings flow back onto the same host subtree. The settings system should not invent a separate cycle model. It should rely on the Property DAG as the source of truth for dependency tracking and cycle detection. When a cycle is detected, the runtime should emit a diagnostic that names:

- the `ImportSettings` site
- the provider component instance
- the selector being applied
- the host property participating in the cycle

### Event and handler behavior

Provider components are full component instances, so provider-owned lifecycle handlers such as `@mount`, `@tick`, `@pre_render`, and `@unmount` should continue to work naturally.

Hidden provider nodes should not receive standard input interrupts directly because they do not render and do not participate in hit-testing.

Selector-exported input handlers should be part of the model, not deferred. The important distinction is:

- scalar properties, transitions, and timeline tracks use precedence and override rules
- event handlers compose rather than override

For a host-node event dispatch, handler order should be:

- host component base-layer selector handlers
- imported-provider selector handlers in provider layer order
- inline handlers authored directly on the host node

Within a layer, source order is preserved.

Each exported handler executes against the provider component instance that declared it:

- the handler function and `self` state come from the provider's handler registry and properties
- the event origin/target in the node context is the matched host node, not the non-rendering provider node

That gives themes/plugins a way to react to host-node input without pretending the non-rendering provider subtree was hit-tested.

## Designtime Changes

Designtime selector queries should stop reimplementing selector logic.

Specifically:

- selector parsing should call the shared runtime selector parser
- selector matching should read `RuntimeSelectorMetadata` from expanded nodes
- class lookup should stop rescanning template settings through the manifest

The manifest ORM can continue editing authored selector blocks. After edits, the runtime selector metadata for affected nodes is regenerated as part of the ordinary reload/rebuild path.

The `ImportSettings` subtree should be exposed to `pax-cli dev` and other structural inspection surfaces the same way other subtrees are exposed, with explicit metadata marking it as non-rendering / settings-only.

That means:

- structural tree inspection includes provider descendants
- selector queries can see provider descendants when traversing the full tree
- visual pick/raycast flows still skip them because they produce no render output

## Release / Cartridge / Descriptor Concerns

This feature cannot be designtime-only. Release builds need the same selector and provider data.

Release cartridges use baked program representations. Authored selector blocks
and the complete class binding must therefore survive manifest, generated Rust,
program IR, and binary baking.

If Pax later introduces a slimmer descriptor-oriented release format, that format must still preserve:

- node selector metadata (`type`, `id`, `classes`)
- component-authored selector blocks and timeline selector blocks
- `ImportSettings` nodes and their host/provider relationships

In other words, selector logic has to move into runtime, and selector data has to survive release compilation.

## Diagnostics Boundary

The compiler validates selector syntax and ordinary PAXEL syntax, but it does
not construct a static class universe, report unused classes, or attempt
selector/receiver compatibility analysis.

The runtime is the sole matching authority. It warns and skips safely when a
class binding produces an unsupported value, a class name is invalid, or a
matched declaration does not exist on the receiving element. An unmatched
class is valid and does not warn because providers may arrive through imports
or hot reload.

## Phased Rollout

### Phase 1: Lift selector logic into runtime

- move selector parse/match code out of designtime-only code
- add normalized selector metadata to template nodes and expanded nodes
- make designtime consume the shared runtime selector engine

### Phase 2: Apply local component settings at runtime

- stop flattening local `@settings` into `get_inline_properties(...)`
- resolve the host component's own selector blocks at runtime without `ImportSettings` yet

This is the critical architecture shift. It proves runtime settings application before dynamic imports are added.

### Phase 3: Add `ImportSettings`

- introduce the non-rendering primitive
- mount provider subtrees
- register providers into host `SettingsScope`
- apply provider layering and invalidation

### Phase 4: Lift timelines and transitions fully

- route selector-targeted timelines and transitions through the same runtime resolver
- ensure release cartridge data and designtime inspection use the same representation

### Phase 5: Optimize

- selector indexes by `id`, `class`, and `type`
- provider-layer invalidation that only recomputes affected nodes
- cache matched selector results where correctness allows

Correctness comes first; indexing and cache shape should follow stable semantics.

## Open Questions

- What is the exact user-facing format for runtime cycle diagnostics when `bind:` and imported settings create feedback loops?

## Summary

The long-term design is not "compile-time import, but more clever." It is a runtime settings system with:

- first-class selector metadata
- a shared runtime selector engine
- component-local settings scopes
- non-rendering provider components mounted through `ImportSettings`

That keeps Pax aligned with the ticket's theming/plugin direction while preserving the ability to parse, inspect, serialize, and ship the same behavior consistently across designtime and release builds.
