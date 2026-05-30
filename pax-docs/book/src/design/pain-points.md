# Pain points

Example (structure is not rigid but this showcases the style of notes we should be gathering.

## 2026-05-19

Tried to do `x`, ran into `y`, solved it by `z`.  
Recommendations:  (e.g. document in pax-docs, document in API docs (inline comments), update AGENTS.md, create a more targeted example, consider redesigning APIs, etc.) 

## 2026-05-20

Mobile WebKit requires `DeviceOrientationEvent.requestPermission()` /
`DeviceMotionEvent.requestPermission()` to run inside a user gesture. The web
chassis originally handled that by injecting a global "Enable motion" button,
which made every Pax web app show gyro-specific UI on mobile.

Solved by keeping sensor startup in the web chassis but exposing
`window.paxRequestDeviceSensorPermissions()` for app-owned controls. The
`gyro-helper` example owns its visible `Enable motion` button and calls that hook
from its Pax `Button` handler, so other web apps do not inherit the prompt.

Recommendations: document the helper near `$gyro` / `$accel` web-target docs if
this becomes an officially supported developer-facing API, and consider a
typed Pax runtime API for requesting platform permissions instead of direct web
JS interop.

## 2026-05-21

Tried to use `examples/src/starter-project` as a quick iOS chassis smoke test,
but its path dependencies include other example crates with their own `#[pax]`
roots. Building it through `pax-cli` set `PAX_DIR` to `starter-project/.pax`,
then the dependent examples failed the active-project guard because their roots
did not match that `PAX_DIR`.

Solved by switching chassis validation to a single-root example such as
`examples/src/increment` or `examples/src/router-playground`.

Recommendations: document preferred single-root smoke-test examples for chassis
work, or add a dedicated minimal iOS/macOS validation example that avoids
cross-example Pax dependencies.

While converting `neon-opacity` gradients to `@gradient`, elements following
`opacity=1` rendered with default slate fills. The parser had interpreted the
newline-separated `f` in the next `fill` attribute as the frame unit, producing
`opacity=1f` and a bogus `ill` setting.

Solved by making number+unit literals grammar-atomic so units must be adjacent:
`10px`, `50%`, and `(expr)px` are valid; `10 px` is rejected.

Recommendations: keep unit suffixes whitespace-free in docs and examples, and
add parser regressions whenever a grammar shorthand can consume the first
character of a neighboring attribute.

## 2026-05-25

The `color-picker` 2D saturation/lightness field was authored as generated
`ImageSource::Data` pixels, which made hue reactivity harder to reason about
while testing gradient syntax. The same HSL-style field can be represented more
directly as two orthogonal gradient fills: a horizontal gray-to-selected-hue
gradient under a vertical white-to-transparent-to-black gradient.

Solved by replacing the generated palette image with two `@gradient` rectangles
and driving the hue stop from a computed `Color` property.

While validating this on the iPad simulator, percentage stops rendered too early
because the GPU shader compared device-pixel-scaled coordinates against
unscaled stop distances. On a 2x DPR screen, the 100% stop landed halfway across
the fill. The symptom looked like broken stop alpha, but the manifest and
runtime color data were correct.

The same example also exposed native iOS slider jitter: the Swift view applied
the last runtime value on every render pass while the user was tracking the
thumb, so the control could fight an in-progress drag. A follow-up diagnostic
showed that stock iOS liquid-glass `UISlider` interaction itself uses smoothed
motion that can look like momentum or a spring-loaded release. That behavior is
native, not a Pax echo bug. The bridge keeps the native slider as the
hit-tested control, avoids runtime value writes during active tracking, keeps
the local Swift element value in sync with values sent to Rust, and avoids
echoing native-origin value changes straight back to the same native control.

The slider also exposed a runtime binding trap: `bind:` parsed correctly as
`DoubleBinding`, but generated property descriptor code applied it through
`Property::replace_with`. That copied the bound property's current value into
the child property instead of preserving the source property handle, so native
slider interrupts updated only the slider-local `value`. Unconditional final
`DoubleBinding` settings now assign the property handle directly.

The macOS color picker exposed a separate native-overlay tangent: the app
template only emitted `Click` interrupts from a SwiftUI gesture, while the
palette was authored with `@mouse_down`, `@mouse_move`, and `@mouse_up`.
Native-overlay containers also need to pass through empty hit-test regions so
canvas events can reach the vector event layer below. The macOS template now
dispatches pointer events from the canvas `NSView`, and the shared Apple bridge
routes `MouseDown` / `MouseMove` / `MouseUp`.

Recommendations: prefer vector gradients over generated image buffers when a
visual is naturally expressible as fills; it keeps reactivity in the normal
property graph and gives gradient syntax examples more realistic coverage. When
debugging gradient alpha, check the renderer's coordinate space before assuming
parse or manifest loss. Native form controls should not overwrite active user
tracking state from stale runtime values. For form controls, verify that
`bind:` preserves property identity, not only initial value mirroring. When a
target supports both canvas and native controls, verify event parity for
`Click`, mouse down/move/up, wheel, and touch rather than assuming a click-only
smoke test covers drag-oriented widgets.

## 2026-05-28

Firefox websocket failures from the designtime privileged-agent connection can
arrive as generic browser events. `ewebsock` 0.4 read `ErrorEvent.message()`
through a generated wasm-bindgen getter, which could throw
`expected a string argument, found undefined` before Pax saw the close/error
event. The client then logged noisy partial failures instead of quietly keeping
the designtime session alive for reconnect.

Solved by upgrading `pax-designtime` to `ewebsock` 0.8, whose wasm error path
uses JS reflection for optional error fields, and by keeping explicit tests for
close/error events scheduling reconnect without failing `handle_recv`.
Direct wasm validation also required enabling the `web-sys` `Location` feature
because `DesigntimeManager` reads `window.location().origin()`.

Recommendations: when browser websocket behavior changes, validate
`pax-designtime` directly with `cargo check -p pax-designtime --target
wasm32-unknown-unknown` in addition to native unit tests.

## 2026-05-29

Clean Ubuntu first-touch validation exposed host-side Linux development
dependencies that are easy to miss from macOS. Building `pax-cli` from source on
Ubuntu 26.04 ARM64 failed in `glib-sys` until the VM had GLib, Cairo, and Pango
development packages installed. The dependency came through Pax's current
text/rendering stack, not through the generated user's app logic.

Solved in the first-touch Ubuntu harness by adding `libglib2.0-dev`,
`libcairo2-dev`, and `libpango1.0-dev` to the baseline package set.

The same smoke also showed a stale web-interface contract path. Non-libdev
projects receive the web interface from `pax-compiler`'s embedded
`files/interfaces/web/public` output. The TypeScript source had mostly moved
from `occlusionLayerId` to `renderLayerId`, but an ignored generated public
bundle and `photoPickerCreate` still carried the old name. A fresh generated app
compiled successfully, then failed at runtime with `undefined id or
occlusionLayer` and rendered a black page.

Recommendations: after changing compiler/runtime interface message shapes,
rebuild the web interface bundle before packaging or source-linked smoke tests,
and grep both `src` and generated `public` interface files for retired contract
terms. For first-touch CI, include a browser smoke that loads and clicks the
fresh generated app, not just `pax-cli build`.
