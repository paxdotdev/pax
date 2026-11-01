"""Add an opt-in WebGPU startup regression page to a built web app."""
import argparse
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("build", type=Path, help="The app's .pax/build/debug/web directory")
args = parser.parse_args()
html = (args.build / "index.html").read_text()
script = Path(__file__).with_name("probe.js").read_text()
marker = '<script src="pax-interface-web.js"'
assert html.count(marker) == 1
html = html.replace(marker, '<script>' + script + '</script>\n' + marker)
mounted = '.then(app => { window.paxApp = app; })'
failed = '.catch(error => console.error("Pax initialization failed", error))'
assert mounted in html and failed in html
html = html.replace(mounted, '.then(app => { window.paxApp = app; window.finishGpuProbe(true); })')
html = html.replace(failed, '.catch(error => { console.error("Pax initialization failed", error); window.finishGpuProbe(false); })')
(args.build / "gpu-startup-test.html").write_text(html)
print("Created gpu-startup-test.html; see tests/src/webgpu-startup/README.md for test URLs.")
