# Browser GPU startup regression probe

Build and serve Increment, a small 2D scene that works on both renderers, then
generate an opt-in test page:

```sh
pax-cli run --path examples/increment --target web --port 8093
# In another terminal, after the build finishes:
python3 tests/src/webgpu-startup/prepare.py examples/increment/.pax/build/debug/web
```

Open `/gpu-startup-test.html?pax_renderer=gpu&gpu_test=real`.
The visible trace must end in `PASS`, with exactly one adapter and one device
request. A second adapter request is rejected to detect the old discarded
capability-probe adapter. The real browser promises and application rendering
remain in use; a pending request is not replaced by a timeout or fallback.

Repeat with `gpu_test=adapter-null`, `adapter-reject`, `device-reject`,
`gpu-absent`, and `context-null`. For each, test both `pax_renderer=gpu`
(startup must reject) and `pax_renderer=auto` (must mount with Vello CPU).
Also test `gpu_test=real` with `pax_renderer=auto` (GPU, one adapter/device)
and `pax_renderer=cpu` (CPU, zero adapter/device requests).

The trace checks request counts, startup settlement, and the rendering contexts
acquired by mounted Pax layer canvases: `webgpu` identifies GPU and `2d`
identifies Vello CPU presentation. It observes the app's successful `getContext`
calls without acquiring extra contexts. Detached initialization probes, retired
surfaces, auxiliary canvases, and uninitialized tiles do not count. At least one
initialized Pax layer canvas must exist on successful mount, and all observed
layer contexts must agree with the expected renderer. Renderer assertions do
not depend on log wording or verbosity; `pax_log=info` is optional diagnostics.

Use a desktop browser that normally supports WebGPU and a build without the
`cpu` feature for this matrix. Run it in actual Firefox and Chrome, recording
versions, viewport, DPR, and any console errors. `PASS` qualifies acquisition
and mounting, not pixels or performance: separately check that Increment's
rectangle and text render, clicking increments the count, and resizing keeps
them aligned. The overlay ignores pointer input. Other apps with canvas content
can use the same generated page, but CPU mode does not qualify Spatial
Workbench's 3D rendering.

Probe regression checks (from the repository root):

```sh
node --test tests/src/webgpu-startup/probe.test.mjs
```

These injected conditions are confined to this test page. Rebuilding the app
may remove the generated page; regenerate it when needed. Remove the generated
`gpu-startup-test.html` after qualification. The ordinary `index.html` and host
bundle are never modified.
