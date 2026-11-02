# Linux native composition probes

## Website application workload

`pax-website` is the complete-application desktop robustness workload. On a
qualified GPU-backed X11 session, build the unchanged application and run:

```sh
pax-cli build --path pax-website --target linux --release
python3 tests/src/native-platform-probes/linux/check-native-website.py \
  pax-website/.pax/build/release/linux/pax-website \
  --seconds 30 --output /tmp/pax-website-acceptance
```

For an installed package using the optional GTK 4.22 runtime, add
`--flatpak-app org.paxlang.WebsiteAcceptance --flatpak-branch pilot`. The positional
executable identifies the packaged launcher; process and loaded-library hashes
are checked through its mount namespace, and XRes verifies X11 window ownership.
The pointer route includes the website's full-width horizontal gallery. GTK
4.14.5 consumes vertical wheel input there even when the vertical axis has no
travel; GTK 4.22's controller propagates that input to the enclosing scroller.
Preserve that failure when testing the older runtime instead of routing input
around the gallery.

Keep the ordinary host hardware/device environment. `--startup-only` records the
launch boundary and requires eight seconds of continued operation after the first
committed-frame report, without scrolling. `--open-source` additionally clicks the first
visible View Source control after traversal and requires actual multiline,
selectable native rich-text runs; merely opening its background does not pass.
It drags a native selection, copies it with Ctrl+C, then uses Ctrl+A/Ctrl+C to
compare the whole rendered document byte-for-byte as UTF-8. Install `xclip` on
the test host for these optional clipboard checks. The runner verifies X11
keyboard focus and clipboard ownership against the launched process before
reading, and records byte counts and hashes in `source-clipboard.json`.
It then wheels the code pane on both axes while the outer page stays still,
switches to a second visible source tab, and closes the drawer.
These interactions cover one drawer and two sources, not every source tab.
The source drawer needs a cartridge and host that both implement normalized
rich text.
`startup.json` records a bracketed monotonic/wall-clock anchor for correlating
relative phase samples with separate process and renderer diagnostics. Older
receipts without this anchor cannot establish an exact common start time.
`--screenshots` opts into phase-boundary captures. Leave it off for memory
comparisons: although captures occur outside timing intervals, their allocations
can affect later RSS high-water marks.
`--seconds` accepts 10–300 seconds for the stationary observation and each
traversal's maximum duration. `--stationary-seconds 1200` overrides only the
stationary phase for a longer bounded soak (10–3900 seconds); traversal timeouts
remain unchanged. Set `PAX_GPU_MEMORY_DIAGNOSTICS=1` for bounded
renderer/cache summaries; the Flatpak runner forwards this explicit diagnostic
setting. Diagnostic runs should be distinguished from ordinary timing runs.
Use `--stationary-only --stationary-seconds 3900 --post-eviction-seconds 900`
with that environment variable to observe the hero for up to 65 minutes, ending
15 minutes after the first logged resource-cache eviction if it occurs earlier.
This mode skips traversal and interaction checks and rejects screenshots. The
receipt records the eviction line, observation time, and whether the full
post-eviction interval completed; a capped run without that interval does not
establish behavior after eviction. Pair it with process-memory sampling and
avoid concurrent builds or guest input.
`PAX_RENDER_TIMINGS=1` is also forwarded to the installed app. Its `layer_init`,
`prepare`, `encode`, and `submit` spans help attribute CPU wall time during
startup; nested spans overlap and must not be summed as independent costs.
They are not GPU timestamps or presentation measurements.
When using an external CPU sampler, verify Rust caller chains and ELF module
bias before relying on inclusive attribution. The tested perf 7.0.14/libdw
combination reported this cartridge's module bias one page too high and selected
incorrect unwind rules. Missing callers in that report do not establish a Pax
stack or allocation defect. The separate host phase timings remain usable;
keep raw sampling evidence and validate any corrected offline report separately.
The runner uses the same opt-in host diagnostics as the focused fixtures, actual
X11 wheel input, and isolated output directories. It records executable/loaded
library hashes, native state and process CPU/RSS. Six GiB RSS is a probe stop
safeguard, not the product's acceptance budget.
Before each input batch, it queries the X server's actual window under the
pointer and requires that window to belong to the website process. This avoids
relying on window-manager client discovery in the headless Xorg session;
`pointer-owner.json` preserves the last ownership observation, including failures.

Startup is measured from process launch to the first observed committed-frame
report and mapped window. Reports arrive every 250 ms and are polled every
100 ms; this is an upper-bound observation, not GPU presentation time, cold-disk
startup or full-site readiness. Compilation is measured separately. Each timed
phase excludes screenshots. Phase callback means use cumulative counter
differences; percentile histograms remain startup-inclusive. Neither callback
throughput nor DCV streaming establishes physical-display FPS.

The probe separates stationary hero animation, first down/up traversal and repeat
down/up traversal. It requires actual outer scroll-offset changes and settled
top/bottom endpoints within the requested time budget; it does not
mutate engine scroll positions. Preserve first-visit image/row allocations when
comparing warmed runs. `--traversals 1` scopes a diagnostic to the first down/up
pass; the default two passes are needed to compare repetition and reentry.
The default traversal probe does not qualify nested scrolling or selection/copy;
`--open-source` adds the specific drawer interactions above. Opt into the remaining
application checks separately, for example:

```sh
python3 tests/src/native-platform-probes/linux/check-native-website.py \
  pax-website/.pax/build/release/linux/pax-website \
  --flatpak-app org.paxlang.WebsiteAcceptance --flatpak-branch pilot \
  --interactions-only --interaction-cases calculator paxflix gallery resize path-studies source-tabs route \
  --screenshots --output /tmp/pax-website-interactions
```

`--interactions-only` preserves startup, process identity, ownership and clean
shutdown checks while skipping the stationary and traversal timing phases. Omit
it to run the selected interactions after those phases. Cases can be run
individually; `website-interactions.json` records each assertion and the runner
retains the state at a failure. The helper's source hash is recorded alongside
the main runner's hash in `checks.json`.

- `calculator` clicks the embedded calculator's actual keys, checks `7+8=15`,
  checks that its centered initial graph resolves the default curve and responds
  to native wheel movement while its parent remains stationary, zooms while
  preserving the world center, and drags the plot on both axes while its parent
  remains stationary, then switches back to calculation mode. These are authored
  Pax controls with native text labels, not GTK Button widgets.
- `paxflix` opens the full-screen demo, opens movie details, checks playback-demo
  feedback, scrolls its catalog and horizontal shelf independently, then closes
  the demo and checks that the website's scroll position is preserved. Playback
  is the example's local feedback action; no media service is contacted.
- `gallery` checks automatic motion, hover pause/resume, horizontal wheel input
  and vertical propagation to the enclosing website.
- `resize` changes the actual X11 window from 640×900 back to 1280×960 and checks
  responsive navigation, native scroller bounds and continued wheel input.
- `path-studies` visits both authored path demos, pauses and resumes playback,
  and checks the bound native slider value. It focuses each genuine GTK slider
  and uses Home/End to exercise the Rust scrub handler, requiring playback to
  pause at the chosen endpoint and resume when the authored toggle is clicked.
- `source-tabs` opens all six section drawers, reaches every reported tab with
  native horizontal wheel input, and copies each complete rendered source as
  exact UTF-8. It requires `xclip`; receipts in `website-source-tabs.json` identify
  each source, its byte count and hash. The application owns source selection;
  this probe does not set engine properties or scroll offsets.
- `route` clicks the footer's `PAX NOTES →` link, checks the site's authored
  `/blog` fallback, then uses `Return home →` to remount a scrollable home page.
  This native site defines home and a default route; the fallback itself is
  expected. Outgoing native nodes must retire after authored exit timelines;
  the probe records their IDs, bounds and frame progression while waiting.
  No external browser is opened by this case.

To check authored source content independently of the native copy operation,
generate the catalogue with
[`make-website-source-manifest.py`](../windows/make-website-source-manifest.py)
from the exact source snapshot used for the package. Compare each drawer index,
tab label and UTF-8 hash in `website-source-tabs.json` with that catalogue's
section manifest. Repeated sources must match in each authored drawer. The
Release CLI package qualified on 2026-10-25 passed all 106 assertions in the
calculator, Paxflix, gallery, resize, source-tab and route matrix; all 72 copied
tabs matched the frozen authored sources in their six drawers. A separate
`path-studies` pass qualified both demos with 15 assertions. These are named
matrix results; `fullWebsiteAcceptance` remains false. The website build used a
reviewed host SDK override. The native-wheel qualification below separately
checks the automatic embedded SDK build path.

These cases do not qualify touch, physical-display frame rate, broad accessibility,
external navigation or every capability illustrated by gallery cards. The website's
[acceptance scope](../../../../pax-website/README.md#native-desktop-robustness-workload)
lists those additional checks; focused IME, controls and snapping fixtures remain
part of desktop qualification.

This directory includes both the original isolated probes and acceptance runners
for the generated Pax Linux application. For the implemented host, its current
limits, and the `check-native-*.py` runners, start with the
[Linux host guide](../../../../pax-host/native/linux/README.md).

The ordinary, keyboard, IBus/AT-SPI and Wayland application runners also accept
`--flatpak-app <installed-app-id> --flatpak-branch pilot`. See the host guide's
[installed Flatpak checks](../../../../pax-host/native/linux/README.md#installed-flatpak-packages)
for exact invocation, host libXRes dependency, process-identity checks and the
limited evidence-directory permission. Headless Wayland and protocol-level
accessibility results do not establish physical Wayland input, screen-reader
parity or file-picker support.

The isolated probes below exercise GTK native-widget composition and WGPU 28 image
interop. They are not a Pax Linux application target or a production frame loop.
They use real GTK controls and no browser or WebView.

The [EC2 GPU lab provisioning runbook](../../../../scripts/native-gpu-lab/README.md)
defines a pinned NVIDIA Ubuntu 24.04 AMI option and an optional Xorg image recipe.
The approved October 15 pilot exercised the vendor AMI on a Tesla T4. The
[guest setup script](../../../../scripts/native-gpu-lab/guest/setup-linux-pilot.sh)
preserves its driver and kernel. Because the pinned Ubuntu snapshot service was
unavailable, that pilot resolved official live repositories once and archived
the signed indexes, exact package transaction, and package hashes before install.
This establishes the recorded transaction, not the original snapshot recipe.

Dependencies: a C compiler, `pkg-config`, GTK 4.14 or newer development headers,
Vulkan headers/loader, libepoxy, JSON-GLib, Fontconfig, PangoFc (`pangoft2`),
Xvfb, xauth, xdotool, and Weston. The Rust export
probe uses the same WGPU major version as Pax and does not depend on gtk-rs.

```sh
cd tests/src/native-platform-probes/linux
bash build.sh
/tmp/pax-1038-linux-probes/vulkan-capabilities
bash check-x11.sh
bash check-wayland.sh
```

`build.sh` writes binaries outside the repository. Override `PAX_PROBE_OUTPUT`
to change that directory. The X11 check supplies actual mouse clicks at
GTK-computed window coordinates, types into a live Entry, and selects all text
with Ctrl+A before animation. Event counters distinguish the curved hole's Pax
scene click from the partially visible Entry's native click; the full selection
must survive animation and scrolling. The headless Wayland check changes text
and selection through GTK's API and does not claim external pointer/keyboard input.
Both check focus, transform-aware picking, native selectable text, and retained
state while native controls animate under rounded clipping, a curved path hole,
and a partial alpha mask. Picking and snapshots use the same general path.
An input-only Pax scene widget owns fully transparent holes while fractional
coverage stays interactive through the actual GTK control. This layer is
necessary because GTK picks descendants before checking a parent's `contains`
method; a custom parent hit region alone cannot clip an Entry's internal widgets.
A one-shot render samples actual GSK mask pixels to verify that coverage agrees
with picking. Independent horizontal and vertical offsets in nested native
scrollers are checked while entry state remains intact. These checks do not
validate IME composition, accessibility clients, or physical touch hardware.
The input-only layer proves hit ownership; runtime event forwarding, pointer
capture, and crossing/drag behavior are not implemented by this probe.

The default X11 check uses Xvfb for compatibility testing. To run the same actual
mouse/keyboard assertions in an existing authenticated NVIDIA Xorg session,
preserve its `DISPLAY` and `XAUTHORITY` and set
`PAX_PROBE_X11_USE_EXISTING_DISPLAY=1`. This mode never starts a replacement
display. Verify the actual GL/Vulkan devices and run the hardware preflight in
that session; selecting an existing display alone does not establish hardware
qualification. The Wayland script remains a headless compatibility test.

Build and enable external-image interop separately:

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/pax-1038-wgpu-target \
  cargo build --locked --manifest-path wgpu-export/Cargo.toml
export PAX_PROBE_GPU_LIBRARY=/tmp/pax-1038-wgpu-target/debug/libpax_linux_wgpu_export_probe.so
bash check-x11.sh
bash check-wayland.sh
```

Keep `wgpu-export/Cargo.lock` with the probe so this command uses its recorded
dependency versions. Rebuild the C probe and Rust library together when the FFI
changes.

The default transport requests exportable Vulkan DMA-BUF images and imports
them with `GdkDmabufTextureBuilder`. Missing export capabilities are errors; the
probe does not silently switch transports. The Rust producer also requires an
integrated or discrete Vulkan GPU by default. Set `PAX_PROBE_ALLOW_SOFTWARE=1`
explicitly for CPU, virtual, or unidentified adapter diagnostics; these runs
cannot establish physical hardware qualification. To select the exact adapter
from the capability report, set `PAX_PROBE_GPU_DEVICE_UUID` to its 32-digit
hexadecimal `device_uuid`. A missing or ineligible UUID is an error, even when
diagnostic adapters are allowed; another adapter is never substituted.

To exercise Vulkan opaque file
descriptors imported by `GL_EXT_memory_object_fd` and `GdkGLTextureBuilder`:

```sh
PAX_PROBE_GPU_TRANSPORT=opaque-fd bash check-x11.sh
PAX_PROBE_GPU_TRANSPORT=opaque-fd bash check-wayland.sh
```

For example, the existing llvmpipe VM additionally needs
`PAX_PROBE_ALLOW_SOFTWARE=1` on those commands. Neither that opt-in nor the
opaque-FD transport supplies portable external synchronization.

The optional bridge renders three color-and-clip vector scenes through `pax-gpu`
into owned offscreen targets, then copies each target once on the shared WGPU
queue into an exportable image. These immutable imports appear both as separate
tiles and underneath live transformed GTK controls in the same GSK composition.
One-shot diagnostic downloads verify every imported pixel against the clipped
Pax pattern. A second test composes all three retained textures and verifies
every output pixel after animation; correct individual downloads alone cannot
validate GPU composition. A varying-size allocation cycle checks release after
CPU downloads. This diagnostic readback is not part of presentation. The default
producer uses synchronous completion waits; the opt-in DMA-BUF synchronization
test below exercises a separate one-way handoff. Neither mode implements a
streaming pool or buffer reuse.
Opaque-FD imports check Vulkan and GL device/driver UUIDs before importing memory.
They still require an external-semaphore path before portability is established:
[`GL_EXT_semaphore`](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_external_objects.txt)
waits also establish external memory visibility and texture layout state. A CPU
queue wait and successful pixels on one driver do not prove that contract on
other drivers. In particular, an adapter without external-semaphore support can
only supply diagnostic compatibility evidence from this probe.
The Rust path dependency requires this probe to remain in the repository's
directory layout (or a checkout staged with the same layout).

The checks save PNG captures and logs in the output directory. Preserve adapter,
driver, renderer, display-backend and transport output when reporting results.
Vulkan or GL software adapters in a VM provide compatibility evidence for that
configuration, not hardware performance or DMA-BUF capability on other systems.

## Native rich-text contract

Build `native-rich-text.c` like `native-styling.c` and run it under a real GTK
display with a 60-second timeout. Repeat with `GDK_SCALE=2` and the optional
GTK 4.22 runtime when available. The test uses production scene code to check
UTF-8 run boundaries, rendered text, per-range colors/weight/slant, native
selection across style updates, link routing, plain/rich transitions, same-ID
recreation, cached widget properties, and live DPI line-spacing changes on a
fixed-size label. Its link signal checks native-to-host routing without opening
an external application; actual pointer/input acceptance remains separate.

The October 24 candidate passed all 39 checks on GTK 4.14.5 at widget scales
1 and 2, and on GTK 4.22.5 at scale 1. Adjacent authored links to the same URL
remain distinct native links, while formatting inside one link shares its
native link target. This result does not establish full website acceptance.

`native-text-pointer-focus.c` adds actual X11 input through `xdotool` (with
`timeout` also on `PATH`). Build it the same way and run with `GDK_BACKEND=x11`.
It checks first and repeated selection of an oversized label inside nested
scrollers, preservation of scroll offsets and existing focus-reveal settings,
and subsequent Tab navigation to an offscreen button. The October 24 fix passed
all 15 checks on GTK 4.14.5 at scales 1 and 2; the prior host reproduced the
initial focus jump and incorrect selection. Its later checks were affected by
that displaced geometry. This is mouse/keyboard evidence, not touchscreen
qualification. The installed GTK 4.22.5 website also passed selection, exact
native Ctrl+C and Ctrl+A/Ctrl+C readback, both code scroll axes with a stationary
outer page, a second source tab, and drawer close.

`pax-chassis-common/src/desktop/rich-text-cases.json` contains exact normalized
fixtures shared with the parser and Windows contract, including source markup
captured from the actual ExampleHost highlighter. Markdown syntax is tested in
Rust; the C host consumes the normalized document and owns native shaping.

## Hardware capability preflight

Run this separate check on the candidate machine's actual desktop session before
attempting composition qualification:

```sh
bash check-hardware-preflight.sh
python3 -I check-preflight-report.py --self-test
```

The runner only builds the C capability probe. It needs the C/GTK/Vulkan
development dependencies above, Python 3, and GNU `timeout`; it does not build
the Rust exporter or create an Xvfb/Weston session. It preserves the caller's
display and Vulkan ICD configuration. Each invocation creates a fresh
`preflight.*` directory under `PAX_PROBE_OUTPUT` (default
`/tmp/pax-1038-linux-probes`), prints that directory on stderr, and saves
`hardware-preflight.json` plus `hardware-preflight.log` there. Validated JSON is
also printed on stdout. Exit status `0` means `preflight_eligible`, `2` means the
required capabilities are unavailable, and `1` means an operational error,
invalid report, or process timeout. The 30-second timeout bounds the process;
this probe submits no GPU work. Compilation failures have no JSON report.

Schema version 1 records each Vulkan device's type, vendor/device IDs, driver
details, and 32-digit `device_uuid`/`driver_uuid`, plus GDK's advertised DMA-BUF
format/modifier pairs. CPU, virtual, and unidentified devices remain visible in
the inventory but cannot pass the physical GPU gate. An unavailable GDK display
still produces Vulkan/device/synchronization inventory and fails eligibility.

Eligibility requires an integrated or discrete GPU, an actual 64×64 DMA-BUF
image allocation and FD export with a DRM modifier advertised by GDK, and
separately queried binary-semaphore `SYNC_FD` export **and** import properties.
Images use `SAMPLED | TRANSFER_DST`; the report records actual modifiers and
plane layouts before closing the FDs and freeing allocations. It does not
write image pixels or import them into GDK. Extension availability alone never
establishes these capabilities. The bidirectional semaphore requirement is an
explicit baseline for future buffer reuse; import support is not inherently
required by an immutable one-way export. No semaphore is created, exported,
imported, or waited on by this preflight.

The separate `current_exporter_candidate` field records the existing Rust
exporter's narrower RGBA8 linear/modifier-zero allocation requirement. A device
may pass the modifier preflight while requiring exporter changes before it can
run the composition probe. GDK's matching format list does not establish that
GDK and Vulkan use the same physical device or driver.

Every `qualification` field remains false: WGPU/Pax rendering, GDK import,
same-device import, composition, synchronization, foreign queue ownership,
DMA-BUF reservation fences, and hardware performance need separate evidence.
`queue_family_foreign` is inventoried but is not itself an eligibility rule.
The default exporter releases to `VK_QUEUE_FAMILY_EXTERNAL`; the opt-in test
below uses `VK_QUEUE_FAMILY_FOREIGN_EXT` and reservation fences. The capability
report deliberately does not infer those test results. Neither preflight
success nor an immutable pixel test qualifies portable synchronization or
streaming reuse.

## NVIDIA Xorg pilot and explicit synchronization

The October 15, 2026 hardware run used Ubuntu 24.04.4, kernel
`6.17.0-1019-aws`, NVIDIA driver `595.91.07` (the AMI's open kernel module),
Tesla T4, GTK `4.14.5`, and an authenticated NVIDIA Xorg console. The pilot
requires **`GSK_RENDERER=ngl` on that GTK version**: legacy `gl` renders all
three externally sampled DMA-BUF tiles with the last tile's color. This fails
the combined-pixel assertion both with and without initial diagnostic downloads;
the source buffers themselves remain correct. The same test passes with `ngl`.
This is a renderer/version-specific finding, not a claim about every GTK release.
No GTK patch or CPU presentation fallback is installed.

After building both probe halves, use the actual display session and adapter
UUID from the preflight report:

```sh
export PAX_PROBE_X11_USE_EXISTING_DISPLAY=1
# Replace this pilot UUID with the intended device from your report.
export PAX_PROBE_GPU_DEVICE_UUID=c00113a21f03d821294fa1a4ba58d452
GSK_RENDERER=ngl PAX_PROBE_GPU_TRANSPORT=dmabuf-egl PAX_PROBE_DMABUF_SYNC=1 \
  PAX_PROBE_SKIP_INITIAL_DOWNLOAD=1 PAX_PROBE_SYNC_DELAY_TEST=1 \
  bash check-x11.sh
```

`PAX_PROBE_DMABUF_SYNC=1` enables an experimental immutable producer handoff.
The same Vulkan queue copies the Pax image, releases ownership to `FOREIGN_EXT`, and signals an
exportable binary semaphore. The host imports its `SYNC_FD` as a DMA-BUF WRITE
reservation fence before GDK publication. No producer CPU completion wait occurs
before publication. At GDK reference retirement the probe exports the current
reservation fences for WRITE access (which waits for readers too), waits with a
bounded diagnostic poll, then releases the Vulkan allocation and command
resources. Failures retain resources rather than freeing potentially live
memory. This blocking retirement is unsuitable for a production frame loop.
On the recorded NVIDIA driver, reservation-fence import **does not establish a
working consumer wait**: the forced gate returned a GPU result in about 81 ms
while that exact producer fence was still unsignaled. The import ioctl and GDK
constructor each took only tens of microseconds. Preserve this failing result;
omitting the gate or observing correct pixels cannot qualify synchronization.
The [kernel DMA-BUF synchronization contract](https://docs.kernel.org/driver-api/dma-buf.html#dma-buffer-ioctls)
and [Vulkan foreign queue ownership extension](https://docs.vulkan.org/refpages/latest/refpages/source/VK_EXT_queue_family_foreign.html)
define these distinct synchronization and ownership operations.

The separate opt-in `PAX_PROBE_GPU_TRANSPORT=dmabuf-egl` uses the same Vulkan
producer but imports its `SYNC_FD` into an EGL native-fence object and calls
[`eglWaitSyncKHR`](https://registry.khronos.org/EGL/extensions/KHR/EGL_KHR_wait_sync.txt)
on a private GDK GL context. This is a server/GPU wait. The DMA-BUF is imported
as an EGLImage/external sampler and copied on the GPU into an owned 2D texture;
`GdkGLTextureBuilder` receives a GL fence for that copy. NVIDIA advertises this
linear format as external-only, so this route adds **one GPU conversion copy**
to the existing Pax-to-export copy. No CPU pixels enter presentation. The forced
gate passed with a pending producer at import, a signaled producer after the GPU
result, and about 1.99 seconds spent waiting for the two-second gate. The native
X11 controls, composed pixels, and both twelve-image lifetime checks passed too.
This is evidence for this exact configuration and immutable path. GL retirement
still uses `glFinish`; it is not a nonblocking streaming implementation. There
is no automatic transport fallback. Without `dmabuf-egl`, the gated command is
the known failing reservation-fence reproducer.

The synchronization mode also composes twelve same-sized exports in batches,
retires their renderer caches and input references before downloading the GPU
output, checks every pixel, and checks releases/allocation counts. The optional
`PAX_PROBE_SYNC_DELAY_TEST=1` adds a single bounded, two-second GPU event gate to
an 81×49 test image. Its consumer test must observe a pending producer fence and
wait for the correct GPU output while retaining every input reference. This gate
is test machinery, not pacing or performance evidence. Keep the initial source
downloads disabled for this qualification; they would otherwise wait for producer
completion before presentation. Per-test output readbacks remain diagnostic.

A production reuse implementation still needs a bounded pool, nonblocking
retirement, consumer-fence import and Vulkan ownership reacquisition, a new
immutable GDK texture identity for each published generation, resize retirement,
and backpressure when consumers retain every slot. This probe allocates a new
image for every publication and cannot establish those behaviors. Native DRM
Wayland, portable cross-driver import, IME, accessibility, and sustained frame
performance remain unqualified by this Xorg pilot.

## Offscreen renderer checks and virtualization limits

The probe manifest also exposes the existing `pax-gpu` offscreen pixel test,
without duplicating its source. From the repository root:

```sh
WGPU_BACKEND=vulkan cargo test --locked \
    --manifest-path tests/src/native-platform-probes/linux/wgpu-export/Cargo.toml \
    --test offscreen -j 2 -- \
    offscreen_preserves_alpha_clips_clean_frames_and_resize_lifetimes \
    --ignored --exact --nocapture
```

Require one executed test, no ignored tests, and retain the adapter log. This
test intentionally reads pixels back for assertions; it does not test native
presentation or external-memory sharing. Select the intended Vulkan ICD when
testing a translated driver, so another installed driver cannot satisfy the run.

The current Parallels ARM64 guest exposes VirGL OpenGL 4.0 / OpenGL ES 3.0 when
the user can access `/dev/dri/renderD128`. Those limits do not satisfy WGPU's
storage-buffer requirements for the Pax renderer. Its Vulkan driver is llvmpipe;
the explicit software diagnostic remains useful for composition API checks.

A separate, isolated libkrun 1.19.6 / libkrunfw 5.6.2 / MoltenVK 1.4.2 experiment
enumerated `Virtio-GPU Venus (Apple M2 Max)` using Mesa 26.0.8 with a local
allocation-alignment adaptation. Unmodified Mesa failed to initialize because
the guest's 4 KiB allocations did not meet the host's 16 KiB mapping requirement.
The patched driver exposed no external-memory FD, DMA-BUF, semaphore FD, DRM
modifier, or foreign-queue extensions; the actual Pax offscreen test then
aborted before completing its pixel assertions. This is a capability finding,
not a validated Linux renderer or GTK presentation workaround. The experiment
does not alter the installed guest driver or become a Pax dependency. See
[PAX-1038](https://linear.app/paxdev/issue/PAX-1038) for the experimental evidence
and the [upstream alignment issue](https://github.com/libkrun/krunkit/issues/114).

## Native text measurement qualification

Build `examples/fixtures/native-text-layout` with `pax-cli build --path
examples/fixtures/native-text-layout --target linux`, then run the checker in the
same authenticated NVIDIA Xorg session and GPU environment used above:

```sh
dbus-run-session -- python3 tests/src/native-platform-probes/linux/check-native-text-layout.py \
  examples/fixtures/native-text-layout/.pax/build/debug/linux/native-text-layout \
  --output /tmp/pax-text-layout-check
```

The output directory must be new. The checker drives actual native buttons and
compares ten engine layout records with GTK Text allocations through four
short/long-content and wide/narrow states. It checks independent repeated Text
nodes, autosized Stacker and padded Frame propagation, fixed-height preservation,
and normal close with all owned GL publications retired. Evidence includes the
exact loaded host-library hash, snapshots, explicit one-shot screenshots, and a
`checks.json` completion flag; an interrupted run is not a pass.

On October 20, 2026, Debug and baked Release passed the 117 ordinary layout checks
on the retained Ubuntu/Tesla T4 host with GTK 4.14.5; the optional GNOME 50 Flatpak
package passed them with GTK 4.22.5. Running the identical Debug cartridge against
the preserved earlier host reproduced nine engine/native allocation mismatches
after content or width changes. The corrected host retains authored auto
constraints separately from resolved allocation sizes.

For glyph containment and real external XSettings font-resolution changes, add:

```sh
dbus-run-session -- python3 tests/src/native-platform-probes/linux/check-native-text-layout.py \
  examples/fixtures/native-text-layout/.pax/build/debug/linux/native-text-layout \
  --check-text-ink --live-font-dpi --output /tmp/pax-text-layout-live
```

`--live-font-dpi` starts at 96 DPI, changes to 144 DPI while the same application
is idle, and restores 96 DPI. The checker observes native allocation changes
before sending another input event or requesting an engine layout record. It
requires an unchanged application process and widget scale factor, correct auto
Text and parent geometry, contained Pango ink, and restoration of the initial
dimensions. Debug, baked Release, and the GTK 4.22 Flatpak each passed all 357
checks. The separate `--cold-font-dpi 144` option replaces `--live-font-dpi` to
test initial high-DPI layout; all three lanes passed 222 checks, and 99 comparison
checks established that their initial dimensions and auto Text ink matched the
corresponding live 144-DPI state. Font metrics differ between runtime versions,
so compare cold and live results within the same runtime.

These options use `xsettings_test.py` to publish actual XSettings on the test
display. It refuses an existing settings manager, relinquishes ownership if a
manager appears, and removes its temporary selection on close. Use an isolated
authenticated X11 test session, not a desktop with an active settings manager.
The runs restored the initially absent selection and left no Flatpak processes.
They qualify the X11 protocol and GTK effective font-resolution path, not a
particular desktop's accessibility settings UI or a Wayland portal path.

The separate `--wayland --portal-font-scale --check-text-ink` lane uses the
installed `xdg-desktop-portal` and GTK backend, with real GSettings changes in
an isolated D-Bus session. It requires the GNOME interface schema, dconf,
Python GObject bindings, an already running Wayland compositor, and the installed
optional Flatpak. Set `XDG_CONFIG_HOME` to a new directory beside the evidence
directory; the helper refuses the normal configuration directory. For example,
with the appropriate `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`, and GPU selection:

```sh
export XDG_CONFIG_HOME=/tmp/pax-portal-check/config
export XDG_CURRENT_DESKTOP=PaxPilot GDK_BACKEND=wayland GTK_USE_PORTAL=1
mkdir -p "$XDG_CONFIG_HOME/xdg-desktop-portal"
printf '[preferred]\ndefault=gtk\n' > "$XDG_CONFIG_HOME/xdg-desktop-portal/portals.conf"
unset DISPLAY
dbus-run-session -- sh -c '
  dbus-update-activation-environment WAYLAND_DISPLAY XDG_RUNTIME_DIR \
    XDG_CONFIG_HOME XDG_CURRENT_DESKTOP GDK_BACKEND GTK_USE_PORTAL
  python3 tests/src/native-platform-probes/linux/check-native-text-layout.py \
    examples/fixtures/native-text-layout/.pax/build/release/linux/native-text-layout \
    --flatpak-app org.paxlang.NativeTextLayout --wayland \
    --check-text-ink --portal-font-scale --output /tmp/pax-portal-check/evidence
'
```

On October 20, the GTK 4.22.5 package passed 358 checks on Weston 13's headless
GL backend. The helper records the actual portal providers and their process
identities, observes each published scale, and restores the scoped setting.
The checker uses the scene's actual surface origin rather than assuming X11
decoration coordinates. This qualifies the tested portal-to-application path,
not a running GNOME/KDE desktop, another portal provider, or physical display
scaling.

To check an already installed optional package, add
`--flatpak-app org.paxlang.NativeTextLayout` (branch `pilot` by default), and pass
the matching unpackaged executable as the positional argument. The checker
records actual executable, cartridge, and loaded host-library hashes through
the application namespace. Native input targets a visible window with the
exact process identity; Flatpak uses XRes to verify its host PID.

The deliberately fixed-height Text remains 30 pixels high. Its default
`clip=false` permits glyph overflow when the font grows, so the checker records
that overflow without treating it as an auto-layout failure. Containment checks
apply to the fixture's untransformed auto Text; they do not qualify transformed
text or arbitrary clipping masks.

The focused [`text-constraints.c`](text-constraints.c) harness includes its
standalone compile/run commands and additionally passed 125 assertions at
both `GDK_SCALE=1` and `GDK_SCALE=2`, including explicit zero-width constraints,
fixed dimensions, generation checks, coalesced/deferred invalidation, and no
callbacks after stop. Each run observed fourteen measurements and two
invalidations. This checks logical sizing against double scaling; it does not
qualify physical monitor changes. After the font-resolution change, the ordinary
`native-desktop-smoke` Release checker also passed 54 checks directly and 56
inside the optional Flatpak, covering native input, bundled fonts and images,
composition, bounded allocations, and normal GL retirement.

## Native controls and larger scenes

Build the [`native-controls`](../../../../examples/fixtures/native-controls/README.md)
fixture, then run `check-native-controls.py` in an authenticated X11 session with
AT-SPI enabled and a shared D-Bus session:

```sh
dbus-run-session -- python3 tests/src/native-platform-probes/linux/check-native-controls.py \
  examples/fixtures/native-controls/.pax/build/release/linux/native-controls \
  --output /tmp/pax-controls-check
```

The runner uses real keyboard, drag, and wheel input. It checks repeated Slider
bindings, programmatic range changes without input events, option replacement,
independent radio groups, conditional teardown, and standalone Scroller corner
pixels and input exclusion. It also opens ordinary and transformed dropdowns and
observes their choices through AT-SPI. Popup visibility and selection do not prove
exact popup placement, clipping, or stacking. The focused `native-controls.c`
harness additionally checks host range reconciliation, off-grid authored values,
native roles, callback guards, label typography, and focused-option retirement;
its header points to the shared standalone compile command in `text-constraints.c`.

For independent scrolling panes and dense vector content:

```sh
python3 tests/src/native-platform-probes/linux/check-native-scroll.py \
  examples/fixtures/rounded-scroller-tiles/.pax/build/release/linux/rounded-scroller-tiles \
  --title "Rounded Scroller Tiles" --scrollers 6 --seconds 20 \
  --output /tmp/pax-rounded-scroll-check
python3 tests/src/native-platform-probes/linux/check-native-scroll.py \
  examples/fixtures/scroll-cull-stress/.pax/build/release/linux/scroll-cull-stress \
  --title "Scroll Cull Stress" --scrollers 1 --seconds 20 \
  --output /tmp/pax-cull-scroll-check
python3 tests/src/native-platform-probes/linux/check-native-canonical.py \
  examples/fixtures/scroll-matrix/.pax/build/release/linux/scroll-matrix \
  --fixture scroll-matrix --output /tmp/pax-scroll-matrix-check
python3 tests/src/native-platform-probes/linux/check-native-canonical.py \
  examples/fixtures/neon-opacity/.pax/build/release/linux/neon-opacity \
  --fixture neon-opacity --output /tmp/pax-neon-opacity-check
```

The independent-pane checker waits for offsets to settle before comparing panes;
the matrix checker instead tests nested axis ownership. The canonical checks
require the intended Pango font families, genuine native forms, continued GPU
publication/reuse, and normal retirement. Captures and mask metadata are evidence
for inspection, not a complete animated-mask pixel oracle.

These are bounded diagnostics with reporting enabled. Host work and callback
histograms include startup; do not subtract percentile values to manufacture a
warm-only distribution. Windowed counters and process samples are recorded
separately. No X11 presentation samples means display cadence is unavailable.
For before/after comparisons, `--batches 60` runs the same number and sequence
of wheel-input batches instead of the `--seconds` interval. Record both elapsed
time and completed frames; a fixed input count does not guarantee identical
compositor scheduling or displayed frame rate. The measured interval includes
waiting for a fresh host report after the final input batch, so short runs do
not reuse counters published before that input.

The image/byte caps cover owned exports, not all driver memory, and the four-GiB
RSS check is a runaway-allocation guard rather than a product performance budget.
`neon-opacity` advances its animation by frame count, so compare matched phases
when evaluating screenshots from different hosts.


## Native scene commit batching

[`native-commit.c`](native-commit.c) uses the standalone compile command in
[`text-constraints.c`](text-constraints.c), substituting its filename. Run it
under the authenticated display or `xvfb-run` with a 30-second timeout. It uses
small memory textures and genuine GTK widgets; no Pax cartridge or hardware GPU
presentation is needed.

The probe checks that several texture changes and native patches produce one
placement reconciliation, then verifies final layer-owner reparenting, obsolete
tile removal, native sibling order, parent-relative transforms, and retained
selection during same-parent placement. Actual reparenting is compared directly
with the immediate path, with selection bounds recorded before and after both
operations; this does not assume GTK preserves selection across a parent change.
It also covers an empty batch, failure before publication, and the immediate
behavior of standalone apply/texture calls. The unchanged-commit regression
requires zero placement passes for an empty native patch, unchanged owners and
unchanged tiles. Separate cases require one pass for an owner change without a
texture update, owner restoration, and tile removal without other updates.
These checks establish
native placement semantics, not input latency or final GPU pixels. The opt-in
host report's `sceneReconciliations` and `scenePlacementVisits` counters allow
the same work to be measured in a packaged application.

The October 23 runs passed all 35 checks on GTK 4.14.5 and GTK 4.22.5, including
unchanged commits, owner-only changes, and tile removal. The prior batching
implementation failed the new unchanged-commit assertion. Same-parent batching retained
the selected character range `1..3`. Both immediate and batched reparenting kept
the native label but collapsed that selection to `3..3`. Batching therefore
preserves the existing reparenting behavior; selection retention across an actual
parent change remains unsupported. These checks do not qualify focus or IME
preservation across reparenting.

## Native text and surface styling

[`native-styling.c`](native-styling.c) follows the standalone compile command in
[`text-constraints.c`](text-constraints.c), substituting its filename. It checks
rendered Button normal/hover/active colors and borders, reactive Button/Entry
surfaces, alignment, native multiline text tags, content replacement, insertion
into an empty editor, and retained-buffer safety after node deletion. Native
state setters exercise GTK's CSS states; these are not physical pointer tests.
Run it normally and with `GDK_SCALE=2`. Each configuration passed 56 checks on
GTK 4.14.5. Effective font resolution changes from 96 to 144 and back changed
actual multiline glyph heights once and restored them; the font setting used by
this standalone process is restored before exit.

The October 21 extension passed 63 checks at scale1, adding actual normal and
reactive Dropdown arrow pixels, exactly-once foreground alpha, and the real
native popup's authored background. The popup pixel probe snapshots its content
widget rather than its separate native surface owner. Earlier failed harness
runs are retained in the acceptance evidence; scale2 coverage of this extension
is separate from the earlier 56-check result.

The October 23 extension passed 71 checks on GTK 4.14.5, including fully
transparent selectable text pixels, visible selection highlighting, unchanged
measurement, and zero/half/opaque/theme foreground transitions. The prior host
failed both zero-alpha pixel cases. A separate scroller extent regression failed
against the prior host and passed with finite negative sizes normalized to an
empty native box, including positive-size recovery. The final 72-check run also
covers same-ID text deletion/recreation and passed on GTK 4.14.5 at widget scale
factors 1 and 2, and on GTK 4.22.5 at scale 1, with a Release-optimized native
harness.

[`native-event-blocker.c`](native-event-blocker.c) uses the standalone compile
command in [`text-constraints.c`](text-constraints.c), substituting its filename,
using `-o /tmp/native-event-blocker`, and adding `x11` to the `pkg-config`
dependencies. It snapshots the complete
native scene and uses actual X11 clicks through `xdotool`. Run the built probe
on an isolated display, repeating with `GDK_SCALE=2`:

```sh
timeout 35s xvfb-run -a dbus-run-session -- env \
  GDK_BACKEND=x11 GSK_RENDERER=ngl LIBGL_ALWAYS_SOFTWARE=1 \
  /tmp/native-event-blocker
```

The October 24 candidate passed all 28 checks on GTK 4.14.5 at scales 1 and 2.
These cover solid and reactive background pixels, omitted/null patch retention,
background alpha multiplied once by node opacity, masks, ancestor clipping,
transparent hit ownership, neighboring button input, and deletion/recreation.
Repeated updates retain the native widget population and identity; this is not
an allocation-lifetime test. The prior host failed nine nontransparent pixel
assertions in the same runs. Use the OpenGL renderer: GTK 4.14's Cairo fallback
paints these general-matrix transforms pink. The software OpenGL runs establish
pixel and input behavior, not GPU presentation performance.

For the shared generated text-style fixture:

```sh
python3 tests/src/native-platform-probes/linux/check-native-text-style.py \
  examples/fixtures/native-text-style/.pax/build/release/linux/native-text-style \
  --output /tmp/pax-native-text-style
```

This runner compares actual Pango block and per-line geometry for the six
swatches, edits both genuine native editors, toggles reactive styles through a
real button, and checks content/identity preservation and normal shutdown. Its
captures require visual review for typography and final appearance.
The baked Release fixture passed 58 checks directly on GTK 4.14.5 and 58 in the
installed GTK 4.22.5 Flatpak. Pass `--flatpak-app org.paxlang.NativeTextStyle`
with the same package path to exercise an already installed fixture bundle.
The focused [`native-controls.c`](native-controls.c) probe also passed 66 checks,
including actual native Pango underline application and removal on Text, Button,
Entry, Dropdown, and RadioList labels. These checks cover uniform text styling;
they do not establish Markdown, links, or mixed rich-text spans.

## Native snapping, navigation, and snapshot state

[`native-wheel.c`](native-wheel.c) exercises the raw Wheel bridge using actual
X11 mouse-wheel events on its own window. Compile like
[`native-scroller-offsets.c`](native-scroller-offsets.c), adding `x11` to the
`pkg-config` dependencies, and run on a private display:

```sh
timeout 30s xvfb-run -a dbus-run-session -- env \
  GDK_BACKEND=x11 GSK_RENDERER=cairo /tmp/native-wheel
```

Repeat with `GDK_SCALE=2`. The probe checks one interrupt over nested native
controls, logical coordinates with an embedded scene offset, both directions
and axes, live modifiers, cancellation before native default, uncancelled native
movement and position acknowledgments, vector-only input, and stopped-scene
silence. It does not synthesize scrolling or claim physical touchpad/momentum
qualification. Shared dispatcher tests separately cover Pax hit testing,
handler bubbling, and default-prevention return values.

The October 24 candidate passed all 24 checks on GTK 4.14.5 and the pinned
GTK 4.22.5 Flatpak runtime at widget scale factors 1 and 2. The prior host failed
eleven assertions at each scale on GTK 4.14.5; an intermediate live-query
candidate failed the queued-position assertion at each scale. GDK scroll events on X11 do not
provide an embedded event position. The host preserves preceding pointer events
per seat in surface coordinates, then converts to current scene coordinates.
This keeps a queued wheel's hit point correct even if later pointer motion is
already queued while the UI thread is busy. Leave, grab loss, unmap and shutdown
clear this state. If no preceding position is available, it queries the logical
seat pointer; querying a physical X11 source slave is invalid. The extra checks
cover the queued wheel/motion regression and remapping. Earlier failed candidates
are preserved in the test evidence.

[`check-native-wheel.py`](check-native-wheel.py) runs the
[`native-wheel`](../../../../examples/fixtures/native-wheel/README.md) Pax fixture
with owned X11 input. The fixture's actual Snapshot button emits Rust handler
counts and bound positions; the probe compares those positions with the host's
native adjustment reports. It checks nested bubbling, both axes, root-handler
cancellation, opaque unrelated siblings, rounded viewport ownership, modifiers,
and normal shutdown. Two sibling-overlay points lie outside the observed native
text label, over vector paint alone. Each must dispatch exactly once to the
overlay and root with cancellation disabled, while both native viewports stay
still. Use an already built Linux package, or pass
`--flatpak-app` for a package installed in the pinned runtime:

```sh
python3 tests/src/native-platform-probes/linux/check-native-wheel.py \
  examples/fixtures/native-wheel/.pax/build/release/linux/native-wheel \
  --output /tmp/pax-native-wheel --screenshots
```

The probe leaves application state changes to authored Reset and Checkbox
controls. It never injects a Pax event or writes a scroll offset. Its raw X11
notches each require exactly one callback per expected handler. These are
functional mouse-wheel tests; physical touchpad gestures can deliver multiple
events, and their device-dependent momentum still needs separate qualification.
The complete Release CLI package passed all 162 assertions in the installed
GTK 4.22.5 runtime on the NVIDIA Xorg guest at scale 1. This package was built
with GTK 4.14.5 development headers through the ordinary CLI's embedded SDK
path, with `PAX_HOST_SDK_DIR` unset and no output substitution. The rounded inner
corner delivered callbacks only to the outer viewport and root while moving only the
outer viewport; the outer corner delivered only the root callback with no native
movement. These checks exercise the actual Rust handlers and bound positions,
in addition to the isolated C bridge coverage above.

[`native-scrolling.c`](native-scrolling.c) uses the standalone command in
[`text-constraints.c`](text-constraints.c). Its 28-check GTK 4.14.5 result covers
both-axis point normalization, dynamic clearing and extent restoration,
acknowledgement suppression, held continuous input, interrupted animation,
no-motion release, hidden/remapped widgets, reduced motion, and shutdown with
a retained adjustment. Controller signals in this C probe are synthetic lifecycle
tests; they do not qualify a physical touchpad.

[`native-scroller-offsets.c`](native-scroller-offsets.c) has a standalone compile
command in its header. Run it with a 30-second timeout in a GTK session, under
`xvfb-run`, or on an isolated headless Weston display with `GDK_BACKEND=wayland`
and `GSK_RENDERER=cairo`. It waits for actual GTK main-loop allocation before
checking authored offsets and their `ScrollerPosition` acknowledgments. Cases
cover initial nonzero offsets, viewport resizing, content growth paired with a
position request, exact echoes, explicit zero, legal clamping, shrink/regrowth,
native identity retention, per-axis pending updates, and deletion before allocation.

The October 24 candidate passed all 19 offset checks on GTK 4.14.5 and the
installed GTK 4.22 Flatpak runtime using an isolated headless Wayland display.
The prior implementation acknowledged `(0,0)` before its first allocation and
clamped a content-growth request against the old extent. The unchanged adjacent
snapping and commit probes passed 28 and 35 checks on GTK 4.14.5; the real X11
pointer-focus probe passed 15 checks at each of scales 1 and 2 on private Xvfb
displays. These checks establish allocation and native interaction behavior;
software-rendered virtual displays do not qualify GPU presentation performance.

The generated fixture runner uses actual X11 input:

```sh
python3 tests/src/native-platform-probes/linux/check-native-snapping.py \
  examples/fixtures/native-scroll-snap/.pax/build/release/linux/native-scroll-snap \
  --output /tmp/pax-native-snapping
```

It compares native offsets with the shared Rust readout through real wheel input,
authored jumps, dynamic point lists, viewport resizing, held pointer input, and
native scrollbar dragging. It also checks native child hit testing and that
unrelated pointer motion does not delay settling. See the
[host's settling policy](../../../../pax-host/native/linux/README.md#scrolling-and-native-routes)
for the 150ms quiescence compromise.

[`navigation-contract.c`](navigation-contract.c) has its standalone compile
command in its header and consumes the shared
[`navigation-cases.json`](../navigation-cases.json). The current 25-check result
includes repeated query keys, Unicode and escaped separators, literal plus signs,
empty query components, trailing-slash normalization, and ignored empty
destinations. The generated application runner checks actual Router/Link updates
and resolved default-font glyphs:

```sh
python3 tests/src/native-platform-probes/linux/check-native-navigation.py \
  examples/fixtures/native-navigation/.pax/build/release/linux/native-navigation \
  --output /tmp/pax-native-navigation
```

[`native-snapshot.c`](native-snapshot.c) uses the same GTK compile command as the
scrolling probe. It checks final pixels through nested world-space clips and
opaque/fractional native masks. The old snapshot stack order reproduces four
save/restore warnings; the corrected host passes all17 checks with zero warnings
and the same expected interior, clipped-edge, and mask pixels. This is a focused
snapshot-state regression, not a solution to the deferred translucency contract.

The canonical Neon runner additionally accepts `--check-neon-surfaces`: actual
controls pause animation and select full group opacity before final screenshot
pixels are checked against authored Button/Textbox background colors. This
optional interaction happens after its short timing window.

## Animation and idle memory diagnostics

```sh
python3 tests/src/native-platform-probes/linux/check-native-performance.py \
  examples/fixtures/neon-opacity/.pax/build/release/linux/neon-opacity \
  --seconds 120 --output /tmp/pax-neon-memory
```

The runner samples process CPU, RSS and `/proc/PID/smaps_rollup`, host ticks and
commits, and owned GPU publications once per second. It observes animation for
the selected duration, clicks the canonical Pause Animation button, then observes
the same process for another equal interval while native event delivery continues.
Use `--cycles 2` to resume and repeat animation/pause in the same process;
`--paused-seconds 60` selects a shorter pause independently of `--seconds`.
Repeated animation helps distinguish cache warm-up from continuing growth.
`--no-screenshots` preserves JSON state snapshots while omitting GPU readback
and PNG captures, whose temporary allocations can raise the process's retained
memory between phases. Keep that choice identical within a comparison.
`PAX_GPU_MEMORY_DIAGNOSTICS=1` also enables bounded renderer cache summaries in the
app log. `PAX_PROPERTY_MEMORY_DIAGNOSTICS=1` adds shared property-graph population
counters. `PaxGpuCacheMemory` reports maxima observed during cache operations,
including a lifetime peak; a zero window can mean no cache operation reported a
value. Its companion `PaxGpuCacheOwnership` record samples current cache charges,
entry counts and configured budgets, separating resource entries owned only by
the cache from those with additional Rust owners. The resource cache can exceed
its eviction target while entries have external owners. Sampling borrows entries
without changing their ownership or LRU order, only on the existing first/every
120th layer-flush reporting cadence. Sibling tiles share caches: use the reported
cache identities to avoid counting them twice. Identities are process-local and
may be reused after a cache is destroyed. `sample_unix_ms` supplies a wall-clock
correlation point; preserve monotonic process samples too when measuring rates.
These are logical cache charges, not total process memory, driver allocations or
in-flight GPU ownership. Inactive layers do not flush and therefore do not emit
fresh samples; the last record is not a current global residency total.
The CPU geometry cache charges retained vector capacities and inline entries
against its budget, rather than just the populated geometry. Hash-map spare
buckets, control bytes, and allocator overhead are not included in that counter.
Equal elapsed time can contain different amounts of animation work;
compare memory against both elapsed time and completed frames. Startup-inclusive
timing histograms and Xorg callback throughput do not qualify physical display
framerate. A plateau in one bounded run is not a general leak-free guarantee.
For a shared-engine comparison, preserve identical host, fixture, diagnostic,
and input settings; build isolated packages and record the loaded-library hashes.

### Packaged font faces and CLI first touch

[`native-font-loading.c`](native-font-loading.c) exercises sequential registration
of regular and italic faces after GTK has already resolved fonts, checks the
actual Fontconfig file used for glyphs, and repeats the same files in a separate
scene configuration. It also checks that adding a face through another control
invalidates an existing automatic Text measurement with its current generation,
updates that label without a content patch, and stays quiet on unchanged batches.
Build it using the `text-constraints.c` command, substitute this filename, and pass
`pax-std/native-assets/pax-default-fonts` as its argument. PangoFc (`pangoft2`) is an
explicit build dependency.

The generated navigation runner additionally compares the actual regular, bold,
and italic glyph files with the packaged asset hashes, drags a real native text
selection, and checks exactly one route update and one observed pointer press for
each Button/Link action. It also rejects navigation after a selection drag across
a Link or a release without a matching scene press. Input baselines wait for the
native report to observe the preceding release, since route logs and periodic
scene reports are published independently. Family names alone are insufficient
font provenance.

The October 21 T4 acceptance passed 18 focused font-registration checks and 125
Text constraint checks on GTK 4.14.5. The generated navigation runner passed 53
checks in each of Debug, baked Release, and the installed GTK 4.22.5 Release
Flatpak; the generated snapping runner passed 25 checks in each configuration.
Every run closed normally with owned GL publications retired. These are X11
input checks and do not qualify a physical touchpad or desktop Wayland input.

[`check-native-first-touch.py`](check-native-first-touch.py) accepts an unchanged
`pax-cli create` default PaxLogo package. It exercises the genuine Slider, theme
Button, vector replay click, animation, actual packaged default-font bytes, clean
close, and owned GL retirement:

```sh
python3 tests/src/native-platform-probes/linux/check-native-first-touch.py \
  /absolute/package/linux-alpha-first-touch --title linux-alpha-first-touch \
  --output /absolute/new-evidence-directory
```

When testing unpublished local Pax sources, preserve the original scaffold and
record the Cargo dependency path adjustment separately. Do not replace the
scaffold template or inject explicit font declarations to make this gate pass.
The checker also accepts the shared Flatpak launch options; package relocation
should preserve the launcher, cartridge, host and asset hashes.

The October 21 first-touch gate passed all 11 checks in four configurations:
the exact CLI Debug package, the exact CLI baked Release package, that Release
package copied to another directory with the source/compiler paths hidden, and
the installed GTK 4.22.5 Flatpak. Both CLI builds ran without `PAX_HOST_SDK_DIR`;
the generated native host inputs and common ABI header matched the qualified
sources. The receiver check ran from `/tmp` in a private mount namespace on the
same machine, not on a separately provisioned distribution. The original
scaffold, Cargo-only path adjustment, package hashes, runtime/app commits and
loaded-library identities accompany the acceptance evidence.
