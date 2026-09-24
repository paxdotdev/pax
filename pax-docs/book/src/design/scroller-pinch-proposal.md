# Proposal: general pinch input and scroll arbitration

Status: deferred; early, unvalidated design draft retained as prior art. No pinch
gesture support has been implemented or validated on any chassis. Names and
snippets below describe possible new APIs and are not available in Pax today.

Follow-up: [PAX-1003 — Pinch gesture support in Pax Core](https://linear.app/paxdev/issue/PAX-1003/design-and-implement-pinch-gesture-support-in-pax-core).

## Open decisions before implementation

This document records one candidate design, not an approved specification. Its
prescriptive language describes that candidate only. In particular, the proposed
Managed touch policy, event payload, lifecycle, and Scroller arbitration have not
been established by prototypes or device testing.

- How much recognition, capture, pan, momentum, and gesture arbitration should
  defer to native platform/browser handlers versus shared Pax logic? Which host
  differences should be exposed, normalized, or treated as capability limits?
- How should pinch events coexist with raw @touch events? Should both streams
  remain observable, recognize simultaneously, or become exclusive after a
  threshold? Define their ordering and what existing touch handlers observe when
  a pinch starts, ends, or is canceled.
- Can a touch handler suppress pinch recognition, or a pinch handler cancel the
  corresponding touch interaction? Distinguish event propagation/bubbling,
  prevent-default behavior, recognizer ownership, and cancellation of another
  event stream. Do not assume an existing stop-propagation mechanism solves all
  of these, especially after a native gesture has already started.
- Where should handlers and ownership policy attach: any event-capable element,
  a container/gesture region, a root handler, or some combination? How do nested
  surfaces and native editable controls participate?
- Is any public Scroller API change necessary? Re-evaluate the proposed shared
  touch policy and internal Scroller coordination after testing native and web
  event plumbing, rather than treating either as a settled requirement.

Direction from the calculator discussion: application code should retain zoom
coefficients, sensitivity, limits, anchoring, and rendering decisions. Prefer the
explicit name @pinch_change (consistent with @slider_change), and explore generic
input events instead of making zoom a Scroller responsibility. These preferences
do not resolve the plumbing questions above.

## Objective and ownership

Expose pinch through the ordinary Pax input system on event-capable elements,
including Group, Frame, and Scroller. Support native iOS/iPadOS and mobile web,
including adding a second finger after a pan begins. Keep ordinary native input
unchanged unless a surface explicitly opts into managed touch ownership.

The application owns its zoom coefficient, limits, sensitivity, focal-point
anchoring, and rendering strategy. A handler may scale a subtree, rebuild graph
geometry, reflow text, or use the measurements for something unrelated to zoom.
The recognizer owns only contact capture, gesture baselines, measurements, and
lifecycle. Scroller neither stores zoom nor scales its slotted contents.

There is no Scroller-specific PanAndZoom/PanAndPinch public mode in this revised
proposal. Internal coordination with scrolling hosts is still needed to prevent
pan, momentum, and stale host offsets from fighting application-authored updates.
A separate scroll_enabled property could be useful independently, but is not
required in the minimum pinch API.

## Current calculator behavior

Graph zoom changes pixels per world unit in calculator state, regenerates curve,
grid, and axis geometry in logical pixels, and updates the Scroller content size
and offsets. It does not apply a scale transform to all slotted children.
Calculate zoom changes font/cell metrics and reflows history. These states are
independent. Scroller supplies clipping and offset translation, not zoom.

## Proposed public surface

Use @pinch_change, consistent with @slider_change and @checkbox_change. Add
@pinch_start, @pinch_end, and @pinch_cancel alongside it. These are regular routed
events, not callbacks unique to Scroller. A root handler is possible, but two
unrelated contacts elsewhere in a window must not become one global pinch.

Proposed authoring shape (touch_policy and its enum names remain reviewable):

```pax
<Group width=100% height=100%
    touch_policy=TouchPolicy::Managed
    @pinch_start=begin_zoom
    @pinch_change=update_zoom
    @pinch_end=end_zoom
    @pinch_cancel=cancel_zoom
>
    <!-- Application-owned content, optionally including a Scroller. -->
</Group>
```

The common input property touch_policy defaults to Native, preserving existing
host behavior. Managed reserves the touch stream for Pax routing before contact.
The reliable pinch contract applies to managed regions; merely binding a handler
does not silently disable native scrolling or browser page zoom. Policy changes
apply to the next sequence. A plain Group does not acquire automatic pan/zoom:
one-finger input remains ordinary touch input for userland. An eligible Scroller
on the hit path can supply panning and momentum.

Each event carries:

- A sequence ID and two stable contact IDs with their current positions.
- The centroid, current contact distance, and start_distance.
- delta_distance since the last delivered change (zero at start).

Positions and distances use window logical pixels, consistent with existing
Touch events. They stay independent of the content transforms being changed by
the application. Existing coordinate conversion can map the centroid to a local
viewport for anchoring. No persistent application zoom value is in the event.
A typical userland mapping is:

```text
zoom = clamp(start_zoom * distance / start_distance, min_zoom, max_zoom)
```

Applications may instead use additive deltas, nonlinear sensitivity, or discrete
font sizes. Reject degenerate starting separations and never emit nonfinite
measurements. Coalescing preserves cumulative measurements and computes deltas
against the last delivered update. Flush a pending final change before end;
terminal events carry the last valid measurements. Do not drop lifecycle events.

## Why generic events still need gesture ownership

Mobile browsers decide direct-manipulation ownership before an application can
reactively disable scrolling on the second touch. Changing touch-action after a
gesture starts cannot reclaim it. See the
[Pointer Events direct-manipulation contract](https://www.w3.org/TR/pointerevents3/#determining-supported-direct-manipulation-behavior).
Current web touch-start listeners are also passive, so their returned
prevent_default cannot cancel the browser action.

Separately, the current native Scroller ignores programmatic offsets during
tracking, dragging, or deceleration, and the web host ignores engine offset
patches during active iOS WebKit scrolling. These guards protect against stale
echoes but would reject offsets needed to anchor an application pinch.

General events therefore remove the need for a Scroller-specific zoom API, but
not the need to coordinate input ownership and accepted offset updates. The
ownership policy belongs in the shared input layer; Scroller participates only
where its scrolling behavior is involved.

## Gesture lifecycle and scroll arbitration

1. The first contact selects the deepest eligible managed pinch region. A second
   contact must begin in that same region. Capture persists outside its bounds;
   unrelated contacts are never paired. Normal event bubbling does not create
   additional recognizers or grant ancestors ownership.
2. One finger can pan the nearest eligible Scroller on the hit path. Recognition
   of a pinch stops pan/coast/snap before pinch_start and snapshots the actually
   presented offsets. Plain containers need no Scroller to recognize a pinch.
3. During pinch, userland controls content and offsets. Native pan, wheel, snap,
   and stale host echoes cannot overwrite those updates. Centroid movement is
   delivered to the app, not also applied as a Scroller pan. Apply content extents
   and anchor offsets in one view update before clamping to avoid shrink jumps.
4. A third contact does not replace the captured pair. When a tracked contact
   lifts, flush the final change and end the pinch. Rebase any remaining pan at
   its current position with zero initial velocity. A new eligible pair starts a
   fresh sequence and baseline.
5. Cancellation, unmount, suspension, or orientation/layout changes that invalidate
   the captured coordinate frame send one cancel and clear capture, press state,
   and velocity. Preserve the last applied view; do not roll back application
   zoom. Recognized pan/pinch sequences must not later activate a button.

Managed surfaces contain their active gestures instead of handing them to an
ancestor page at a boundary. Gestures starting outside retain ordinary behavior.
Nested native editable controls keep exclusive input; mixing native selection
and managed pinch in the same region is an explicit v1 limitation. The calculator
can opt in its plot/history surface while keeping the editor outside it.

## Host implementation and compatibility

- **Mobile web:** establish touch-action: none on managed hit surfaces before
  contact. Canvas-rendered regions also need host hit-region plumbing; registering
  Rust handlers alone is insufficient. Use one captured pointer/touch stream and
  suppress compatibility mouse duplication. Restrict any necessary nonpassive
  listeners to managed surfaces. Managed Scrollers supply touch pan/momentum
  where browser-owned pan cannot guarantee transfer to application pinch. Keep
  wheel and scrollbars when no touch gesture owns the view. Managed root Scrollers
  do not use page-scroll delegation; ordinary root Scrollers retain it.
- **iOS/iPadOS:** implement the same shared event contract. Native pan may remain
  where it satisfies the transition contract, but must stop before authored
  pinch offsets apply. Make active-scroll rejection ownership-aware while
  retaining stale-echo protection for ordinary scrolling.
- **Desktop:** preserve mouse, wheel, keyboard, and scrollbar behavior. Touchscreen
  pinch follows the touch contract. Trackpad magnification, Ctrl-wheel zoom, and
  rotation are separate follow-ups rather than claimed v1 support.

Where native momentum cannot be retained, managed Scrollers use bounded,
time-based deceleration, clamping, and velocity reset on touch/pinch/cancel.
Snap runs after pan/coasting settles, never during pinch. Exact OS physics and
elastic overscroll are not promised. This host pan/momentum work is part of the
proposed scope, even though it does not introduce a Scroller zoom API.

Browser page pinch is replaced only inside opted-in regions; do not disable page
zoom globally or alter viewport meta tags. Preserve accessible zoom buttons and
keyboard alternatives. OS accessibility magnification remains external.

## Calculator integration

- Graph maps contact measurements to continuous scale within the existing 4–256
  logical pixels/unit bounds. Capture the world point under the initial centroid
  and update offsets so it follows the current centroid. Rebuild visible curve
  geometry with the same work/memory caps. Preserve the draft and submitted
  formula. Cartesian and polar use the same gesture; zoom keys double/halve.
- Calculate maps measurements to existing 12/14/16/20/24px text sizes with midpoint
  thresholds and hysteresis. Preserve a history-entry/character anchor under the
  centroid where possible, clamping when content is shorter than the viewport.
  Keep the fixed editor separate.
- Modes keep independent zoom state. Coalesce expensive resampling and reflow to
  at most once per presented frame. All mappings and limits remain userland.

## Acceptance and delivery

First prove a generic Group pinch without Scroller, then nested Scroller
coordination, then the calculator. Validate on physical Safari on iPhone/iPad,
Android Chrome, and native Argus/Molino, plus desktop scrolling regressions.
Responsive browser emulation alone does not establish multitouch correctness.

Cover pan-to-pinch-to-pan, third contacts, leaving bounds, unrelated contacts,
background/cancel/unmount, orientation/layout changes, nested ownership, no
accidental activation, coalescing, min/max zoom, pan/momentum/snap interruption,
and ancestor page scrolling outside the surface. Check authored content-size and
offset changes together for anchor jumps and stale host echoes. Report a failing
host as unsupported until corrected instead of adding calculator workarounds.

Update shared events, handler metadata, routing, chassis/host codecs, input-region
metadata, and Scroller coordination together. Audit manifest/program IR, binary
serialization, Rust manifest conversion, and release cartridge baking for new
common properties, enum values, and event bindings. Exercise debug and baked
release round trips and example builds. Update canonical event/scrolling articles
and generated API docs when implementation establishes the behavior.

Next step when this work resumes: revisit the open decisions above and validate
small native/mobile-web prototypes before agreeing on an implementation spec.
Keep this candidate as prior art; do not treat it as the task's acceptance contract.
