# Android native acceptance probe

Build and install the fixture from the repository root using the Android setup
in the book. The probe uses actual Android widgets, not injected Pax events:

```sh
python3 tests/src/native-platform-probes/android/build-ui-probe.py --output target/android-ui-probe --install
pax-cli run --target android --path examples/fixtures/native-android-smoke --hot-reload=off
python3 tests/src/native-platform-probes/android/check-native-app.py --output /tmp/pax-android-debug
pax-cli run --release --target android --path examples/fixtures/native-android-smoke
python3 tests/src/native-platform-probes/android/check-native-app.py --output /tmp/pax-android-release
```

Set `ANDROID_HOME` and, when needed, `ANDROID_SERIAL`. `--adb` accepts an explicit
adb path. The probe saves screenshots, lifecycle/presentation logs, and a JSON
result. It restores system rotation settings even if a rotation check fails.
It restarts only `dev.pax.native_android_smoke` and finishes by leaving its default
screen open. Keep the emulator unlocked and in portrait before starting.

Installing the helper also runs host regressions for query-only and empty route
references, system font aliases,
weight/italic selection, locale-independent name matching, exact mixed-case
families, native mask paths, and pending state coalescing. On API 31 and newer,
the report names the font files Android actually uses for the Paxflix title
before and after family resolution; it does not assume all devices ship the
same fonts.

Checks cover explicit initialization; native button, checkbox, textbox, slider,
dropdown and radio-list
bindings, rapid typing, and application text transformations; IME visibility; background/resume and Activity recreation; transformed
native input bounds; opaque mask hit rejection; nested vertical scrolling;
horizontal scrolling with vertical parent handoff;
and normal Activity teardown. The test helper injects real keyboard events with
fresh timestamps and checks Android's injection result. The shell's `input text`
can silently lose the end of a long sequence when its precomputed events expire
under device load. Text injection is restricted to the fixture's focused native
editor; it does not assign its text or call Pax handlers directly. This
does not qualify multilingual IME composition, accessibility, all device GPUs,
all diagonal/edge gesture cases, process-death persistence, or store distribution.

Add `--remote-assets` to exercise two HTTPS PNG loads and a Google Fonts CSS
source, checking upload completion, intrinsic text size, and zero asset failures.
This optional screen requires Internet access; its Rust logos are downloaded
from rust-lang.org at runtime. The default controls suite stays offline.

For a specific fixture screen, start the Activity with `--es pax.startup composition` or `--es pax.startup scroll`. `--es pax.startup fail-initializer`
exercises the initializer failure UI. `--ez pax.diagnostics true` writes bounded
presentation counters to the `Pax` log tag every 120 display ticks. Compare
native pending bytes, GPU frame count, window count and retired surfaces during
scrolling and after settling; emulator timing is not a device performance claim.

## Orthographic spatial-workbench smoke test

Build and install `examples/fixtures/spatial-workbench` for Android, then run:

```sh
python3 tests/src/native-platform-probes/android/check-native-spatial.py --profile debug --output /tmp/pax-spatial-debug
```

Use a viewport at least 1000 logical pixels wide so the fixture's controls are
visible together. Set `ANDROID_HOME`, `JAVA_HOME`, and `ANDROID_SERIAL` as above.
The probe uses the fixture's development signing key and injects native touch and
keyboard events. It checks retained editor identity/text, transformed button
input, orthographic zoom, the unsupported-perspective fallback, and clipping
toggle/recovery. It leaves the fixture open. Run separately with `--profile release`
after installing the Release development APK. The screenshots require visual
review; this does not qualify perspective, exact edge-on poses, pixel-perfect
depth compositing, or physical-device performance. Add `--edge-on` to request an
additional exact 90-degree native tilt and recovery check; review its separate
capture and require `edgeOnQualified=true` in the instrumentation result.

## Native/canvas scroll alignment

After building and installing the development-signed fixture, run:

```sh
python3 tests/src/native-platform-probes/android/check-scroll-alignment.py --profile debug --output /tmp/pax-scroll-debug
python3 tests/src/native-platform-probes/android/check-scroll-alignment.py --profile debug --nested --output /tmp/pax-scroll-nested-debug
```

Use `--profile release` after installing its Release development APK. This
requires `ANDROID_HOME` and `JAVA_HOME`; keep the device unlocked in portrait.
The separate test-only instrumentation targets the fixture and uses its local
development key. It never runs inside a production app.

The `scroll-alignment` startup screen pairs native green Buttons with GPU
magenta Rectangles at the same content coordinates. The probe checks that tiles
are children of the native scrolling content, removes only Pax's frame callback,
and injects real Android drags and reversals. The pixels must stay aligned while
Pax produces no frames. It then resumes Pax and scrolls more than 3,000 logical
pixels across recycled tile windows. A separate thread captures composited pixels
during the gestures, allowing at most two physical pixels of edge difference for
rasterization. Only complete marker bands are compared: a clipped one-pixel
sliver can fall below the color threshold on one renderer. Instrumentation
results and PNG evidence are saved in the output
directory. `--nested` places the markers' scroller inside a scaled Frame and an
outer Scroller. This checks translation, tile reuse, and that nested ownership;
it does not qualify every transform, scroll-driven animation, or device compositor.

For the installed, development-signed website, capture the main page's fast
flings and reversals with:

```sh
python3 tests/src/native-platform-probes/android/check-scroll-alignment.py --website --profile release --output /tmp/pax-website-scroll
```

This uses asynchronous Android touch injection so a busy UI thread does not
slow the intended fling into a sequence of gentle drags. It also scrolls with
Pax's frame callback stopped. The output contains composited screenshots and a
per-draw trace of scroll position, tile placement, native text bounds, texture
updates, and pending publication. It verifies capture and page traversal;
**visual review is required** to judge artwork/native alignment. In particular,
inspect tile reuse during reversal back to the hero: artwork from a lower
section must not appear behind the hero text. A fast gesture can also outrun
prepared tiles; report missing artwork separately from displaced artwork.
Run one probe at a time and avoid builds during a capture used for timing.

## Textbox application transformations

After installing the fixture, run:

```sh
python3 tests/src/native-platform-probes/android/check-native-input.py --profile debug --output /tmp/pax-native-input-debug
```

Repeat with the Release APK and `--profile release`.
The probe keeps one real Android InputConnection across Rust uppercase updates,
checks that the Editable buffer identity and selection survive, then replaces a
selected range. It also keeps an editable `Text` connection across an application
patch and verifies that continued input reaches the Rust-bound property.
The broader controls probe also exercises injected key events;
`check-ime.py` covers composing text and Unicode through a test keyboard.

## Native images

The `native-images` fixture mode exercises real `NativeImage` patches, shared
bitmap ownership, and asynchronous replacement. After building and installing
the fixture in each profile, run:

```sh
python3 tests/src/native-platform-probes/android/check-native-images.py --profile debug --output /tmp/pax-native-images-debug
```

Repeat with `--profile release` for the baked cartridge. The probe compares
Android-rendered contain/cover/stretch pixels and final composed screen pixels,
changes the bound URL, cancels an in-flight request before remounting the same
source, and checks that deleting
the last native node releases its decoded bitmap ownership. It retains screenshots
of the fit modes and a transformed, clipped image. Physical-device and remote
decoder coverage remain separate qualification work.

## Website acceptance and profiling

The default `--paxflix-presentation overlay` matches this desktop integration
branch. After integrating the routed website from main, use
`--paxflix-presentation routed`; both modes check the actual return target,
Android Back, and state retention through Activity recreation. The mode is
recorded with the result.

The representative application is `pax-website`, including Paxflix,
Calculator, the feature gallery, and source drawers. Build each profile before
running the same device probe:

```sh
python3 tests/src/native-platform-probes/android/build-ui-probe.py --output target/android-ui-probe --install
pax-cli run --target android --path pax-website --hot-reload=off
python3 tests/src/native-platform-probes/android/check-website.py --output /tmp/pax-website-debug
pax-cli run --release --target android --path pax-website
python3 tests/src/native-platform-probes/android/check-website.py --output /tmp/pax-website-release
```

The probe records screenshots, the last accessibility hierarchy, process memory,
Activity-to-first-Pax-frame time, and presentation counters, including evidence
when a check fails. This timer includes host library loading and initialization,
but excludes process creation before `Activity.onCreate`.
The result's `inspection` maxima separate accessibility-root acquisition and
tree traversal from app frame timings; UI inspection can itself add load.
It checks intrinsic text sizing, source rendering, the Paxflix return target,
repeated open/Back cycles, demo retention through background/resume and
recreation, traversal through the gallery, and
`7 + 8 = 15` using the embedded Calculator's touch keys. This does not qualify
every gallery interaction or Calculator operation. `--cycles` accepts 1–20 Paxflix open/return cycles;
`--lifecycle-cycles` accepts 1–20 background/resume and portrait/landscape round trips
(default 1). Each lifecycle cycle checks process and initializer continuity, and
the result labels each memory sample with its checkpoint and elapsed time. For
an extended Release stability pass, use `--cycles 20 --lifecycle-cycles 10`.
Review the memory series and presentation counters alongside crashes and visual
evidence; completing these loops is not physical-device performance qualification.
Only the website
package is restarted. Internet access is needed for remote assets and fonts.
The test-only `dev.pax.android_probe` helper uses Android Instrumentation's
`UiAutomation` to read the real accessibility hierarchy while animations run;
the shell `uiautomator dump` command waits for idle and cannot reliably inspect
these continuously animated sections. The helper has no launcher or user data.
Its installer replaces only that helper if a cleaned build changed its test key.
Installation also runs host-state regressions against Android's JSON classes:
pending updates stay bounded across stalled frames, preserve node lifetimes and
navigation order, and respect newer native edits and later application changes.
The same instrumentation checks native mask-path commands, numeric syntax, and
rejection of malformed paths using Android's actual graphics classes.

With the helper and controls fixture installed, run
`python3 tests/src/native-platform-probes/android/check-rich-text.py --output /tmp/pax-rich-text`.
This inspects native spans and UTF-16 ranges, highlighted source colors,
no-wrap newlines, formatting-only updates, internal Markdown links and Back,
native selection, and editable Text binding. Run it for both Debug and Release.
The website probe also requires decoded source text, multiple token colors,
and inheritance of its authored code font; merely hiding markup is insufficient.
Review its screenshot for the loaded font's appearance.

The rich-text fixture deliberately lists `/rich-destination` in
`[package.metadata.pax.web].server_owned_prefixes`. Its Markdown link and Android
Back must still select and leave the ordinary in-app Router branch. Three
round trips replace nested native containers with canvas content, checking that
reused tiles keep publishing after the old route is removed. The package probe
also rejects web server-handoff policy in the Android asset configuration.

On an emulator, qualify the real input-connection path separately:

```sh
python3 tests/src/native-platform-probes/android/build-ui-probe.py --output target/android-ime-probe --ime --install
python3 tests/src/native-platform-probes/android/check-ime.py --output /tmp/pax-android-ime
```

The test temporarily selects its own Android InputMethodService, exercises
composing replacements, commits, combining accents, Arabic text, and emoji
deletion in Textbox and editable Text, and restores the previous keyboard in
`finally`. It refuses physical devices. Its shell-controlled service only accepts
the fixture's input connection and requires Android's DUMP permission for test
commands. It has no network or storage permission. This verifies host composition
mechanics; it does not replace testing shipping keyboards, languages, or TalkBack.

To test the installed Gboard keyboard itself, select Gboard's English QWERTY
layout with the normal on-screen keys visible, turn off TalkBack, and run on the
emulator with the UI helper installed:

```sh
python3 tests/src/native-platform-probes/android/check-gboard.py --adb "$ANDROID_HOME/platform-tools/adb" --profile debug --output /tmp/pax-gboard-debug
```

Repeat with `--profile release` after installing the Release fixture. The probe
taps keys and a suggestion in Gboard's live UI, inserts and deletes an emoji,
and checks the Rust binding in Textbox and editable Text. It also checks an
application uppercase transformation and that editors advertise editability
and the editor role before focus. The initial value is cleared with hardware key events; the tested
input comes from visible software-keyboard keys. Gboard must already be
installed and selected; this script neither downloads nor selects it. Its
English labels and suggestion expectations are specific to that configuration.
If Gboard opens its stylus toolbar instead, disable its “Use stylus to write in
text fields” preference for this run and restore it afterward. Dismissing the
handwriting tutorial alone does not necessarily leave handwriting mode.
The output records the installed keyboard version, screenshots, and hierarchies.
This does not qualify other keyboards, dictation, handwriting, or language layouts.

For TalkBack inspection, pass `-e preserve_services true` to the UI helper:

```sh
adb shell am instrument -w -e preserve_services true -e all_windows true dev.pax.android_probe/dev.pax.android_probe.Probe
```

Ordinary `UiAutomation` suppresses other accessibility services. The preservation
option keeps TalkBack running; `all_windows` also captures the keyboard and
accessibility overlays. Window discovery is asynchronous, so retry snapshots
while waiting for a newly opened keyboard. Nodes include accessibility focus,
editability, hints, state descriptions, and native action IDs. A hierarchy alone
does not prove screen-reader usability: exercise touch exploration, next/previous
navigation, activation, scrolling, and focus after opening/closing overlays with
TalkBack active, and review its spoken feedback.

Use gestures delivered through the emulator's touch surface or its authenticated
control API. In API 36 testing, shell `input swipe` did not advance TalkBack focus;
the emulator's `sendTouch` API did. Check that focus actually moves before counting
a gesture test. Preserve and restore the original accessibility settings and any
TalkBack diagnostic preferences. Native controls provide a foundation, but the
website still needs work on reading order, control names/roles, and modal focus
containment and return. Emulator checks do not replace physical-device qualification.

To qualify the live development connection, leave a Debug website run open with
the default template reload policy, then run the following in a second terminal:

```sh
python3 tests/src/native-platform-probes/android/check-development.py --output /tmp/pax-android-development
```

This checks session discovery, tree/selector/ray-cast inspection, window captures,
a screenshot sequence, and the capture duration limit. It temporarily changes a
sentence in `pax-website/src/hero_section.pax`, checks the actual native text and
unchanged Android process, then restores the source. It saves the original in
the output directory as well. Do not edit that component or run another device
probe during this check. Capture images still need visual review for native/GPU
composition. `--cli`, `--project`, and `--adb` select explicit tool/project paths.

The host logs `[p50, p95, p99, maximum, count]` arrays for Pax callback time and
Android window frame time. Timings use the most recent 240 samples; `count` is
the total collected by each recorder. Activity frame recorders reset on
recreation; operation recorders follow the retained session.
The `operationMs` object provides the same bounded summaries for each host/cartridge
operation, including native work and JSON encoding/decoding. These separate
rendering, runtime ticks, registry updates, event dispatch, and rich-text parsing.
`operationMaxReplyBytes` records each operation's largest native reply during the
session, so a large bridge payload can be distinguished from expensive rendering.
GPU `nominalColorBytes` is one RGBA plane per live backing: it excludes swapchain
copies, renderer intermediates, images, and driver overhead. Use `dumpsys meminfo`
for process PSS alongside these counters. Trace sections `Pax.tick`,
`Pax.surfacePlan`, `Pax.registry`, `Pax.render`, `Pax.poll`, `Pax.event`, and `Pax.richText` separate
host operations in Android system traces.

`--qualify-performance` is an opt-in engineering gate for a physical device and
rejects emulators. Initial targets are first frame within 3 seconds of
`Activity.onCreate`, peak sampled
PSS at most 512 MiB, at most 32 MiB growth across repeated demo visits, Pax callback
p95 at most 8 ms, and window p95 at most 16.7 ms for a 60 Hz device. These targets
have not yet been qualified on physical hardware. Run Release, record device,
OS, thermal state, refresh rate, cache/network conditions, and use repeat runs;
one passing sample is insufficient to declare alpha. Higher refresh rates need
a correspondingly shorter frame budget.

`.github/workflows/android.yml` cross-builds the controls fixture and website in
Debug and Release. It inspects each APK's SDK levels, debuggable flag, Activity,
asset inventory and ARM64 libraries, including every ELF load segment's 16 KiB
alignment, and verifies Debug signing. Run the same package check locally with:

```sh
python3 tests/src/native-platform-probes/android/check-package.py pax-website/.pax/build/android/release/app-unsigned.apk --profile release
```

Override `--min-sdk`, `--target-sdk`, or `--build-tools` for deliberate metadata
changes. The check expects the packager's compressed/extracted native libraries;
it does not validate App Bundles or replace installation on a 16 KiB device.
Its optional manual device job requires a dedicated runner
labelled `pax-android`, with JDK 17+, Python, Rust's Android target, SDK 36/build
tools 36.0.0, NDK r28+, and an unlocked ARM64 Vulkan device already selected by
`ANDROID_SERIAL`. Supply `ANDROID_HOME`, `ANDROID_NDK_HOME`, and `JAVA_HOME` in
that runner's environment. The job builds and installs the checked-out sources
before running both profiles' controls/website probes and the Debug development
probe; it does not provision a device or publish an app.
It also saves both profiles' website fling captures, which require visual review
even when the automated checks pass.
Use a persistent checkout/development keystore so subsequent installs have the
same signer. The device job disables checkout cleanup to retain these ignored
keys; the hosted package jobs retain checkout's default clean policy. A signature
mismatch requires the runner operator to remove its
old test packages deliberately. Physical-device performance qualification is a
separate opt-in run, not implied by package CI or emulator acceptance.

The independent affine decomposition test needs only a JDK:

```sh
javac -d /tmp/pax-android-affine pax-host/native/android/java/dev/pax/Affine.java pax-host/native/android/tests/AffineTest.java
java -cp /tmp/pax-android-affine dev.pax.AffineTest
```

Package checks also verify that every selected `nativeExports` symbol exists in
the cartridge ELF dynamic symbol table and that generic program exports are absent.
Rebuild the cartridge and Android host together when updating Pax.
