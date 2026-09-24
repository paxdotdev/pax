# core::import_settings
<!-- summary: API docs for pax-std::core::import_settings. -->
<!-- tags: api, pax-std -->

## Structs
### `ImportSettings`
Mounts a non-rendering subtree whose component descendants export selector settings.
`transition=SettingsTransition::Ease(400ms, TransitionCurve::InOutQuad)`
eases changes to effective settings after the initial appearance. Explicit
inline values, property timelines, and two-way bindings retain ownership.

#### Properties
##### `transition`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`SettingsTransition`](../../../api/pax-std/core/import_settings.md#settingstransition)>

Optional motion for changes to effective imported settings. Each new
target starts a full-duration transition from the currently displayed value.

## Enums
### `SettingsTransition`
Motion shared by the settings exported from one ImportSettings instance.
Named timeline playback is intentionally not supported yet.

#### Variants
##### `None`
Apply settings immediately (the default).

##### `Ease`([`Duration`](../../../api/pax-runtime-api/animation.md#duration), [`TransitionCurve`](../../../api/pax-std/core/import_settings.md#transitioncurve))
Interpolate changed settings using this duration and easing curve.
Nonpositive or non-finite durations apply the destination immediately.

---

### `TransitionCurve`
Built-in easing curves for automatic settings transitions.

#### Variants
##### `Linear`
Constant progress.

##### `Hold`
Retain the source until the duration ends.

##### `InQuad`
Accelerate from rest.

##### `OutQuad`
Decelerate into the destination.

##### `InOutQuad`
Accelerate, then decelerate.

##### `InBack`
Move back before accelerating forward.

##### `OutBack`
Overshoot, then settle.

##### `InOutBack`
Anticipation and overshoot.
