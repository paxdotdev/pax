# Demand-Driven Frame Scheduling (Exploration)

Authoring Date: 2026-08-11

<!-- summary: Exploration of pausing Pax's display-synchronized tick when no mounted work observes time. -->
<!-- tags: runtime, performance, scheduling, animation, chassis -->

## Status

This is a light design proposal, not an implementation specification. It is
intended to test the shape of an idea and identify the questions that would
need to be resolved before implementation.

## Motivation

Pax currently uses a display-synchronized tick while an application is
mounted. The oscillator-like clock is a useful architectural primitive:
timelines, transitions, `$frames` and `$millis`, and lifecycle handlers can all
observe a coherent progression of time.

The render path is already substantially lazy. If no canvas layer or removal
is dirty, rendering exits before scene traversal, resource uploads, command
encoding, or GPU submission. At rest, however, the chassis still wakes the
runtime on every display tick and the runtime still performs CPU work before
discovering that there is nothing to draw.

The opportunity is to preserve Pax's clock model while making delivery of its
pulses demand-driven. A clock does not necessarily need to pulse when nothing
mounted is observing it.

## Proposed Direction

After processing a frame or an external interrupt, the runtime could tell the
chassis when it needs to wake again. The exact API is deliberately left open,
but the result might have four conceptual states:

- **Vsync:** Active work observes every display tick, such as an animation,
  transition, frame-dependent expression, or continuous lifecycle handler.
- **Deadline:** No continuous frame is needed, but known work must run at a
  particular monotonic time.
- **Immediate:** Effects, messages, or newly dirtied work require another pass
  without waiting for the next external event.
- **Idle:** No mounted work currently observes time and no work is pending. The
  chassis can stop scheduling frames.

`Vsync` would keep the oscillator behaving as it does today. `Idle` would not
remove or replace the clock; it would pause delivery until something makes the
clock observable again.

## Waking an Idle Application

External interrupts would wake the runtime and give it an opportunity to
recompute frame demand. Likely wake sources include:

- pointer, touch, keyboard, scrolling, focus, and resize events;
- asynchronous image, font, asset, data, or network completion;
- hot reload and designtime changes;
- native element events and platform lifecycle changes; and
- timers or other work with a registered deadline.

These events are wake sources, not substitute clocks. Once awake, the runtime
would process the interrupt, settle reactive work, render if necessary, and
then report whether it needs continuous ticks, a future deadline, an immediate
follow-up, or no scheduled wake at all.

## Tracking Frame Demand

The first implementation should be conservative: if the runtime cannot prove
that it is safe to sleep, it should keep requesting vsync. Continuous demand
could include:

- active animations and transitions;
- live PAXEL dependencies on `$frames` or `$millis`;
- mounted `@tick` or other handlers whose contract requires every frame;
- native scrolling inertia, media, sensors, or platform work that holds a
  frame lease; and
- unsettled reactive effects, queued native messages, or dirty render state.

This likely calls for explicit demand accounting rather than inferring idleness
only from the property graph. Completed timelines or detached nodes may leave
clock-shaped dependencies behind, and those stale edges should not
accidentally pin the whole application to vsync. Explicit accounting would
also make the reason an application remains awake inspectable in developer
tools.

`@pre_render` needs a deliberate semantic decision. It could conservatively
hold a continuous frame lease, or it could run only before frames scheduled for
another reason. That decision should be made before treating the idle state as
observable behavior.

## Time Semantics

When an idle application wakes, Pax should sample monotonic time before
settling reactive work. Millisecond-based animation can then advance by real
elapsed time instead of replaying every missed display pulse.

Frame-counted behavior is different: skipping pulses changes its meaning. Any
active observer whose behavior depends on `$frames` should therefore keep
vsync armed. Longer term, Pax may want to distinguish display-frame count from
logical update count, but this proposal does not require that change.

## Chassis Integration

On the web, the chassis could request `requestAnimationFrame` only while the
runtime asks for vsync, use a timer for a known deadline, and let DOM or async
events arm the next frame from idle. On Apple platforms, the corresponding
display link could be paused and resumed.

The runtime/chassis handshake must prevent a lost-wakeup race: an event can
arrive while the runtime is deciding to sleep. A small synchronized state such
as `frame_armed` plus `wake_pending`, or an equivalent generation counter,
should ensure that work arriving across that boundary always schedules another
pass.

## Incremental Exploration

A useful first step would be instrumentation rather than sleeping. The runtime
could report why each next frame remains armed and surface those reasons in
profiling or developer tools. That would reveal stale clock dependencies and
unclassified continuous work without changing behavior.

From there, a conservative experiment could:

1. Pause only when there are no animations, clock-dependent expressions,
   continuous handlers, pending effects or messages, or dirty render layers.
2. Prove the wake handshake on the web, where scheduling behavior is easy to
   observe.
3. Extend the same contract to macOS, iOS, and iPadOS display links.
4. Add deadline scheduling for timers and other known future work after the
   idle/vsync boundary is reliable.
5. Publish idle CPU and energy measurements across representative targets and
   compare them against the continuously ticking baseline.

## Non-Goals

- Replacing Pax's display-synchronized clock or changing animation semantics.
- Adding a new public timer or scheduling API as part of the first experiment.
- Claiming zero system energy use; operating systems, compositors, native
  controls, and media may continue to do work outside Pax.
- Sleeping through work whose lifecycle or timing contract is not yet
  understood.

## Open Questions

- Which lifecycle handlers are continuous clock consumers, and which should
  run only on an otherwise scheduled frame?
- What owns and releases a frame lease for native scrolling, video, sensors,
  and other platform-driven activity?
- Can clock dependencies be detached reliably when timelines complete or
  nodes unmount, and how should stale dependencies be diagnosed?
- How should background-tab throttling and long suspended intervals affect
  frame-counted and millisecond-counted behavior?
- Which idle CPU, GPU, and energy measurements should become regression
  benchmarks for each target?
