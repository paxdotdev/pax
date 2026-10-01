"""Create temporary wake-only host assets after building Async Workbench for web.

Disables continuous RAF rescheduling and the hidden-tab timer fallback in a copy
of the generated host. Initial frames, input, property/channel wake routes, and
shutdown wake routes stay intact. Open /wake-only.html on the fixture's dev URL.
The generated copies live under .pax and must not be committed.
"""
from pathlib import Path

root = Path(__file__).resolve().parents[3]
build = root / "examples/src/async-workbench/.pax/build/debug/web"
source = (build / "pax-interface-web.js").read_text()
start = source.index("  function renderLoop(chassis, mount2) {")
end = source.index("\n  }", start)
loop = source[start:end]
recurrence = "frameScheduler.schedule(() => renderLoop(chassis, mount2));"
assert loop.count(recurrence) == 1
source = source[:start] + loop.replace(recurrence, "// Wake-only qualification: no recurring RAF.") + source[end:]
fallback = "hiddenTabPumpHandle = window.setInterval(pumpHiddenFrame, HIDDEN_TAB_FRAME_FALLBACK_MS);"
assert source.count(fallback) == 1
source = source.replace(fallback, "// Wake-only qualification: no hidden-tab timer.")
(build / "pax-interface-wake-only.js").write_text(source)
html = (build / "index.html").read_text().replace("pax-interface-web.js", "pax-interface-wake-only.js")
(build / "wake-only.html").write_text(html)
print("Open /wake-only.html on the Async Workbench dev server.")
