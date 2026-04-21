# Animation & Motion
<!-- summary: Timelines, easing, and procedural motion. -->
<!-- tags: animation, motion -->

- Imperative animations: .ease_to and related APIs
- Timeline syntax: `@timeline` blocks, frames, and keyframes.
- @timeline blocks vs. inline syntax
- Lifecycle transitions with `@in` and `@out`.
- Easing curves and interpolation behavior.
- Path-based and procedural animation patterns.
- Syncing multiple animations with shared playheads.
- Examples from `transition-grid`, `timeline-playground`, `marionette`, and `mouse-animation`.

## Timelines

Timelines animate properties declaratively.  A timeline track describes values at frames or percentages, and Pax samples the track as time advances or as a bound playhead changes.

```pax
<Rectangle id=card opacity=@timeline {
    frames: 30,
    loop: false,
    0: 0,
    100%: 1,
} />
```

Named timelines can target selectors in the component template.

```pax
@timeline pulse {
    frames: 60,
    #badge {
        opacity: {
            0%: 0.4,
            50%: 1,
            100%: 0.4,
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
    frames: 18,
    self {
        opacity: {
            0: 0,
            18: 1,
        },
    }
}
```

The transition timeline targets the entering component's own template.  Use `self` for properties on the component itself, and selectors such as `#title` or `.item` for children.

## `@out` Transitions

An `@out` transition plays when a component instance leaves the mounted tree, such as when an `if` branch changes or a keyed `for` item is removed.  Bind it the same way as `@in`.

```pax
@settings {
    @in: enter,
    @out: exit,
}

@timeline exit {
    frames: 18,
    self {
        opacity: {
            0: 1,
            18: 0,
        },
    }
}
```

Exiting instances remain mounted while their `@out` transition is running, then are removed.  If an exit transition cannot complete, Pax applies a default timeout so stale nodes are not retained indefinitely.

Entering and exiting children run in parallel by default.  For repeated lists, use keyed `for` loops when the identity of an item should survive reordering or insertion; removed keys can then play `@out` while retained keys keep their existing component instances.
