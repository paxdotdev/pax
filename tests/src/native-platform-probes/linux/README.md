# Linux native composition probes

These isolated probes exercise GTK native-widget composition and WGPU 28 image
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
Vulkan headers/loader, libepoxy, Xvfb, xauth, xdotool, and Weston. The Rust export
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
