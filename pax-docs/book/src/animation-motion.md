# Animation & Motion
<!-- summary: Timelines, easing, and procedural motion. -->
<!-- tags: animation, motion -->

- Imperative animations: .ease_to and related APIs
- Timeline syntax: `@timeline` blocks, durations, and keyframes.
- @timeline blocks vs. inline syntax
- Lifecycle transitions with `@in` and `@out`.
- Easing curves and interpolation behavior.
- Path-based and procedural animation patterns.
- Syncing multiple animations with shared playheads.
- Examples from `transition-grid`, `timeline-playground`, `marionette`, and `mouse-animation`.

## Timelines

Timelines animate properties declaratively.  A timeline track describes values at frames, durations, or percentages, and Pax samples the track as time advances or as a bound playhead changes.

```pax
<Rectangle id=card opacity=@timeline {
    duration: 30,
    loop: false,
    0: 0,
    100%: 1,
} />
```

Use `duration` to set the timeline length.  Durations may be expressed in milliseconds, seconds, explicit frames, or a unitless frame count.

```pax
<Rectangle id=card opacity=@timeline {
    duration: 250ms,
    loop: false,
    0ms: 0,
    250ms: 1,
} />

<Rectangle id=spinner rotate=@timeline {
    duration: {(100 + self.offset)ms},
    0%: 0deg,
    100%: 360deg,
} />
```

Frame-based timelines are written as `duration: 30` or `duration: 30f`.  Numeric frame markers keep their existing behavior.

Named timelines can target selectors in the component template.

```pax
@timeline pulse {
    duration: 60,
    #badge {
        opacity: {
            0%: 0.4,
            50%: 1,
            100%: 0.4,
        },
    }
}
```

Timeline keyframes may use `$base` inside `{...}` expressions. `$base` is the value the property had before this timeline layer was applied, which is useful for relative motion that should not duplicate the layout formula.

```pax
@timeline enter {
    duration: 18,
    self {
        opacity: {
            0: 0,
            18: {$base},
        },
        y: {
            0: {$base - 32px},
            18: {$base},
        },
    }
}
```

## `@in` Transitions

An `@in` transition plays when a component instance enters the mounted tree.  Bind `@in` in the component's `@settings` block to the name of a timeline in the same component.

```pax
@settings {
    @in: enter,
}

@timeline enter {
    duration: 300ms,
    self {
        opacity: {
            0ms: 0,
            300ms: 1,
        },
    }
}
```

The transition timeline targets the entering component's own template.  Use `self` for properties on the component itself, and selectors such as `#title` or `.item` for children.

`@in` can also be bound directly on an element.  A named element-level transition uses a timeline from the containing component, so selector targets are resolved in that same template:

```pax
<Group id=panel @in=panel_enter />
<Text id=caption text="Ready" />

@timeline panel_enter {
    duration: 300ms,
    self {
        opacity: {
            0ms: 0,
            300ms: 1,
        },
    },
    #caption {
        y: {
            0ms: {$base + 12px},
            300ms: {$base},
        },
    },
}
```

For a transition that only affects the element itself, declare the timeline inline with property tracks:

```pax
<Group
    @in=@timeline {
        duration: 300ms,
        opacity: {
            0ms: 0,
            300ms: 1,
        },
    }
/>
```

## `@out` Transitions

An `@out` transition plays when a component instance leaves the mounted tree, such as when an `if` branch changes or a keyed `for` item is removed.  Bind it the same way as `@in`.

```pax
@settings {
    @in: enter,
    @out: exit,
}

@timeline exit {
    duration: 300ms,
    self {
        opacity: {
            0ms: 1,
            300ms: 0,
        },
    }
}
```

Exiting instances remain mounted while their `@out` transition is running, then are removed.  If an exit transition cannot complete, Pax applies a default timeout so stale nodes are not retained indefinitely.

Element-level `@out` uses the same named and inline forms as element-level `@in`.

Entering and exiting children run in parallel by default.  For repeated lists, use keyed `for` loops when the identity of an item should survive reordering or insertion; removed keys can then play `@out` while retained keys keep their existing component instances.

### Interrupted `@in` / `@out`

When a stable instance reverses directly from `@in` to `@out`, or from `@out` back to `@in`, the destination timeline takes over from each property's currently sampled value by default. Its remaining authored keyframes, duration, and easing are unchanged. Conditional branches, route branches, and repeated children can rescue an instance that is still mounted for `@out`; keyed repeats use the declared key as the instance identity.

Set `interruption: Restart` on a named or inline lifecycle timeline when it should always begin from its authored starting value instead:

```pax
@timeline exit {
    duration: 300ms,
    interruption: Restart,
    self {
        opacity: {
            0ms: 1,
            300ms: 0,
        },
    }
}
```

The default is `Takeover`, which can also be written explicitly. This setting applies only to direct `@in` / `@out` reversals on the same mounted instance. It does not change `$base`, ordinary timeline playback, or the identity rules for unrelated instances.

## Container-owned Motion

`@in` and `@out` define how a component animates itself.  Layout containers can also decide how sibling placement reacts while those transitions are running.

This behavior depends on the runtime distinction between `received_children` and exit-retained payload.

`Stacker` exposes separate knobs for exit behavior and reflow behavior:
- `exit_mode` controls whether exiting children remain in normal stack flow or hold their prior frames as ghosts
- `reflow_transition` controls whether surviving children snap or ease into their new stack positions; Stackers default to `Snap`

```pax
<Stacker id=list gutter=12px>
    for (item, i) in self.items key item.id {
        <Chip label={item.label} />
    }
</Stacker>

@settings {
    #list {
        exit_mode: ContainerExitMode::Ghost
        reflow_transition: {
            kind: ContainerReflowTransitionKind::Ease
            duration: 18
            curve: ContainerReflowCurve::OutQuad
            name: ""
        }
    }
}
```

With `exit_mode: ContainerExitMode::Ghost`, an exiting child can finish its `@out` transition from its previous stack frame while the retained children resolve layout without it.  By default, `reflow_transition.kind: ContainerReflowTransitionKind::Snap` switches immediately.  Opt into `Ease` when retained children should animate between previous and new stack frames.  `Named` is reserved for a future current-component motion resource and is not wired yet.

## Imperative Easing

Rust handlers can animate a `Property<T>` with `.ease_to` or enqueue with `.ease_to_later`.  Passing a plain number preserves historical frame-based timing; pass `Duration` for wall-clock timing.

```rust
use pax_runtime_api::{Duration, EasingCurve};

self.opacity
    .ease_to(1.0.into(), Duration::Milliseconds(250.into()), EasingCurve::OutQuad);
self.opacity
    .ease_to_later(0.5.into(), Duration::Seconds(1.into()), EasingCurve::InOutQuad);
self.opacity
    .ease_to_later(0.0.into(), Duration::Frames(18.into()), EasingCurve::Linear);
```
