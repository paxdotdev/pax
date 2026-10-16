# Linux GTK host

This SDK runs a real Pax cartridge in a GTK 4 application. A user-owned Rust
`main` uses `pax_kit::init_with!` and `pax_host` to configure the cartridge, pass
startup data, and open a window or mount into an existing `GtkBox`/empty
`GtkWindow`. The compiler packages the cartridge, this shared library, and ordered
asset roots beside the executable. The host and cartridge negotiate desktop ABI 4.

Build the native SDK on Linux:

```sh
./pax-host/native/linux/build.sh --output-directory /absolute/sdk --configuration Debug
```

Development packages must provide GTK >=4.14, epoxy, json-glib, and Fontconfig
through `pkg-config`. The supported renderer selection is `GSK_RENDERER=ngl`;
legacy GTK 4.14 `gl` has a separately recorded DMA-BUF texture-binding failure in
the platform probes. A compatible Vulkan physical GPU, exportable linear RGBA8
DMA-BUF image, foreign queue ownership, binary SYNC_FD import/export, EGL native
fences, EGL DMA-BUF modifiers, and GLES3 external images are required. There is no
software adapter or CPU presentation fallback. `PAX_GPU_DEVICE_UUID` selects the
exact Vulkan device using 32 hexadecimal digits; a failed selection is terminal.

The retained GTK scene provides plain Text/selection, single-line Textbox,
Button, Checkbox, Frame, Scroller, and EventBlocker nodes. Editable Text and
TextArea use GTK text views, but their typography/style fidelity is incomplete;
markdown, link styling, and general rich text are not qualified. It updates existing
widgets so edits and selection survive geometry and rendering changes. Native
masks use GSK paths and alpha masks; picking checks the same paths, so fully
opaque vector occlusion passes input through and fractional occlusion retains
native interaction. **Fractional visual composition is not correct yet:** the
actual fixture's 50% yellow vector over a white Textbox produces RGB
`[251, 235, 201]` instead of `[247, 215, 148]` in both Debug and Release. The
vector is already blended in the GPU tile, then the inverse native alpha mask
attenuates its contribution again. Input coverage passes, but translucent vectors
over native content are not visually qualified. Correct composition requires
separating vector underlays/foregrounds around native surfaces, or including
native snapshots in a shared GPU compositor; changing mask weights is not a
correct general fix.

Native siblings use their current `z_index`; GPU tiles paint
below native peers within each container, with masks revealing foreground
vectors. Creation-time native layer IDs are not a live stacking order. GPU tiles belong to their logical layer/scroller. Other
native control families currently return an explicit unsupported error. Real
IME composition remains owned by GTK, but an IME acceptance run is still required.

System font families and bundled font files work through Pango/Fontconfig.
Remote font or image URLs are currently unsupported. Packaged images are decoded
once on the GTK thread and uploaded through the cartridge image API; large-image
loading is not yet asynchronous. The actual fixture uses a bundled Manrope font
and a packaged image. These input uploads are separate from presentation, which
does not download rendered frames.

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
checks visible scroller text against an initial unscrolled capture, startup data,
Rust button/input handlers, retained selection, nested
scrolling, opaque/fractional mask input, actual Pango font resolution, resize,
allocation counters, export reuse, and close. Its explicit
one-shot screenshots are test-only GPU downloads. Production presentation has no
such readback. Passing the C build, Rust check, or earlier immutable platform
probe alone does not qualify this continuous host, its release cartridge, native
Wayland, embedding, IME, or device-loss recovery; record each actual run separately.

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
