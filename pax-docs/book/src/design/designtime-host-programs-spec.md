# Designtime Host Programs (Draft)

Authoring Date: 2026-04-27

<!-- summary: Proposed foundation for Pax-authored designtime hosts, live cartridge sessions, and tool workflows. -->
<!-- tags: designtime, ai, images, assets, tooling -->

## Problem

Pax already has most of the low-level pieces needed for Pax-authored designtime tools:

- designtime can inspect, raycast, capture, and replace parts of a running scene
- the design server can already write binary files into a project's `assets/` directory
- `pax-designer` proves that a Pax-authored tool can mount around a running user program

What it does **not** have yet is a neutral foundation for those capabilities.

Today, the path is still designer-shaped:

- the compiler has a special `designer` build path that merges userland and designer manifests
- the runtime has explicit `new_with_designer(...)` entrypoints instead of a more general host model
- the current LLM flow is oriented around whole-component replacement, not asset enrichment
- generated image provenance has no durable designtime home

That makes experimentation harder than it needs to be. The first step should not be "build the final design tool UI." The first step should be "define the contract for Pax programs that are `pax-designer`-shaped."

Those host programs need to be able to:

- own rendering and input around one or more live userland cartridge instances
- provide tool modes such as inspector, asset enrichment, chat, or visual canvas
- mutate userland source deterministically through designtime
- coordinate dense interaction flows through an explicit routing layer

## Goals

- Make a designtime host an ordinary Pax program rather than a compiler mode.
- Define the core contract and vocabulary for host-composed live cartridge surfaces.
- Introduce an explicit FSM-based interaction routing layer for dense editor and agent workflows.
- Keep the first image-enrichment flow compatible with existing `Image` and `NativeImage` usage.
- Allow an AI or agent flow to inspect scene context, generate one or more images, write chosen assets into the project, and apply narrow manifest edits.
- Keep the first proof of concept web-first and debug/designtime-only.
- Avoid widening the runtime or release-baked manifest format unless the runtime truly needs new data.

## Non-goals

- Shipping a full Canva/Figma-style product UI in the first pass.
- Designing the final backend/provider abstraction for image generation.
- Introducing a new runtime image primitive in phase 1.
- Solving every authoring flow up front, especially fully automatic gallery/data-binding generation.
- Supporting multiple concurrently active top-level designtime host programs in one session.

## Current Constraints In This Repo

The proposal should start from what already exists.

### Designtime already exposes useful scene tooling

`pax-designtime/src/messages.rs` and `pax-runtime/src/designtime_support.rs` already define a meaningful designtime surface:

- `Look`
- `InspectTree`
- `RayCast`
- `SelectorQuery`
- `ReplaceNode`
- `Logs`

That is already enough to support "inspect current scene, find target node, capture context, and apply a narrow patch."

### The design server can already write files into project assets

`LoadFileToStaticDirRequest` in `pax-designtime/src/messages.rs`, together with the handling in `pax-compiler/src/design_server/websocket.rs`, already writes arbitrary binary payloads into:

- the user project's `assets/` directory
- the served web `assets/` directory

That is a strong phase-1 foundation. It means generated images can behave like ordinary project assets immediately.

### The compiler/runtime boundary is still special-cased for the designer

The current `designer` flow in `pax-compiler/src/lib.rs`:

- disables the normal static-analysis manifest build path
- assumes the first manifest is userland and the second is designer
- merges manifests into one build artifact
- wraps the userland root in an extra synthetic root component

The runtime side mirrors that specialization in `pax-runtime/src/engine/mod.rs` with `new_with_designer(...)` and explicit userland-root tracking.

This works for `pax-designer`, but it is the wrong long-term abstraction for "a Pax-authored host tool that may or may not render visible chrome."

### The current LLM flow is too coarse for image enrichment

The existing LLM pathway in:

- `pax-designer/src/console/mod.rs`
- `pax-designtime/src/lib.rs`
- `pax-designtime/src/privileged_agent.rs`

is built around a prompt plus screenshot that returns a full `ComponentDefinition` replacement. That is useful for broad UI rewrites, but it is heavier than needed for "swap this placeholder with a generated image" or "try three new background variants."

### The interaction routing layer is still too implicit

The current designer stack has useful pieces:

- `pax-designer/src/model/input.rs` normalizes raw key input into `InputEvent`
- `pax-designer/src/model/action/pointer.rs` chooses a tool on pointer entry
- `pax-designer/src/model/tools.rs` defines per-tool gesture handlers
- tools such as `moving_tool.rs` and `multi_select_tool.rs` own gesture-local state

That is a workable tool architecture, but it is not yet an explicit interaction-routing architecture.

The missing layer is an FSM-oriented router that says:

- what mode the editor is in
- which events are valid in that mode
- which guards decide between similar gestures
- what transition or command each event produces

Without that layer, dense interactions end up spread across:

- input mapping
- modifier queries
- tool activation branches
- per-tool internal control flow
- incidental app-state checks

That is exactly the kind of complexity that tends to grow quickly in vector-design-tool interactions, and the same pressure is likely to appear in an AI design tool once it adds:

- hierarchical navigation
- box transforms
- hover/selection/edit submodes
- agent-assisted generate/review/apply loops

## Proposed Model

### 1. Introduce a generic designtime host

A designtime session should be able to attach one **host Pax program** around the userland cartridge being edited.

The host program is an ordinary Pax program. It controls the full designtime experience:

- layout and rendering around the userland cartridge
- overlays, inspectors, chrome, chat, and tool panels
- input routing and focus
- source-mutating tool workflows

`pax-designer` then becomes one host implementation, not the mechanism itself.

### 2. Use cartridge vocabulary for live userland embedding

The core vocabulary should stay aligned with the runtime/cartridge architecture already used throughout Pax.

- `CartridgeDefinition`: the shared compiled/static definition of the userland cartridge.
- `CartridgeSession`: one live running instance of a `CartridgeDefinition`, with isolated runtime memory, properties, route state, timers, event queues, and viewport state.
- `CartridgeSurface`: a host-owned visual and input surface that presents a `CartridgeSession` inside the host program's layout.

These are deliberately separate concepts.

The host may share one `CartridgeDefinition` across many `CartridgeSession`s, but each session must have distinct runtime state. That is required for springboard-style layouts where the same userland app appears simultaneously in multiple routes or FSM states.

The surface is the Pax node/component concept. A surface answers "where does this live cartridge session appear, and how is input routed into it?"

The session answers "which live execution state is being shown?"

Conceptually:

```text
CartridgeDefinition
  -> CartridgeSession(home route)
      -> CartridgeSurface(tile A)
  -> CartridgeSession(settings route)
      -> CartridgeSurface(tile B)
  -> CartridgeSession(checkout route)
      -> CartridgeSurface(tile C)
```

### 3. Evolve `InlineFrame` toward `CartridgeSurface`

The existing `InlineFrame` is in the right family of primitive. It lets a host program embed userland inside `pax-designer`.

The current implementation is still singular:

- it mounts the implicit userland root instance
- runtime/designtime context tracks one userland root instance and one userland root expanded node
- designtime inspection starts from that singular userland root

The next shape should not discard `InlineFrame`; it should clarify and generalize it.

A `CartridgeSurface` should be the conceptual successor or wrapper:

```pax
<CartridgeSurface session={home_session} />
<CartridgeSurface session={settings_session} />
```

Implementation can keep `InlineFrame` as a lower-level or transitional name, but the design contract should use `CartridgeSurface` because it names the live cartridge embedding behavior rather than a generic layout frame.

### 4. The host owns composition

The host program must totally control rendering around the userland cartridge.

The base case is simple:

- one `CartridgeSession`
- one `CartridgeSurface`
- the surface fills the full viewport

The design-tool case is still one session, but composed differently:

- one `CartridgeSession`
- one `CartridgeSurface`
- the surface is placed inside a zoomable, pannable design canvas

The springboard case is many sessions:

- one shared `CartridgeDefinition`
- many `CartridgeSession`s, each initialized with a route or state seed
- many `CartridgeSurface`s arranged by the host in a scroller, grid, carousel, or other layout

This keeps the host singular while still supporting many simultaneous live userland views.

### 5. Prefer one host, many internal tools

Supporting multiple concurrently active top-level host programs would make core ownership questions ambiguous:

- which host owns input first?
- which host owns overlays and z-order?
- which host owns writeback transactions?
- which host owns undo boundaries?
- how do duplicate host cartridges avoid conflicting state?

Pax already has component composition. The cleaner contract is one designtime host program with internal tool modules/layers:

- asset enrichment
- inspector
- chat
- visual design canvas
- editor opener/deep-link UI

The host can choose whether those modules are visible, hidden, layered, routed, or loaded behind feature flags. The designtime should only need to manage one host authority per session.

### Cross-cutting: add an explicit interaction FSM routing layer

The host should include a first-class interaction router.

This should **not** be one giant flat FSM for the whole application. That would likely explode in size. The better shape is:

- durable editor state in the normal model/app state
- transient interaction state in one or more small FSMs
- a router layer that dispatches normalized events into those FSMs

The recommended split is:

- **editor context**: durable state such as selection, open containers, project mode, viewport transform, active component, and designtime handles
- **gesture FSM**: transient pointer/keyboard gesture state such as idle, marquee-selecting, panning, translating, resizing, rotating, creating, text-editing, or forwarding input into a `CartridgeSurface`
- **agent FSM**: transient AI workflow state such as dormant, collecting context, prompting, generating, reviewing variants, and applying

That separation matters because the FSM should own only interaction sequencing, not the entire document/application state.

### Why an FSM layer fits Pax especially well

Pax already prefers declarative structure plus side-effectful handlers at the edges. An interaction FSM matches that shape well:

- the FSM is the explicit declarative routing layer
- guards consult current reactive editor state
- transition effects emit narrow commands/actions
- actions keep owning manifest mutation, serialization, and side effects

That preserves the good part of the current designer architecture while making the interaction space more legible.

### Proposed event flow

The desired high-level flow is:

1. raw platform input is normalized into typed editor events
2. the interaction router selects the active FSM region
3. the current state plus guards decide the transition
4. the transition emits one or more commands/actions
5. commands mutate app state and/or designtime state transactionally

Conceptually:

```text
RawInput -> InteractionEvent -> InteractionRouter -> FSM Transition -> Action/Command -> AppState/Designtime
```

### Recommended first Rust shape

The first implementation does not need a macro DSL or external state-machine library. Plain Rust enums plus guarded transition functions are enough.

```rust
pub enum InteractionEvent {
    PointerDown { button: MouseButton, point: Point2<Glass> },
    PointerMove { point: Point2<Glass> },
    PointerUp { button: MouseButton, point: Point2<Glass> },
    KeyDown(RawInput),
    KeyUp(RawInput),
    Command(EditorCommand),
    Async(AsyncEvent),
}

pub enum GestureState {
    Idle,
    Panning { start: Point2<Glass>, original: Transform2<Glass, World> },
    MarqueeSelect { start: Point2<Glass> },
    Translating { session: TransformSession },
    Creating { session: CreateSession },
    TextEditing { target: UniqueTemplateNodeIdentifier },
}

pub enum AgentState {
    Dormant,
    CollectingContext { target: UniqueTemplateNodeIdentifier },
    Prompting { target: UniqueTemplateNodeIdentifier },
    Generating { job_id: String, target: UniqueTemplateNodeIdentifier },
    ReviewingVariants { target: UniqueTemplateNodeIdentifier, variants: Vec<GeneratedAsset> },
    Applying { target: UniqueTemplateNodeIdentifier, asset: GeneratedAsset },
}

pub struct InteractionRouter {
    pub gesture: GestureState,
    pub agent: AgentState,
}

impl InteractionRouter {
    pub fn dispatch(
        &mut self,
        event: InteractionEvent,
        ctx: &mut EditorContext,
    ) -> anyhow::Result<()> {
        if self.agent.handle(&event, ctx)?.is_some() {
            return Ok(());
        }
        self.gesture.handle(&event, ctx)?;
        Ok(())
    }
}
```

The important architectural point is not the exact API. It is that:

- state is explicit
- transitions are explicit
- guards are explicit
- side effects happen through narrow actions instead of arbitrary branching spread through tools

### Guarded transitions instead of state explosion

Modifier keys, selection state, and hit-test results should usually be **guards**, not separate top-level states.

For example:

- `Idle + PointerDown + hit selected node -> Translating`
- `Idle + PointerDown + no hit -> MarqueeSelect`
- `Idle + Space + PointerDown -> Panning`
- `ReviewingVariants + AcceptVariant -> Applying`

That gives the routing benefits of an FSM without turning every modifier combination into a separate enum variant.

### Relationship to the current tool system

The FSM layer should not force an immediate rewrite of every existing tool.

A good migration path is:

- keep existing tool implementations as low-level gesture executors
- let the FSM/router decide **when** those executors start and stop
- gradually move hidden tool-local routing logic into explicit transitions

In that model:

- `ToolBehavior` becomes closer to a gesture implementation detail
- the FSM/router becomes the authoritative interaction contract
- `AppState` remains the durable editor state

### Why this matters for image enrichment too

Image enrichment is not only "generate an asset." It is an interaction sequence:

- identify or select target
- collect scene context
- prompt or refine
- wait on async generation
- review candidates
- apply accepted result
- optionally revert or regenerate

That is already a modeful interaction flow. Modeling it as an explicit agent FSM makes it compatible with the same routing architecture that will later support:

- transforms
- hierarchy navigation
- selection semantics
- text editing
- multi-step assistant flows

### 6. Move host attachment toward a runtime concern, not a merged-manifest compiler trick

The current designer build path is useful as proof that Pax can mount a tool around userland. The next step should be to reframe that ability around a host abstraction instead of a `designer` special case.

The desired contract is:

- build the userland project normally for designtime
- build or load exactly one host project for the designtime session
- let the host create and lay out `CartridgeSurface`s
- let designtime create, seed, suspend, reset, and destroy `CartridgeSession`s
- keep each session root explicitly addressable for inspection and mutation

That preserves the useful runtime behavior already present today while removing the assumption that the only valid host is `PaxDesigner`.

If implementation needs a transitional step, it is acceptable to keep some of the current plumbing temporarily. The important thing is that the external model should become "attach a host" rather than "enter designer mode."

### 7. Add a route/session seed contract

Multiple `CartridgeSession`s only become useful if the host can initialize them into meaningful states.

The first seed contract can be intentionally narrow:

- route identifier
- viewport size
- optional serialized app-state seed
- optional scenario label for host UI

The route seed should be treated as a designtime/debug affordance at first. It does not need to be part of release cartridge format unless runtime route initialization becomes a shipping feature.

The host should be able to request:

```rust
let home = cartridge_sessions.spawn(RouteSeed::named("home"));
let settings = cartridge_sessions.spawn(RouteSeed::named("settings"));
```

and then present those sessions through surfaces:

```pax
<CartridgeSurface session={home} />
<CartridgeSurface session={settings} />
```

### 8. Treat image enrichment as an asset workflow, not a new rendering feature

Phase 1 should reuse the existing authoring/runtime surface:

- `Image source=ImageSource::Url("assets/...")`
- `NativeImage url="assets/..."`

Generated images should simply land under a conventional location such as:

```text
assets/generated/
```

and the accepted result should patch the selected node to reference that path.

This keeps the first implementation aligned with:

- existing runtime loading behavior
- existing asset copying/bundling
- existing release builds

It also keeps the first cut out of the binary-baking blast radius, because the runtime still only needs an asset path.

### 9. Keep generated-image provenance in designtime sidecar metadata

Phase 1 should avoid adding new runtime-facing manifest fields for prompt provenance, model metadata, or variant history.

Instead, store that information in a designtime-only sidecar file, for example:

```text
.pax/generated-assets.json
```

or an equivalent file under `assets/generated/`.

That metadata should record at least:

- generated asset path
- target component/template node identity
- prompt or prompt summary
- provider/model metadata
- output dimensions and format
- creation timestamp
- variant/regeneration lineage

This keeps phase 1 lightweight and avoids prematurely changing:

- `PaxManifest`
- `ProgramIR`
- release cartridge generation
- binary roundtrip tests

If provenance later becomes runtime-visible, that can be a separate, deliberate manifest/runtime change.

### 10. Model image enrichment as a narrow transaction

The intended phase-1 interaction should be small and reversible:

1. The user selects an existing image target or a known placeholder target in a specific `CartridgeSession`.
2. The host gathers context:
   - selected node identity
   - bounds and transform
   - local screenshot or `Look` capture
   - optionally nearby structural context from `InspectTree`
3. The host sends a generation request.
4. One or more candidate images come back.
5. The accepted candidate is written into `assets/generated/...`.
6. Designtime applies a narrow manifest edit to the selected node.
7. Sidecar metadata is updated so the result can be regenerated, swapped, or audited later.

The most important part is step 6: the manifest edit should stay narrow. This flow should prefer targeted property updates over full component rewrites whenever possible.

### 11. Prefer existing nodes for the first cut

The narrowest first proof should require one of these to already exist:

- an `Image`
- a `NativeImage`
- an agreed placeholder node that the tool knows how to replace

That avoids mixing two problems into one ticket:

- asset generation
- new layout authoring

Creating brand-new image nodes from scratch is useful, but it should follow once the basic asset loop is reliable.

### 12. Expand the mutation surface only where the host truly needs it

The existing dev/designtime message surface is already enough for inspection and coarse replacement. The next additions should be driven by the host workflow, not by speculative completeness.

The most useful next operations appear to be:

- set/update an image property on a specific selected node
- create a generated asset record and return its written path
- optionally insert a new image node with bounds derived from a selected container

That is a better shape than overloading the current whole-component LLM response path for every image action.

## Recommended First Cut

The first implementation should stay intentionally small:

- web only
- designtime only
- one designtime host program per session
- one `CartridgeDefinition`
- one or more `CartridgeSession`s with isolated runtime state
- `CartridgeSurface` as the conceptual successor/wrapper for `InlineFrame`
- existing selected `Image` or `NativeImage` targets only
- generated assets written as ordinary files under `assets/generated/`
- provenance stored in designtime sidecar metadata
- host modules invisible by default unless a tool mode needs chrome

That first cut is already enough to validate the core question behind this ticket:

"Can Pax support a host-composed designtime environment where tools inspect and mutate live cartridge sessions through deterministic source updates?"

## Phased Plan

### Phase 0: Architecture and naming

- agree that the abstraction is a generic designtime host, not a `pax-designer` special case
- adopt `CartridgeDefinition`, `CartridgeSession`, and `CartridgeSurface` as the working vocabulary
- define the interaction-router boundary and the first FSM regions
- define the smallest host attachment contract
- decide where generated-asset provenance lives

### Phase 1: Generic host attachment on web

- make it possible to attach a host Pax program in designtime without hard-coding `PaxDesigner` semantics
- expose one live `CartridgeSession` through a `CartridgeSurface`
- add a first interaction router with gesture and agent FSMs
- keep userland root inspection/mutation intact
- preserve the "hidden by default" behavior for the host

### Phase 2: Multiple cartridge sessions

- allow one `CartridgeDefinition` to back multiple isolated `CartridgeSession`s
- add the initial route/session seed contract
- support host-composed springboard layouts through multiple `CartridgeSurface`s

### Phase 3: Image enrichment transaction

- add the narrow asset-write and node-update flow
- save generated outputs into `assets/generated/`
- patch selected image targets to point at generated assets
- record sidecar provenance

### Phase 4: Optional visible chrome

- prompt UI
- variant chooser
- regenerate/history controls
- richer feedback around what changed

### Phase 5: Broader authoring flows

- create-new-image-node flows
- tiling/background workflows
- gallery or multi-asset workflows
- additional target platforms

## Open Questions

- Should the host attach through CLI config, designtime config, or explicit runtime wiring first?
- Should `InlineFrame` be renamed, wrapped, or kept as a lower-level primitive beneath `CartridgeSurface`?
- What is the minimum viable `RouteSeed` contract for userland apps?
- How should a host discover route seeds or scenarios exposed by the userland cartridge?
- Is the right phase-1 target strictly existing image nodes, or should a selected container also be a valid insertion target?
- Where should generated-asset provenance live so it is easy to diff, ignore, and clean up?
- How much of the orchestration should remain inside a Pax host UI versus an external agent/chat surface?
- Do we want to generalize the current LLM request/response protocol immediately, or prove the asset loop first with a narrower API?
- What is the cleanest way to keep host input fully pass-through when no visible chrome is active?

## Why This Order

This ordering keeps the risk in the right place.

It proves the host-program contract using infrastructure Pax mostly already has:

- designtime scene inspection
- narrow manifest edits
- live userland embedding through `InlineFrame`
- ordinary asset URLs
- ordinary asset bundling

and delays the more invasive changes until there is evidence they are needed:

- new runtime image semantics
- new release-baked manifest fields
- a fully generalized multi-tool host product shell

That should let PAX-878 define the designtime host contract first, then use asset enrichment as one concrete workflow that exercises the contract.
