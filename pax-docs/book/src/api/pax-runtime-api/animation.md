# animation
<!-- summary: Animation primitives: interpolation, easing curves, transition queues, and timelines. See also: properties.ease_to and properties.ease_to_later. -->
<!-- tags: api, pax-runtime-api -->

Animation primitives: interpolation, easing curves, transition queues, and timelines. See also: properties.ease_to and properties.ease_to_later.

## Structs
### `Timeline`
Minimal timeline state used by animation-oriented controls.

#### Properties
##### `playhead_position`
Type: `usize`

Current playhead frame.

##### `frame_count`
Type: `usize`

Total number of frames.

##### `is_playing`
Type: `bool`

Whether the timeline is currently advancing.

## Enums
### `Duration`
A duration used by animation and transition systems.

`Frames` preserves Pax's historical frame-count semantics, while
`Milliseconds` and `Seconds` advance from the chassis-provided monotonic
wall clock.

#### Variants
##### `Frames`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Duration measured in runtime frames.

##### `Milliseconds`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Duration measured in milliseconds.

##### `Seconds`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Duration measured in seconds.

#### Implementations
##### `as_frames_f64`
<pre><code class="api-signature language-rust ignore">pub fn as_frames_f64(&amp;self) -&gt; f64</code></pre>

Converts the duration to frames, using 60fps as the nominal conversion
rate when converting wall-clock units for mixed-unit authoring.

##### `as_milliseconds_f64`
<pre><code class="api-signature language-rust ignore">pub fn as_milliseconds_f64(&amp;self) -&gt; f64</code></pre>

Converts the duration to milliseconds, using 60fps as the nominal
frame duration when converting frame units for mixed-unit authoring.

##### `is_frame_based`
<pre><code class="api-signature language-rust ignore">pub fn is_frame_based(&amp;self) -&gt; bool</code></pre>

Returns true when the duration is expressed in frame units.

##### `raw_value`
<pre><code class="api-signature language-rust ignore">pub fn raw_value(&amp;self) -&gt; f64</code></pre>

Raw value in this duration's own unit.

---

### `EasingCurve`
Pre-built easing curves for use in transitions, as well as a `Custom` variant that can be used to
specify an arbitrary easing curve via a function `f: f64 -> f64` mapping a time on the unit interval to a multiplier on the unit interval.

#### Variants
##### `Linear`
A linear easing curve, where the interpolated value changes at a constant rate over time.

##### `Hold`
A hold easing curve, where the interpolated value remains constant until the end of the transition, at which point it jumps to the final value.

##### `InQuad`
A quadratic easing curve where the interpolated value starts slow and accelerates towards the end of the transition.

##### `OutQuad`
A quadratic easing curve where the interpolated value starts fast and decelerates towards the end of the transition.

##### `InOutQuad`
A quadratic easing curve where the interpolated value starts slow, accelerates towards the middle of the transition, and then decelerates towards the end of the transition.

##### `InBack`
A back easing curve where the interpolated value starts by briefly moving in the opposite direction before accelerating towards the final value.

##### `OutBack`
A back easing curve where the interpolated value overshoots the final value before settling back to it.

##### `InOutBack`
A back easing curve where the interpolated value starts by briefly moving in the opposite direction, then accelerates towards the final value, overshooting it before settling back to it.

##### `Custom`(`Box`<`dyn` `Fn`(`f64`) -> `f64`>)
A custom easing curve defined by a user-provided function that maps a time on the unit interval to a multiplier on the unit interval.

#### Implementations
##### `interpolate`
<pre><code class="api-signature language-rust ignore">pub fn interpolate&lt;T: <a href="../../api/pax-runtime-api/animation.md#interpolatable">Interpolatable</a>&gt;(&amp;self, v0: &amp;T, v1: &amp;T, t: f64) -&gt; T</code></pre>

Interpolates between `v0` and `v1` using `t` as time on the unit interval.

## Traits
### `Interpolatable`
Marks a value as able to participate in Pax property transitions.
