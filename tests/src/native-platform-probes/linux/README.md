# Linux native composition probes

These isolated probes exercise GTK native-widget composition and WGPU 28 image
interop. They are not a Pax Linux application target or a production frame loop.
They use real GTK controls and no browser or WebView.

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
probe does not silently switch transports. To exercise Vulkan opaque file
descriptors imported by `GL_EXT_memory_object_fd` and `GdkGLTextureBuilder`:

```sh
PAX_PROBE_GPU_TRANSPORT=opaque-fd bash check-x11.sh
PAX_PROBE_GPU_TRANSPORT=opaque-fd bash check-wayland.sh
```

The optional bridge renders three color-and-clip vector scenes through `pax-gpu`
into owned offscreen targets, then copies each target once on the shared WGPU
queue into an exportable image. These immutable imports appear both as separate
tiles and underneath live transformed GTK controls in the same GSK composition.
One-shot diagnostic downloads verify every imported pixel against the clipped
Pax pattern, and a varying-size allocation cycle verifies GDK release callbacks
and Vulkan allocation retirement. This diagnostic readback is not part of
presentation. The image presentation path
exports native memory without CPU readback and holds it until GDK releases it.
GPU completion and teardown currently wait synchronously, so this probe does
not establish a suitable synchronization or buffer-reuse policy for animation.
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
