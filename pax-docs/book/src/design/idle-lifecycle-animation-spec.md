# Idle Lifecycle Animation (Draft)

Authoring Date: 2026-05-17

<!-- summary: Proposal for an @idle lifecycle animation slot between @in and @out. -->
<!-- tags: animation, lifecycle, transitions, runtime, compiler -->

## Problem

Pax currently has lifecycle transition bindings for entering and exiting mounted state:

- `@in` plays when a component or element enters the mounted tree.
- `@out` plays when a component or element leaves the mounted tree.

There is no matching lifecycle surface for the stable mounted state between those two phases. Authors can use ordinary timelines for continuous animation, but they do not compose with the lifecycle transition machinery: they are not naturally scoped to the enter/idle/out sequence, they do not automatically pause during exit, and they do not reuse the transition playhead/source routing built for element-level transitions.

The desired authoring model is an `@idle` lifecycle animation that starts after `@in`, loops during the mounted/default state, and stops when `@out` begins.

## Goals

- Add an `@idle` lifecycle animation binding parallel to `@in` and `@out`.
- Support component-level bindings in `@settings`.
- Support element-level named timeline bindings.
- Support element-level inline timeline bindings.
- Let idle animations loop by default while preserving existing `loop` timeline controls.
- Keep `@out` as the only lifecycle transition that retains removed nodes.
- Preserve existing `@in/@out` behavior for components that do not opt into `@idle`.

## Non-goals

- A general animation state machine.
- A full pause/resume/timeline-controller API in the first pass.
- New selector semantics beyond the existing transition timeline targeting model.
- Changing how ordinary non-lifecycle timelines work.
- Making idle animations participate in layout retention or route/repeat removal semantics.

## Proposed Authoring Model

Component-level idle uses the existing `@settings` transition style:

```pax
@settings {
    @in: enter,
    @idle: breathe,
    @out: exit,
}

@timeline breathe {
    duration: 1200ms,
    loop: true,
    self {
        rotate: {
            0%: {$base - 1deg},
            50%: {$base + 1deg},
            100%: {$base - 1deg},
        },
    }
}
```

Element-level idle follows the element-level `@in/@out` model. A named timeline can target the bound element with `self` or other nodes in the same containing component:

```pax
<Group id=panel @idle=panel_breathe />
<Text id=caption text="Ready" />

@timeline panel_breathe {
    duration: 900ms,
    loop: true,
    self {
        rotate: {
            0%: {$base},
            50%: {$base + 2deg},
            100%: {$base},
        },
    },
    #caption {
        y: {
            0%: {$base},
            50%: {$base - 2px},
            100%: {$base},
        },
    },
}
```

Inline idle is for transitions that affect only the element itself:

```pax
<Group
    @idle=@timeline {
        duration: 900ms,
        loop: true,
        rotate: {
            0%: {$base},
            50%: {$base + 2deg},
            100%: {$base},
        },
    }
/>
```

## Lifecycle Semantics

The runtime should treat idle as the stable mounted lifecycle phase:

1. When a node mounts, `@in` starts if present.
2. When `@in` reaches its configured duration, the node switches to `IDLE` and resets the transition origin/playhead if the node has an idle binding.
3. If a node has `@idle` but no `@in`, idle starts immediately on mount.
4. `@idle` samples on the lifecycle transition playhead while the node remains mounted.
5. When `@out` begins, it interrupts idle, resets the transition origin/playhead, and samples exit tracks.
6. Idle never retains nodes after removal; if there is no `@out`, removal remains immediate.

The compatibility rule is important: nodes without `@idle` should keep the current `@in` behavior. Today enter transitions remain in the enter phase and hold their final sampled value. The runtime should only perform an enter-to-idle handoff for nodes that actually have idle bindings.

## Looping

Idle should preserve the ordinary timeline `loop` setting. Recommended defaults:

- `@in`: force `loop=false`.
- `@out`: force `loop=false`.
- `@idle`: use the authored `loop` value, defaulting to `true` through normal timeline semantics.

This makes the common case concise while retaining an escape hatch:

```pax
@timeline settle_once {
    duration: 600ms,
    loop: false,
    self {
        scale: {
            0%: {$base},
            50%: {$base + 0.03},
            100%: {$base},
        },
    }
}
```

With `loop=false`, idle should run once and hold its final keyframe while the node remains in the idle phase.

## Compiler And Manifest Changes

Parser and formatting:

- Extend `transition_id` from `@in | @out` to `@in | @idle | @out`.
- Parse `@idle` in component `@settings` as `SettingsBlockElement::Transition`.
- Update diagnostics and formatter rule descriptions.

Manifest and cartridge generation:

- Add `idle` to `ComponentTransitionBindingInfo`.
- Add `idle` to element transition binding info.
- Add `has_idle`, idle duration fields, and `idle_sources` to `ComponentTransitionConfig`.
- Add an `idle: Option<TimelineTrackDefinition>` slot to `TransitionDefinition`.
- Include `@idle` in component-level timeline binding resolution.
- Include `@idle` in element-level transition source scans.
- Add an idle-aware transition track preparation path that preserves repeat semantics instead of forcing `repeat=false`.

Because `TransitionDefinition` crosses the compiler/runtime boundary, this change must audit generated Rust manifests, program IR sanitization, binary baking, and roundtrip tests.

## Runtime Changes

The runtime already has lifecycle transition symbols:

- `$transition_phase`
- `$transition_playhead`
- `$transition_playhead_millis`

The recommended implementation keeps `TRANSITION_PHASE_IDLE = 0` as the idle phase and extends sampling:

- `ENTER` samples `transition.enter`.
- `IDLE` samples `transition.idle` when present, otherwise the transition starting/base value.
- `EXIT` samples `transition.exit`, with the current fallback behavior for missing exit tracks.

`ExpandedNode` needs two additions:

- Transition symbols should be installed when a node has enter, idle, or exit bindings.
- An enter-completion effect should switch `ENTER -> IDLE` and reset the transition origin for nodes with idle bindings.

The existing exit cleanup listener is a nearby pattern for frame-driven lifecycle effects. Idle should not add cleanup retention work; it only needs the handoff and ongoing sampling.

## Disabling Idle

The first implementation should rely on declarative `loop` and lifecycle interruption. A separate imperative API can be added when there is a concrete caller.

If needed, a narrow node-local API is preferable:

```rust
ctx.set_idle_animation_enabled(false);
ctx.set_idle_animation_enabled(true);
```

Re-enabling should reset the idle origin so the loop restarts cleanly. The same operation can later be mirrored on `NodeInterface` for container-style code.

A declarative enablement surface such as `@idle_when={expr}` is possible, but it should be a separate design pass. It widens lifecycle transitions from static bindings into conditional animation controllers and needs clearer semantics for handoff, reset, and exit interruption.

## Tests

Coverage should include:

- Parser acceptance for component-level `@idle`.
- Parser acceptance for named and inline element-level `@idle`.
- Manifest lowering into `TransitionDefinition::idle`.
- Component transition config duration/source computation for idle tracks.
- Runtime sampling of looping idle tracks.
- Enter-to-idle handoff after frame-based and millisecond-based enter durations.
- Immediate idle start when no enter transition exists.
- Exit interruption of active idle animations.
- Release cartridge/binary roundtrip coverage for the new transition field.

## Open Questions

- Should idle default to `loop=true` only for lifecycle bindings, or should it simply inherit the existing timeline default? Recommendation: inherit the existing default.
- Should `@idle` be documented as a "handler" or as a "lifecycle animation binding"? Recommendation: use "lifecycle animation binding" to avoid confusion with Rust event handlers.
- Should component-level idle handoff happen independently for each node targeted by the idle timeline, or only for the component root? Recommendation: follow the current transition source model so every affected node owns its own phase/playhead.
- Should exit fallback sample idle when there is no explicit exit track? Recommendation: no for the first pass; exit should either use an explicit `@out` track or the existing fallback behavior.
