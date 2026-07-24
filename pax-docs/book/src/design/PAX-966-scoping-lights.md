# PAX-966 LightFrame Scoping

Authoring Date: 2026-07-21

Status: implemented MVP; example and cross-platform interaction validated

<!-- summary: Lexically scoped LightSource membership through one-way LightFrame containment. -->
<!-- tags: lighting, lightframe, scope, gpu, drawing, components, scroller -->

## Summary

Add a non-rendering `LightFrame` container. A `LightSource` belongs to its
nearest containing `LightFrame`, or to an implicit root frame when there is no
explicit container. The light may illuminate vector primitives inside that
frame and inside nested frames, but it may not escape to a parent or sibling
frame.

The rule is intentionally one-way:

- outer lights may enter nested frames
- inner lights may not escape their nearest frame
- sibling frames do not see one another's local lights
- `Material::unlit()` continues to override all lighting

This gives components a local lighting environment without names, string
coercion, selector matching, per-background opt-outs, or a new GPU selector
engine. Each expanded `LightFrame` instance is a distinct runtime scope, so a
repeated button can author its own hover light without illuminating neighboring
buttons.

The MVP scopes `LightSource` only. `AmbientLight` keeps its current effective
layer-wide behavior. Shared singleton resources and arbitrary disjoint target
sets remain separate follow-up designs.

## Implementation Status

The MVP is implemented across the authoring, runtime, retained-rendering, GPU,
compiler, binary-baking, and example paths:

- `LightFrame` is exported as a non-rendering structural primitive.
- The runtime resolves each light's nearest expanded render-parent
  `LightFrame`, selects a deterministic per-layer palette of at most eight
  lights, and computes a direct-light bitmask for every retained canvas node.
- Retained nodes are dirtied when light enablement, slot allocation, or
  effective render ancestry changes their masks.
- The GPU primitive and WGSL shader use the mask before direct-light math.
  Unlit materials still bypass lighting, while authored ambient remains
  distinguishable from the default ambient fallback.
- Overflow beyond eight active lights is deterministic and emits one diagnostic
  per changed overflow count.
- `LightFrame` is represented by the baked node type rather than serialized
  string scope metadata, and release-cartridge construction includes the new
  primitive.
- Mouse events expose node-local pointer coordinates. Touch streams retain their
  initial target through movement and release, including when an ancestor
  Scroller ultimately owns the pan; platform cancellation remains distinct.
- `examples/src/glow-buttons` demonstrates twelve independently scoped lights,
  desktop hover, touch-following feedback, scroll coexistence, and a smooth
  return from the active shaded state to the resting appearance.

Debug-only `pax-cli dev inspect tree` lighting metadata remains a useful
follow-up, but is not required for the authoring or rendering behavior.

## Relationship To PAX-960

PAX-960 introduced light-reactive vector materials, `LightSource`, and
`AmbientLight`. Its first implementation deliberately collected lighting per
logical canvas layer. That made the initial rendering path simple, but it also
made every default-lit vector primitive in a reachable layer react whenever a
light was present.

PAX-966 adds membership to that model. It does not change material coefficients,
light math, light coordinate conventions, or the set of primitives that can be
lit. The existing layer reachability rules remain an outer constraint;
`LightFrame` narrows which primitives within those reachable layers see a
particular light.

## Problem

Layer-wide lights make local visual effects difficult to encapsulate. A hover
light authored by one button can illuminate adjacent buttons, panel
backgrounds, controls, outlines, and other vector chrome. Avoiding that effect
currently requires marking every unrelated primitive `Material::unlit()`.

That opt-out model has several problems:

- the component that owns the effect cannot fully encapsulate it
- unrelated surrounding UI must know that the light exists
- omissions change the appearance of distant elements
- examples become dominated by defensive `unlit` settings
- repeated components interfere with one another
- adding a light can dim non-target siblings through the default ambient term,
  even when their desired direct-light contribution is zero

The core problem is light membership, not attenuation or material response.

## Goals

- Let a component contain a light whose effect cannot escape that component's
  chosen subtree.
- Preserve root or scene lights as a convenient way to illuminate an entire
  scene, including nested frames.
- Make nested behavior deterministic and explainable with ordinary tree
  ancestry.
- Give every expanded component or repeat instance independent scope identity.
- Preserve existing rendering when no explicit `LightFrame` is authored.
- Preserve `Material::unlit()` as the local, final opt-out.
- Avoid string-backed scope names and magic conversion from class names.
- Avoid using new canvas layers or offscreen composition as a scoping shortcut.
- Fit the first implementation into the existing bounded light array and
  retained vector renderer.
- Keep the design compatible with future selector targeting, named resources,
  and per-frame ambient environments without requiring them now.

## Non-goals

- Component-authored singleton lights shared across multiple component
  instances.
- A light targeting arbitrary, disjoint nodes such as every `.gem_surface`
  across unrelated subtrees.
- General CSS selectors, dynamic PAXEL selectors, or selector evaluation in the
  shader.
- Two-way lighting isolation where outer lights are prevented from entering a
  frame.
- Per-frame ambient-light overrides.
- Removing or substantially increasing the renderer's current maximum of eight
  simultaneously enabled lights per target layer.
- Lighting images, text, or native controls.
- Spatial clipping of a light to the geometric bounds of a `LightFrame`.
- Shadows, occlusion, additional light shapes, or changes to the shading model.

## Authoring Surface

`LightFrame` is a normal structural container with no MVP-specific properties:

```rust
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::lighting::LightFrameInstance")]
pub struct LightFrame {}
```

This follows the existing empty-primitive pattern used by nodes such as `Mask`
and `InlineFrame`. No author-facing configuration is required for the first
slice.

`LightFrame` should use ordinary container and expanded-node behavior:

- common node properties and transforms work normally
- children use the frame as their render parent
- the frame does not render pixels
- the frame does not clip
- the frame does not create a canvas layer or offscreen surface
- the frame is invisible to raycasting
- the frame participates in child/layout and slot projection as a real
  container, like `Group`, rather than behaving like a transparent control-flow
  fragment
- child z-order is unchanged

The word "frame" describes lexical containment, not a geometric lighting
volume. A descendant that draws outside the frame's bounds remains a member of
the frame. Authors may use an ordinary `Frame` or `Mask` separately when they
also need clipping.

### Basic Local Light

```pax
<LightFrame width=100% height=100%>
    <LightSource
        x={self.pointer_x}
        y={self.pointer_y}
        z=180px
        radius=220px
        intensity=0.9
        enabled={self.hovered}
    />

    <Rectangle
        width=100%
        height=100%
        fill=rgb(56, 86, 132)
        material=Material::glossy(0.42)
    />
</LightFrame>
```

The light affects the rectangle and any other light-reactive vector descendant
of this frame. It does not affect the frame's siblings.

### Root Light Entering A Local Frame

```pax
<LightSource
    shape=LightShape::Directional
    direction={Vector3::new(-0.3, -0.4, -1.0)}
    intensity=0.35
/>

<LightFrame>
    <LightSource enabled={self.hovered} intensity=0.8 />
    <Rectangle material=Material::glossy(0.4) />
</LightFrame>
```

The rectangle receives both the root directional light and the local hover
light. A sibling outside the explicit frame receives only the root directional
light.

### Repeated Component Isolation

In `GlowButton`'s template:

```pax
<LightFrame width=100% height=100%>
    <LightSource
        x={self.pointer_x}
        y={self.pointer_y}
        enabled={self.hovered}
        radius=160px
    />
    <Rectangle
        width=100%
        height=100%
        material=Material::glossy(0.36)
    />
</LightFrame>
```

Ten `GlowButton` instances produce ten distinct frame identities. Enabling the
light in one instance does not add that light to the membership of any other
instance.

## Normative Membership Model

### Terms

- **Implicit root frame:** a conceptual frame enclosing the mounted render
  tree. It does not correspond to an authored node.
- **Owning frame of a light:** the nearest explicit `LightFrame` in the light's
  expanded render-parent ancestry, or the implicit root frame.
- **Effective frame of a primitive:** the nearest explicit `LightFrame` in the
  primitive's expanded render-parent ancestry, or the implicit root frame.
- **Frame ancestry:** a frame, its containing frames, and finally the implicit
  root frame.
- **Layer-reachable light:** a light that the existing logical-canvas-layer
  collector can transform into the primitive's target layer.

### Eligibility Rule

An enabled `LightSource` is eligible to illuminate a vector primitive if and
only if both conditions are true:

1. the light is layer-reachable from the primitive's target canvas layer; and
2. the light's owning frame is in the effective frame's ancestry, including
   equality.

In compact form:

```text
eligible(light, primitive) =
    layer_reachable(light, primitive)
    && ancestor_or_self(owner_frame(light), effective_frame(primitive))
```

This relation is based on expanded runtime instances, not template definitions,
class names, IDs, or source strings.

### Nested Frames

Given this tree:

```text
implicit root
├── root light
├── root surface
└── frame A
    ├── light A
    ├── surface A
    ├── frame B
    │   ├── light B
    │   └── surface B
    └── frame C
        └── surface C
```

The effective membership is:

| Primitive | Eligible lights |
| --- | --- |
| root surface | root light |
| surface A | root light, light A |
| surface B | root light, light A, light B |
| surface C | root light, light A |

Light B cannot escape frame B. Light A can enter B and C because A is an
ancestor of both. A `LightFrame` with no local lights has no visual effect; it
does not block inherited lights.

### Components, Repeats, And Conditionals

Scope identity belongs to an `ExpandedNode` instance:

- every component instance receives a distinct instance of each `LightFrame`
  in its template
- every `for` expansion receives a distinct frame instance
- conditionally mounted frames exist only while their expanded subtree is
  mounted
- exiting nodes retain their existing membership through their exit lifecycle
  and lose it when unmounted
- moving or reprojection under a different render parent recomputes membership

A component boundary does not implicitly create a lighting frame. Components
that need isolation must author `LightFrame` explicitly.

### Slots And Projection

Membership follows expanded **render-parent** ancestry. This is important when
slot projection makes template ancestry and visual ancestry differ. A child
projected beneath a `LightFrame` is a member of that frame even if the child's
source template was authored elsewhere.

This matches the visible composition authors reason about and gives components
control over whether their slot content participates in local lighting.

### Order

For scenes within the supported light count, light membership and contribution
are independent of template order and z-order. A light may appear before or
after the primitive it illuminates.

Order matters only for existing singleton ambient selection and for the
deterministic overflow rule described below.

### Materials

`LightFrame` does not alter material selection:

- `Material::unlit()` always returns the authored fill/stroke color without
  ambient, diffuse, or specular lighting
- lit materials use only their eligible lights
- emissive behavior remains unchanged
- a primitive with no eligible direct lights and no effective authored ambient
  uses identity lighting and renders exactly as it would in an app with no
  lighting resources

The final rule prevents a local light in one frame from dimming unrelated
siblings through Pax's default ambient term.

## AmbientLight Semantics

`AmbientLight` remains outside `LightFrame` membership for the MVP.

For each target logical canvas layer:

- the existing topmost enabled `AmbientLight` selection remains in force
- the selected ambient applies to all light-reactive vector primitives in that
  layer, including primitives whose eligible direct-light mask is empty
- `Material::unlit()` still ignores it
- an `AmbientLight` nested inside a `LightFrame` is not local to that frame

Default ambient and authored ambient must be distinguished at runtime:

| Eligible direct lights | Effective authored ambient | Result |
| --- | --- | --- |
| none | none | identity/legacy rendering |
| one or more | none | Pax default ambient plus eligible lights |
| none | present | authored ambient only |
| one or more | present | authored ambient plus eligible lights |

The renderer therefore needs an `ambient_is_authored` bit or equivalent
internal state. `SceneLighting::active` alone cannot express the first row when
some other frame in the same layer owns an active light.

Per-frame ambient environments can be added later, but they require either
per-primitive ambient data or multiple GPU lighting palettes and should not be
smuggled into the direct-light MVP.

## Logical Canvas Layers And Scrollers

`LightFrame` is an additional membership filter; it does not redefine which
canvas layers can exchange light data.

Keep the existing layer-reachability direction:

- lights in the same logical canvas layer are candidates
- lights from an ancestor/owner layer may enter a descendant scroller-owned
  canvas layer after coordinate transformation
- lights inside a descendant scroller layer do not travel upward to its owner
  layer or sideways into sibling layers

This gives a reliable pattern for lighting scrolled content:

```pax
<LightFrame width=100% height=100%>
    <LightSource x=80% y=10% z=260px />
    <Scroller>
        <GemGrid />
    </Scroller>
</LightFrame>
```

The light lives in the owner layer and may enter the scroller's canvas layer.
Its `LightFrame` still prevents it from affecting siblings outside the frame.

The existing point-light coordinate conversion remains authoritative. Frame
transforms affect descendant light positions through the ordinary expanded-node
transform chain; no new local light coordinate space is introduced.

## Clipping, Bounds, And Native Content

- `LightFrame` bounds do not clip light influence.
- A surrounding ordinary `Frame`, `Mask`, or `Scroller` continues to control
  visual clipping.
- Light membership does not depend on primitive bounds intersecting frame
  bounds.
- `unclippable` rendering does not escape lexical membership; membership is
  derived from the retained render-parent chain.
- Native controls, native text, and other native elements remain unaffected.
- `Image` remains unaffected in the vector-material MVP.

## Compatibility

Existing apps contain no explicit `LightFrame`, so every light and vector
primitive resolves to the implicit root frame. Their membership is therefore
identical to the current layer-wide model.

The only renderer behavior that must become more precise is per-primitive
identity lighting:

- if a primitive has no eligible direct light and there is no authored ambient,
  return its original color
- do not apply the default `0.35` ambient merely because an inaccessible light
  is active elsewhere in the layer

Non-WGPU renderers already degrade materials to ordinary fill/stroke rendering.
`LightFrame` is a no-op on those renderers and must not prevent content from
rendering.

## Runtime Design

### Scope Identification

Add an internal capability to `InstanceNode`, conceptually:

```rust
fn establishes_light_frame(&self) -> bool {
    false
}
```

`LightFrameInstance` overrides it with `true`. Runtime code finds the nearest
frame by walking `ExpandedNode::render_parent_node()`. The implicit root may use
a sentinel ID; explicit frames may use `ExpandedNodeIdentifier` directly.

Scope identity is runtime-only. No scope name or selector needs to cross the
template/compiler boundary.

### Layer Lighting Resolution

For each target logical canvas layer, resolution should produce:

- the selected effective ambient and whether it was explicitly authored
- up to eight enabled, layer-reachable light records
- the owning frame ID for each selected light slot
- a per-canvas-node light mask

Conceptual internal structures:

```rust
struct ResolvedLayerLight {
    slot: u8,
    owner_frame: LightingFrameId,
    light: SceneLight,
}

struct ResolvedLayerLighting {
    ambient: SceneAmbientLight,
    ambient_is_authored: bool,
    active_light_mask: u32,
    lights: Vec<ResolvedLayerLight>,
}
```

For a vector canvas node, bit `n` is set when light slot `n` passes the
eligibility rule. Because the GPU currently supports eight lights, a `u32`
leaves ample room without making scope names part of GPU data.

The per-node mask should be looked up by `begin_bounded_canvas_node` and stored
with the retained vector node/draw operations. This keeps all fill and stroke
operations from one vector primitive in the same lighting environment and
avoids adding a membership parameter to every material-aware drawing method.
Images may carry or ignore the node value; their shader remains unlit.

### Slot Stability And Invalidation

While an enabled light remains selected for a layer, keep its GPU slot stable.
Changing color, position, direction, intensity, radius, or other light
parameters should update the layer lighting buffer without requiring vector
retessellation or changing node masks.

Enabling, disabling, mounting, unmounting, or reparenting a light can change slot
allocation. Reparenting a `LightFrame` or a vector node can change ancestry.
When either happens, all affected retained canvas nodes must be re-recorded with
new masks before a freed slot may be reused for a different light. A broad
per-layer canvas-node invalidation is acceptable for the first implementation;
targeted subtree invalidation is an optimization.

This requirement is important for retained rendering: replaying a primitive
with an old mask against a newly assigned slot can illuminate the wrong frame.

### Maximum Active Lights

The existing WGPU limit remains eight selected, simultaneously enabled lights
per target logical canvas layer. Disabled hover lights should not consume a GPU
slot, which lets a scene contain many repeated components when only a small
number are active simultaneously.

If more than eight enabled candidates are layer-reachable:

- selection must be deterministic
- retain the eight with highest render-tree precedence, matching Pax's existing
  topmost-resource convention
- use expanded node ID as a deterministic tie-breaker
- emit one rate-limited warning per affected layer, including the number of
  omitted lights
- omitted lights contribute to no primitive and set no mask bits

Order therefore affects only overflow selection, not ordinary contribution.
Supporting many independently lit frames simultaneously likely requires a
future palette/storage-buffer design rather than silently stretching this MVP.

## GPU Design

Add a light-membership mask to each vector primitive's GPU data:

```rust
pub(crate) struct GpuPrimitive {
    // existing fields
    pub light_mask: u32,
    // explicit padding as required by the matching WGSL layout
}
```

The Rust and WGSL structures must be changed together and their size/alignment
must be asserted in tests. Do not pack scope data into unrelated material or
fill bits.

`GpuSceneLighting` should expose at least:

- which light slots contain active selected lights
- whether the ambient value is authored or is only the default fallback

Conceptual shader logic:

```text
if material is unlit:
    return source color

eligible = primitive.light_mask & scene.active_light_mask

if eligible is empty and ambient is not authored:
    return source color

lighting = effective ambient
for each active light slot:
    if eligible contains slot:
        accumulate diffuse and specular contribution

return source color * lighting + emissive
```

When `eligible` is non-empty and no ambient was authored, use Pax's existing
default ambient. When ambient was authored, use it even if `eligible` is empty.

The fragment loop remains bounded by the renderer maximum. The mask check
belongs before point/directional light math so inaccessible lights have minimal
per-fragment cost.

## Compiler And Binary Baking

`LightFrame` is a new public template-visible primitive and must work in both
rich debug manifests and release cartridges.

Audit and test:

- `pax-std` exports and engine prelude registration
- compiler static-analysis recognition of the primitive
- generated Rust manifest construction
- program IR and binary manifest roundtrips
- release cartridge generation and mounting
- hot reload when a `LightFrame` is inserted, removed, or moved

The MVP should derive scope from the baked node type and expanded render tree.
It should not add a serialized string scope field to vector primitives or
`LightSource`.

If `SceneLighting` gains an `ambient_is_authored` field, audit its PaxValue,
serde, and any binary representation even if the field appears runtime-facing.

## Diagnostics And Developer Tools

At minimum, provide the active-light overflow warning described above.

For `pax-cli dev inspect tree`, it would be useful to expose debug-only fields
when practical:

- whether a node establishes a `LightFrame`
- the effective frame expanded-node ID
- the resolved direct-light mask for canvas primitives
- the owning frame ID and selected slot for each light

These fields are not required authoring API, but they make membership bugs
distinguishable from material, transform, layer, and shader bugs.

An empty `LightFrame` is valid and should not warn. A local light with no lit
vector descendants may be reported by tooling later, but it is not a compile
error.

## Validation And Coverage

### Automated Runtime Regression Coverage

- mounted expanded trees resolve scope from render-parent ancestry
- root lights enter nested frames
- local lights do not reach parent or sibling surfaces
- nested frames accumulate root and ancestor-frame lights
- enabling or disabling a light reallocates slots and dirties only retained
  nodes whose masks changed
- changing effective render ancestry invalidates a stale retained mask
- a zero direct-light mask preserves identity lighting under default ambient
- authored ambient remains active for a zero direct-light mask
- selection at the eight-light limit is deterministic, reports overflow, and
  clears the overflow state when the scene returns within the limit
- captured touch targets survive movement, release exactly once, and are
  cleared if their expanded node is removed

The existing `pax-runtime`, `pax-gpu`, `pax-compiler`, and `pax-manifest` suites
exercise the surrounding renderer, shader, manifest, and compiler paths. A full
workspace check guards non-WGPU and cross-crate API compatibility.

### Compiler, Cartridge, And Chassis Validation

- the GlowButton example compiles with twelve repeated `LightFrame` component
  instances
- a release-baked macOS cartridge mounts and demonstrates local lighting
  isolation
- the web chassis interface builds successfully
- the shared Swift chassis tests pass, including touch cancellation dispatch
- web desktop pointer interaction and Chrome touch emulation were exercised
- native iOS Scroller interaction was exercised in the simulator

### Visual Acceptance

`examples/src/glow-buttons` is the focused integration regression:

- one hovered or touched instance does not illuminate or dim its neighbors
- pointer and touch coordinates position the light correctly on both axes
- a touch that becomes a scroll keeps presentation feedback while suppressing
  semantic activation
- a horizontal start may transition into vertical scrolling once movement
  makes the intent clear
- quick taps show a complete glow-in/glow-out pulse
- mouse exit and touch release ease continuously back to the resting appearance
- the Scroller exposes the complete grid and footer at compact iOS dimensions

Dedicated inspect-tree lighting metadata and screenshot-based golden tests are
deferred. The implemented example and regression suite remain the acceptance
surface for the MVP.

## Implementation Sequence

1. Add `LightFrame` as a non-rendering container and expose the runtime
   `establishes_light_frame` capability.
2. Add frame-ancestry resolution and tests independent of rendering.
3. Extend per-layer lighting resolution with selected light slots, owning frame
   IDs, explicit-ambient state, and per-node masks.
4. Thread the mask through retained vector-node recording without changing
   material authoring APIs.
5. Add the GPU primitive field, matching WGSL layout, and membership checks.
6. Implement slot/membership invalidation for light lifecycle and render-tree
   changes.
7. Add overflow diagnostics; defer optional inspect-tree metadata.
8. Add compiler, binary-baking, fallback, and visual regression coverage.
9. Update author-facing light/material documentation after behavior is proven
   on the WGPU chassis.

## Deferred Extensions

### Selector Targeting

An explicit form such as `affects=.gem_surface` may later support disjoint
targets that cannot share a useful ancestor. If added, it should reuse Pax's
typed class/ID token model, remain explicit on the light, and avoid coercing
ordinary strings into selector identities. It should compose with, not replace,
the lexical safety boundary: a selector should not let a local light escape its
owning `LightFrame` unless a separate design explicitly allows that.

### Shared Singleton Lights

A component-authored light shared across component instances needs a resource
lifetime and coordinate-anchor model. Deduplicating identical light nodes is
not sufficient because instances may have different transforms, scrollers, and
owners. Treat this as scene-resource hoisting rather than part of
`LightFrame`.

### Per-frame Ambient And Hard Isolation

Future APIs might support local ambient overrides or a hard boundary that also
blocks outer lights. Both require multiple effective lighting environments and
clear inheritance semantics. The one-way direct-light MVP leaves room for that
without committing to properties such as `inherit=false` prematurely.

### Larger Lighting Palettes

Scenes with more than eight simultaneously enabled scoped lights may need a
storage-buffer palette, clustered/tiled selection, or multiple compact lighting
environments. The per-primitive membership concept can survive that evolution,
but the first implementation should prove authoring semantics before widening
the renderer architecture.

## GlowButton Demonstration Component

The MVP includes `examples/src/glow-buttons`, a focused visual proof that each
component instance owns a local light without defensive material settings on
its siblings. It presents twelve differently colored `GlowButton` instances in
a responsive three-column desktop or two-column compact grid.

`GlowButton` is an example-local reusable component with three authored inputs:

- `label: String`
- `base_color: Color`
- `glow_color: Color`

Its template wraps the complete button surface and its `LightSource` in one
`LightFrame`. The primary surface uses a glossy material; decorative highlights,
labels, and shadow accents use `Material::unlit()` where illumination would be
artistically incorrect. The light is disabled while idle and is enabled only
for an active hover or touch interaction. Moving over one instance must neither
illuminate nor dim any other instance.

### Desktop Interaction

- `MouseOver` enables the light, eases its intensity in over 135 ms, and gives
  the shell a subtle 1.018 scale lift.
- `MouseMove` converts window coordinates with `NodeContext::local_point`, then
  eases the light toward the pointer over 52 ms.
- `MouseDown` compresses the shell to 0.982 scale over 70 ms.
- `MouseUp` springs back toward the hovered scale over 130 ms.
- `MouseOut` eases the light to zero over 230 ms and restores unit scale. A
  pre-render check disables the light once the fade is effectively complete.
- The ordinary `Click` event remains the semantic activation path. The
  low-level gesture handlers own presentation only and must not synthesize a
  second press.

### Touch Interaction

Touch intentionally reveals the same effect without pretending that hover
exists on a touch screen:

- `TouchStart` positions the light at the contact point, enables it, eases to a
  slightly stronger 2.65 intensity over 145 ms, and compresses the shell.
- `TouchMove` follows the accepted touch identifier with the same short
  position easing used by the mouse. The runtime captures the initial target,
  so the effect continues cleanly when the finger leaves the button bounds.
- `TouchEnd` updates the final position, restores the shell, and queues a 260 ms
  fade after the glow-in. A casual tap therefore shows a complete, readable
  glow-in/glow-out pulse instead of cancelling the entrance animation.
- An enclosing native Scroller observes the same contact simultaneously. When
  its pan wins, the button continues receiving `TouchMove` and ultimately
  `TouchEnd`, so the glow remains visible and follows the finger while content
  scrolls. Scroll ownership disqualifies semantic activation but does not cancel
  the presentation-level touch stream.
- `TouchCancel` is reserved for a contact the platform actually aborts before
  release, such as a system interruption, and clears the glow promptly.
- A held touch, sub-threshold motion, and motion during scrolling all make the
  light-following behavior discoverable. Letting a component prevent or replace
  scrolling still requires a future explicit gesture-capture API; the MVP does
  not hide that policy in a timing heuristic.
- Normal tap-to-click synthesis performs semantic activation after touch end;
  it is suppressed when scrolling wins, and the visual handlers do not fire the
  application action directly.

The component tracks one accepted touch identifier. Additional touch points do
not retarget that active interaction; fully independent multi-pointer component
capture remains outside this focused example.

### Showcase Acceptance Criteria

- All twelve palette variants remain legible at rest and under illumination.
- Hovering or touching one button changes only that button's scoped material.
- Pointer and finger motion visibly move the highlight across the local surface.
- A quick tap produces a complete pulse; a held touch and small movement produce
  continuous following; release or cancellation always clears the pressed state.
- Starting a scroll on a button shows immediate touch feedback and keeps the
  glow active until release while the Scroller pans. The gesture does not
  increment the press counter.
- Click/tap activation increments the showcase counter exactly once.
- The layout is usable at desktop and compact/mobile widths, with explanatory
  copy changing to match the available interaction model.
- The complete grid and footer live inside a vertical `Scroller` whose explicit
  content height includes the final row and bottom padding on iOS.
- The example builds as a release-baked macOS cartridge, proving that
  `LightFrame` crosses the compiler/runtime boundary without extra serialized
  scope metadata.

## Settled MVP Decisions

- The public container is named `LightFrame`.
- Scope is lexical and follows expanded render-parent ancestry.
- Scope is one-way: outer lights enter; inner lights do not escape.
- The root behaves as an implicit frame.
- Component and repeat instances receive distinct scope identity.
- `LightFrame` scopes `LightSource`, not `AmbientLight`, in the MVP.
- `Material::unlit()` always wins.
- No eligible light plus no authored ambient means identity rendering.
- Existing logical-canvas-layer reachability remains an outer constraint.
- The GPU representation uses per-primitive membership, not extra canvas
  layers, selector strings, or material overloading.
- The existing eight-active-light limit remains, with deterministic overflow
  and diagnostics.
- Selector targeting and shared singleton lights are explicitly deferred.
