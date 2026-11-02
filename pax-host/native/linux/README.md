# Linux GTK host

This SDK runs a real Pax cartridge in a GTK 4 application. A user-owned Rust
`main` uses `pax_kit::init_with!` and `pax_host` to configure the cartridge, pass
startup data, and open a standalone window. The lower-level host ABI also has
`GtkBox`/empty `GtkWindow` mount entry points, but public Rust embedding and its
lifecycle are not qualified for Linux v0. The compiler packages the cartridge, this shared library, and ordered
asset roots beside the executable. The host and cartridge negotiate desktop ABI 4.

Build the native SDK on Linux:

```sh
./pax-host/native/linux/build.sh --output-directory /absolute/sdk --configuration Debug
```

Development packages must provide GTK >=4.14, epoxy, json-glib, Fontconfig, and PangoFc (`pangoft2`)
through `pkg-config`. The supported renderer selection is `GSK_RENDERER=ngl`;
legacy GTK 4.14 `gl` has a separately recorded DMA-BUF texture-binding failure in
the platform probes. A compatible Vulkan physical GPU, exportable linear RGBA8
DMA-BUF image, foreign queue ownership, binary SYNC_FD import/export, EGL native
fences, EGL DMA-BUF modifiers, and GLES3 external images are required. There is no
software adapter or CPU presentation fallback. `PAX_GPU_DEVICE_UUID` selects the
exact Vulkan device using 32 hexadecimal digits; a failed selection is terminal.

The retained GTK scene provides plain Text/selection, single-line Textbox,
Button, Checkbox, Slider, Dropdown, RadioList, Frame, Scroller, and EventBlocker
nodes. Slider uses GtkScale, Dropdown uses GtkDropDown, and RadioList uses
grouped native GtkCheckButtons with radio accessibility roles. Programmatic
updates suppress input callbacks. Option replacement retains the requested
index; invalid or empty choices have no selected item and emit no sentinel.
Authored slider values clamp to the finite range; user changes apply the
positive, min-anchored step. Native dropdown popovers are separate surfaces,
so leaf masking does not establish popup composition or transformed placement.
EventBlocker's solid `background` paints within the retained native node's
transform, clipping and occlusion mask. Background alpha and node opacity each
apply once; an omitted background update retains the previous color. A fully
transparent background still absorbs pointer input. See
[modal underlays](../../../pax-docs/book/src/compositing-effects.md#scroller-islands-and-modal-underlays)
for ordering and the separate keyboard-focus responsibility.
Editable Text and
multiline Textbox use GTK text views with uniform native text styling. Changing
`editable` to false disables editing and the insertion cursor while retaining the
text view and its native selection behavior; a `selectable=false` override on
that view is not implemented.

Display-only Markdown receives normalized UTF-8 styled runs from the desktop
cartridge and uses a genuine GTK label with native selection and links. The
shared parser supports inline emphasis/code, headings/lists, and ExampleHost's
preformatted syntax-colored source; unsupported HTML/CSS and editable Markdown
fail explicitly. See the canonical [Markdown subset](../../../pax-docs/book/src/text-fonts-images.md#markdown-content).
Native links delegate to the host's existing virtual-route/OS navigation path.
The scene updates existing widgets so edits and selection survive geometry and rendering changes within
the same native parent. Moving selectable Text to another native parent retains
the widget but currently clears its selection; immediate and batched placement
have the same behavior. Native
masks use GSK paths and alpha masks; picking checks the same paths, so fully
opaque vector occlusion passes input through and fractional occlusion retains
native interaction. **The experimental v0 deliberately retains the existing
web/Apple alpha-mask approximation for fractional visual composition:** the
actual fixture's 50% yellow vector over a white Textbox produces RGB
`[251, 235, 201]` instead of `[247, 215, 148]` in both Debug and Release. The
vector is already blended in the GPU tile, then the inverse native alpha mask
attenuates its contribution again. Input coverage passes, but translucent vectors
over native content are not visually exact. Exact correction is deferred to
[PAX-1040](https://linear.app/paxdev/issue/PAX-1040/correct-translucency-edge-cases-in-compositor);
changing mask weights is not a correct general fix. GTK 4.22 and the experimental
colored foreground capture are not prerequisites for this v0 path.

Native siblings use their current `z_index`; GPU tiles paint
below native peers within each container, with masks revealing foreground
vectors. Creation-time native layer IDs are not a live stacking order. GPU tiles belong to their logical layer/scroller. Other
native control families currently return an explicit unsupported error. GTK owns
IME composition. The October 16 T4/Xorg run qualified actual IBus/libpinyin
preedit, Chinese candidate commit and cancellation, plus native Tab/Space,
clipboard cut/paste, and external AT-SPI text/selection/button actions. This is
one input engine/toolkit configuration, not broad IME or screen-reader parity.
GTK 4.14's AT-SPI `Component.grabFocus` returned an error in that run; actual
keyboard focus traversal passed. Text queries used explicit Unicode character
bounds because `getText(0, -1)` returned an empty string for GTK entries.

The standalone canvas starts with keyboard focus; clicking rendered vector
content restores it after native editing. Global Pax keyboard events are sent
only while the canvas itself has focus. Native controls retain their own GTK
editing, IME and shortcut behavior. `KeyPress` follows an uncancelled `KeyDown`
only for printable characters or Enter without Control/Command. Physical-key
records preserve native repeat information and pair held keys with `KeyUp` when
focus moves to a native control, the window deactivates, or the app closes.
Synthetic focus-loss releases clear modifiers because the canvas no longer
knows their physical state. `check-native-keyboard.py` verifies this through
real X11 events and Rust fixture readouts, including printable cancellation;
these canvas events do not implement an IME.

Raw `@wheel` input is captured once at the scene before GTK's native descendants
handle it. It carries scene/window-content logical coordinates, current modifier
keys, and GTK's device deltas with down/right positive. Deltas retain their native
units; a wheel step is not a portable pixel distance, and the bridge does not
round smooth input or generate momentum events. `prevent_default()` suppresses
the corresponding GTK default action, including native scrolling. Uncancelled
events retain GTK's scrolling and nesting behavior. Actual native position
changes independently report `ScrollerPosition`; raw wheel input never fabricates
offsets or an additional `@scroll` notification.

System font families and bundled font files work through Pango/Fontconfig.
Authored pixel sizes become Pango point sizes at 72/96 of their value, preserving
logical pixel sizes at GTK's 96-DPI font baseline while allowing GTK's effective
font resolution to scale the glyphs. GTK handles display scaling separately;
the host does not multiply fonts by the monitor scale factor.
The public `GtkWidgetClass.system_setting_changed` hook first lets GTK refresh
the subtree's Pango contexts, then marks font measurements dirty for DPI, font
name, font configuration, or display changes. The next accepted native batch,
including an empty batch, coalesces pending changes and sends
`TextMeasurementInvalidated` with each automatic Text node's latest generation.
The shared engine rejects stale generations and requests new measurements.
Both explicitly fixed axes remain fixed. Only the font-DPI notification category
has direct qualification; other settings categories still need targeted checks.
Remote font or image URLs are currently unsupported. Packaged images are decoded
once on the GTK thread and uploaded through the cartridge image API; large-image
loading is not yet asynchronous. The actual fixture uses a bundled Manrope font
and a packaged image. These input uploads are separate from presentation, which
does not download rendered frames.

Plain Text keeps authored width and height constraints separate from resolved
GTK bounds. An omitted axis remains automatic across patches that omit unchanged
fields, so later content or width changes can grow or shrink its native allocation
along with the engine layout. Explicit dimensions, including zero, stay fixed.
Measurement requests carry generations; the host returns the resolved bounds
under that generation. Placement-only patches do not force another measurement.
The [text-layout acceptance runner](../../../tests/src/native-platform-probes/linux/README.md#native-text-measurement-qualification)
qualified content growth, wrapping, shrinkage, repeated Text nodes, autosized
parents, and live X11 font-DPI changes from 96 to 144 and back. Debug and baked
Release on GTK 4.14.5, plus the installed GTK 4.22.5 Flatpak Release package,
each passed 357 checks on October 20. The settings transition changed native
text before any test click or content/width update, in the same process with
unchanged display scale. Opt-in Pango diagnostics also verified automatic
text's glyph containment and absence of missing glyphs or ellipsis. The fixed
30px label deliberately retains its authored height and can overflow.
Fresh launches at 144-DPI font resolution passed another 222 checks in each
configuration.

These checks use the actual XSettings protocol with a temporary settings owner
on an otherwise unowned lab display. They do not qualify a GNOME/KDE settings
interface, Wayland settings portal, other scripts/fonts, or transformed glyph
clipping. A separate native C harness passed 125 checks at each of widget scale
factors 1 and 2, covering explicit zero/fractional constraints and deferred,
coalesced, generation-checked remeasurement.

Packaged fonts register in the owning scene's Fontconfig configuration before
new controls are parented. PangoFc cache invalidation makes a newly registered
face available to existing labels; the scene then requests generation-checked
remeasurement for automatic Text. Reusing an already registered face does not
clear the font cache. Registration retains the configuration and its files for
the process lifetime; this is not an application font-unload API.

Pointer input is observed once in GTK's capture phase without claiming native
gestures. This lets a `Link` around selectable native Text receive its engine
click while GTK retains text selection, editing and scrolling. Clicks require a
matching press and release within GTK's drag threshold; selecting text across a
Link or releasing a press that began outside the scene does not activate it.
Lost grabs, window deactivation and scene shutdown cancel pending clicks.
Selecting a large native label with the pointer preserves its enclosing scroll
positions. The host temporarily suspends the owning viewports' focus reveal
during that press, then restores their previous settings; keyboard navigation
still reveals an offscreen focused control. GTK continues to own selection.
The shared native
mouse interrupt path currently does not return pointer default-prevention
results, so pointer handlers cannot cancel those toolkit defaults.

## Scrolling and native routes

Scroller retains `GtkScrolledWindow`, its adjustments, native scrollbars, wheel
input, and kinetic movement. Authored snap positions are finite local pixels
after shared percentage resolution. The adapter sorts and deduplicates them,
clamps effective points to the current scrollable extent, and retains authored
points so a later extent increase can restore them. Empty lists restore free
scrolling; omitted lists leave the current policy intact.

Authored offsets wait for GTK to allocate the corresponding viewport and content
extent. The adapter keeps only the latest pending request per axis, applies it
against that allocation, and then acknowledges GTK's actual clamped result. This
preserves nonzero initial positions and simultaneous content-size/position
updates; an unallocated adjustment's zero range must not overwrite a bound
engine property. Identical echoes of native movement do not cancel momentum or
snapping. Requests outside the allocated range still clamp normally.

GTK exposes no public kinetic-completion signal. The alpha settling policy waits
150ms without native adjustment movement after held pointer, keyboard, touch,
and continuous-scroll input ends, then moves the same native adjustments to the
nearest anchor over 180ms. Small discrete wheel/key steps advance to the next
anchor in their direction. GTK's reduced-motion preference removes the animation.
New input interrupts settling; release without further movement still resolves
the interrupted position. Hidden widgets retain their offset and resume pending
settling when mapped. Pointer motion alone does not postpone settling. Position
interrupts report the actual native offset, and identical engine echoes do not
retarget the animation. This guarded quiescence policy is not a native GTK snap
API or proof of every physical touchpad's momentum behavior.

Relative `Navigate` destinations targeting the current view update a per-session
virtual location and emit `RouteChange` to the existing Router. This follows the
established native route contract: decode the whole path before splitting it,
preserve repeated query keys and literal `+`, and normalize away trailing path
slashes before the next relative resolution. Empty destinations do nothing.
Absolute public URIs use asynchronous `GtkUriLauncher`; launch failure is
nonfatal. Relative new-window destinations and the private `pax:` scheme are
not handed to the OS. This does not implement browser history or an in-app
browser.

## Frame ownership and limits

The Rust `pax-chassis-linux` backend copies each changed retained Pax output into
one of at most three export allocations for that surface/dimension pair. All
live and retiring generations share a 48-allocation, 256 MiB cap, including WGPU
retired allocations that have not run their destruction callbacks. A permanently
oversized frame is an error; temporary retention returns backpressure. A false
capacity check leaves existing presentation intact. The host retains one pending
engine tick and commits its native patches, layer ownership, tile removal, and
new textures together once every active surface generation is ready. It does not
advance further engine ticks while this commit is blocked.

The accepted commit places native controls and GPU tiles at most once after all its
patches, layer-owner changes, removals, and texture updates are staged. A frame
with several changed tiles therefore performs one sibling-order and transform
reconciliation. Empty native patches with unchanged layer owners and no tile
changes skip placement; text-metric invalidations still run. An owner change
moves retained tiles even when no new texture is published. No GTK main-context
dispatch occurs inside the batch. The
standalone apply/texture APIs still place immediately outside a batch; an apply error
abandons deferred placement without claiming to roll back property updates.
The focused [native commit probe](../../../tests/src/native-platform-probes/linux/README.md#native-scene-commit-batching)
checks ownership, ordering, reparenting, same-parent selection preservation,
and error boundaries.

Publication queues a Vulkan ownership release to the foreign queue and exports a
binary SYNC_FD. The GTK host imports that fence into EGL and queues a server wait,
then copies the source into a new immutable GL texture for GDK. Its exported EGL
completion SYNC_FD returns to the Vulkan pool. Reuse requires both producer and
consumer completion and queues a foreign ownership acquire before another copy.
Vulkan's already-signaled `-1` export is carried explicitly as
`producerComplete=true`, without constructing an invalid owned FD or queuing an
EGL wait for work that has completed. This follows the
[Vulkan SYNC_FD contract](https://docs.vulkan.org/refpages/latest/refpages/source/VkImportSemaphoreFdInfoKHR.html).
Tokens identify individual publications; duplicate, stale, and unknown releases
are rejected. No fence is inferred from a GDK destruction callback.

GDK owns each immutable output until its destruction callback. A UI-thread
retirement source also checks the GL copy fence without blocking before deleting
the GL object and releasing its logical quota. Live GDK references and pending
copy completions share another 48-image, 256 MiB cap.
The complete active surface set is limited to 24 images/128 MiB so the currently
displayed frame leaves room for its full replacement. Retention beyond the
current frame can still apply temporary backpressure. GL deletion preserves
already-submitted GSK readers according to the driver API. **This logical cap is
not a strict bound on physical driver storage retained for those readers.**
Immutable GL outputs are not reused as mutable buffers. Steady-state driver
residency must be measured separately.

Close stops new ticks and publication, continues release/poll/shutdown work, and
finishes only after Vulkan exports drain. A standalone window also waits for its
owned GL publications to retire. Device loss or missing reliable completion
proof fails closed and can retain resources; it does not claim device-reset
recovery. Mounting requires the application's GTK main thread, and host/cartridge
libraries remain process-pinned for asynchronous callbacks and Rust TLS. Dropping
a prepared, unmounted app pumps the owner-thread GTK context for acknowledged
shutdown, bounded to 15 seconds. A timeout retains ownership and prevents a new
init until cleanup finishes; the application must keep pumping that context.
Mounted/window close remains asynchronous, and the final closed notification is
delivered before a standalone event loop exits.

## Acceptance checks

Button uses native GTK CSS for authored background, hover/pressed background,
outline stroke, and corner radius. Textbox uses native background, stroke, radius,
and focus outline; reactive patches retain the original widgets and editing
state. Theme images and shadows are removed from these authored surfaces so they
do not tint the requested colors. GTK still owns focus, keyboard input, IME,
accessibility, selection, and control behavior.
Fully transparent uniform text uses a transparent native CSS foreground as the
fallback for Pango's zero-alpha sentinel. This changes glyph ink without fading
the widget or its native selection highlight. Returning to a visible fill or
removing the fill removes that override; deleting the node removes its provider.
Collapsed finite negative scroller-content extents become a legal empty native
box, no smaller than the nonnegative viewport, and recover on expansion.
Nonfinite or integer-overflowing scroller extents fail explicitly.

Dropdown arrows use the authored text foreground through native CSS, including
its alpha; labels retain their independent Pango foreground. Native popup
contents use the authored background behind their styled options. The focused
probe checks actual arrow and popup pixels; complete focus-state, popup masking,
and transformed-popup placement remain separate qualification boundaries.

Text, Button labels, single-line editors, and form labels apply Pango font family,
size, weight, slant, foreground alpha, and underline. Text and Button labels
support horizontal/vertical block alignment and independent multiline line
alignment. Single-line editors use native horizontal alignment. Multiline
Textbox and editable Text use uniform native `GtkTextTag` styling, including
paragraph justification; their tags survive content replacement and insertion
into an empty editor. Multiline vertical block alignment and single-line editor
vertical alignment are not implemented as authored overrides. Editable Markdown
remains unsupported. Display-only styled runs and links are covered by the separate
[native rich-text contract](../../../tests/src/native-platform-probes/linux/README.md#native-rich-text-contract).

The [native styling probe](../../../tests/src/native-platform-probes/linux/README.md#native-text-and-surface-styling)
checks actual normal/hover/active and reactive surface pixels, native text
attributes, edit retention, and font-DPI glyph measurements. The canonical
runner's font, mask, and lifetime assertions alone do not establish complete
appearance or text-style parity.

The backend's hardware tests explicitly require a compatible physical GPU:

```sh
CARGO_BUILD_JOBS=2 cargo test -p pax-chassis-linux --lib -- --include-ignored --test-threads=1
```

They exercise held leases, resize generations, global quotas, stale/duplicate
releases, an actually pending SYNC_FD, and close/drain. The delayed-fence case uses
a probe-only Vulkan host event; it is not an EGL composition qualification.

For the packaged fixture, run inside an authenticated NVIDIA Xorg session:

```sh
python3 tests/src/native-platform-probes/linux/check-native-app.py \
  examples/fixtures/native-desktop-smoke/.pax/build/debug/linux/native-desktop-smoke \
  --output /absolute/new-acceptance-directory --seconds 60
```

The runner uses actual X11 keyboard/mouse input and GTK-computed coordinates. It
checks visible scroller text and opaque/half-alpha image RGB against an initial
unscrolled capture, startup data,
Rust button/input handlers, retained selection, nested
scrolling, opaque/fractional mask input, actual Pango font resolution, resize,
allocation counters, export reuse, and close. Its explicit
one-shot screenshots are test-only GPU downloads. Production presentation has no
such readback. Passing the C build, Rust check, or earlier immutable platform
probe alone does not qualify this continuous host, its release cartridge, native
Wayland, embedding, IME, or device-loss recovery; record each actual run separately.

When `PAX_NATIVE_TEST_REPORT` is set, the report also includes bounded timing
histograms: host frame callback work, callback intervals, and GDK presentation
intervals when the window system supplies them. Percentiles are upper bounds
with 0.25 ms buckets (the final bucket reports the observed maximum). Callback
work includes engine/native patch/texture submission but does not wait for final
GPU presentation. A zero presentation sample count means unavailable, not zero
latency. Test screenshots and resize actions can introduce stalls; preserve those
events when interpreting results. Missing GDK history and unavailable presentation
timestamps are counted and break the presentation-interval sequence. No timing
bookkeeping runs without the test report opt-in.

The same report separates callback wall time into `callbackPhases`: engine tick
(including response decoding), message preparation, surface planning, import
capacity, render, import/release, native patches, tile commit/cleanup, and first
activation. These spans do not overlap, but small wrapper costs remain outside
them; backpressure can give phases different sample counts. `startupPhases`
records cartridge loading, engine initialization, GPU importer construction,
engine GPU initialization, and native scene mounting. These are selected startup
calls, not total launch-to-visible latency. Each distribution includes `firstMs`
so first-call costs remain distinguishable from steady-state means. Counts and
means are cumulative over the session; use differences of count × mean for a
specific measurement window.

`callbackThreadCpu` measures CPU consumed by the callback's thread, excluding
time waiting and work on other threads. Its count is zero if the platform clock
is unavailable. GTK layout/rendering after the callback and background workers
are outside this value. Compare with whole-process CPU and a separate sampling
profile; subtracting callback wall time from process CPU cannot attribute the
remaining work. These diagnostics do not measure GPU execution or physical
display latency.

Reports also expose cumulative `sceneReconciliations` and
`scenePlacementVisits` counters. These count placement passes and nodes visited,
not changed nodes, GPU work, or presented frames. The detailed report itself
walks native controls and text layouts, so account for its cost when comparing
process CPU with callback timings.

`check-native-inputs.py` separately requires a shared D-Bus session, IBus with the
`libpinyin` engine selected, `GTK_IM_MODULE=ibus`, `GTK_A11Y=atspi`, and Python
AT-SPI bindings. It drives real CJK preedit/commit/cancel, then uses an external
AT-SPI client to inspect Unicode text/selection and invoke the native button.
This is targeted protocol/input evidence, not complete screen-reader or IME parity.

`check-native-wayland.py` runs with `GDK_BACKEND=wayland` and no `DISPLAY`,
verifies the actual `GdkWaylandDisplay`, invokes real controls through AT-SPI,
checks continuous GPU export reuse and image RGB in a composed capture. It passed
on the October 16 Ubuntu 24.04/T4 guest with Weston 13's headless **GL** backend
(NVIDIA EGL/GLES, not Pixman), alongside the hardware DMA-BUF preflight. This
qualifies that native Wayland protocol and GPU-import path; physical KMS output,
Wayland keyboard/pointer/IME, other compositors, and presentation latency remain
unqualified. The Xorg IBus result above does not establish Wayland IME support.

The October 16 optimized Release package also passed the ordinary acceptance
runner from `/tmp` after relocation, while the original source/build tree was
hidden in a private mount namespace. Bundled Manrope and visible scroller/editor
pixels remained correct. That check makes no packaging claim for other machines
without the documented GTK/driver/runtime prerequisites.

### Installed Flatpak packages

The [opt-in Flatpak wrapper](../../../pax-docs/book/src/targets-build-deploy.md#optional-gtk-422-runtime-package)
packages an existing Linux Release directory without an SDK download. After
installing its artifact, the native acceptance runners can target the installed
application with `--flatpak-app` and `--flatpak-branch`:

```sh
python3 tests/src/native-platform-probes/linux/check-native-app.py \
  /absolute/package/native-desktop-smoke \
  --flatpak-app org.paxlang.NativeDesktopSmoke --flatpak-branch pilot \
  --output /absolute/new-flatpak-acceptance-directory --seconds 60
```

The executable argument identifies the packaged launcher; the test launches the
installed Flatpak instead of that host path. `check-native-keyboard.py`,
`check-native-inputs.py`, `check-native-wayland.py`, and
`check-native-text-layout.py` accept the same two Flatpak arguments; the text
runner uses the separate `native-text-layout` fixture. Preserve each runner's display, D-Bus and input-engine
prerequisites. X11 sandbox checks additionally need the host's public libXRes
library (`libxres1` on Ubuntu) to verify window ownership: the sandbox may omit
`_NET_WM_PID`. Accessibility ownership is checked against the launched Flatpak
instance's D-Bus proxy. These are test dependencies, not application permissions.
The runners grant only their new evidence directory read/write access; the
shipping wrapper has no host/home filesystem or unrestricted D-Bus access.

The October 21 host passed the navigation/default-font and guarded-snapping
runners directly in Debug and baked Release on GTK 4.14.5, and as installed
Release Flatpaks on GTK 4.22.5. An unchanged CLI-created default PaxLogo passed
its Slider, theme, replay, packaged-font and cleanup checks in both direct build
modes and the installed Flatpak. The same Release directory also passed after
relocation with the source repository and compiler target hidden in a private
mount namespace. The CLI builds used the current host sources without an SDK
override. These checks establish the tested T4/runtime paths, not universal Linux
distribution or graphics-driver compatibility.

The October 18 T4 run qualified the unchanged optimized Release package against
GNOME Platform 50 with GTK 4.22.5 and the matching NVIDIA 595.91.07 extension:
24 distinct ordinary checks, 14 canvas keyboard checks, 13 IBus/AT-SPI checks,
and 11 headless Weston GL/Wayland checks passed. Packaged font/image pixels,
real controls, GPU export reuse and resource retirement passed inside the
sandbox; source and compiler paths were inaccessible. This does not establish
physical Wayland output/input, other graphics drivers, general screen-reader
parity or file-chooser support. Run portal diagnostics in a normal user login
session: the lab's SSM impersonation session produced document-portal mount
warnings that did not reproduce in a transient user login session. The latter
verified the document mount and runtime launch, not a file-picker interaction.

The approximately 19 MB app used about 3.4 GB of allocated storage for the cold
Flatpak installation, including shared runtime/graphics extensions. Per-ref
logical sizes, particularly partially installed locale data, are not additive
physical storage or measured download totals. The wrapper records the runtime
commit used for packaging; later branch updates require recording and testing
the actual installed runtime again.

`--hold-ms 1000` adds a test-only slow GDK consumer by retaining genuine published
texture references for one second. It must cause bounded publication
backpressure and resume after those references retire. This does not fabricate a
GPU fence or prove final driver residency; run it separately from the baseline.
The corresponding `PAX_NATIVE_TEST_RETAIN_MS` hook is accepted only alongside the
explicit diagnostic report and is bounded to five seconds.

The separate Debug reload check launches the real CLI/design server, edits one
fixture label, verifies the same native process updates, restores the original
bytes, verifies restoration, and closes the app:

```sh
python3 tests/src/native-platform-probes/linux/check-native-reload.py \
  /absolute/pax-cli examples/fixtures/native-desktop-smoke \
  --output /absolute/new-reload-directory
```

Use the same display, exact GPU UUID and optional `PAX_HOST_SDK_DIR` environment
as the ordinary native run. Release packages disable template live reload.
