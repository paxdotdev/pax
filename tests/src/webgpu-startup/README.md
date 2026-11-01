# Browser GPU startup regression probe

Build and serve Spatial Workbench, then generate an opt-in test page:

```sh
pax-cli run --path examples/fixtures/spatial-workbench --target web --port 8093
# In another terminal, after the build finishes:
python3 tests/src/webgpu-startup/prepare.py examples/fixtures/spatial-workbench/.pax/build/debug/web
```

Open `/gpu-startup-test.html?pax_log=info&pax_renderer=gpu&gpu_test=real`.
The visible trace must end in `PASS`, with exactly one adapter and one device
request. A second adapter request is rejected to detect the old discarded
capability-probe adapter. The real browser promises and application rendering
remain in use; a pending request is not replaced by a timeout or fallback.

Repeat with `gpu_test=adapter-null`, `adapter-reject`, `device-reject`,
`gpu-absent`, and `context-null`. For each, test both `pax_renderer=gpu`
(startup must reject) and `pax_renderer=auto` (must mount with Vello CPU).
The trace checks request counts and the selected renderer as well as settlement.
Use a browser that normally supports WebGPU; keep `pax_log=info` for the renderer
assertions. CPU mode does not qualify Spatial Workbench's 3D rendering.

These injected conditions are confined to this test page. Rebuilding the app
may remove the generated page; regenerate it when needed. Remove the generated
`gpu-startup-test.html` after qualification. The ordinary `index.html` and host
bundle are never modified.
