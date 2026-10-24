#!/usr/bin/env python3
"""Qualify native text spans, layout, route links, selection and source editing."""
import argparse
import base64
import gzip
import json
import os
from pathlib import Path
import re
import subprocess
import time
import xml.etree.ElementTree as ET

APP = 'dev.pax.native_android_smoke'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--adb', default=str(Path(os.environ.get('ANDROID_HOME', ''))/'platform-tools/adb'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output/'result.json').unlink(missing_ok=True)
    result = {'checks': [], 'failures': []}

    def adb(*command, binary=False):
        return subprocess.check_output([args.adb, *command], text=not binary, timeout=40)

    def snapshot():
        status = adb('shell', 'am', 'instrument', '-w', 'dev.pax.android_probe/dev.pax.android_probe.Probe')
        encoded = re.search(r'INSTRUMENTATION_RESULT: hierarchy_gzip=([^\r\n]+)', status)
        assert encoded, 'Install the UI helper with build-ui-probe.py --install: '+status
        xml = gzip.decompress(base64.b64decode(encoded[1])).decode()
        (args.output/'last-ui.xml').write_text(xml)
        return list(ET.fromstring(xml).iter('node'))

    def find(text, timeout=30):
        deadline = time.monotonic()+timeout
        while time.monotonic() < deadline:
            for node in snapshot():
                if node.get('text') == text:
                    return node
            time.sleep(0.2)
        raise AssertionError(f'Missing native text: {text!r}')

    def bounds(node):
        return list(map(int, re.findall(r'-?\d+', node.get('bounds'))))

    def tap(node):
        x0, y0, x1, y1 = bounds(node)
        adb('shell', 'input', 'tap', str((x0+x1)//2), str((y0+y1)//2))

    def styled(node, substring, style):
        content = node.get('text').encode('utf-16-le')
        return any(content[2*start:2*end].decode('utf-16-le') == substring and value == style
                   for start, end, value in (map(int, match) for match in
                       re.findall(r'(\d+):(\d+):(\d+),', node.get('span-styles', ''))))

    pid = ''
    try:
        adb('shell', 'am', 'force-stop', APP)
        adb('shell', 'am', 'start', '-W', '-n', APP+'/dev.pax.PaxActivity', '--es', 'pax.startup', 'rich-text')
        pid = adb('shell', 'pidof', APP).strip()
        content = 'Native Markdown\n\nA bold and italic 🦀.\n\n1. first\n2. second\n\na < b'
        rich = find(content)
        assert styled(rich, 'bold', 1) and styled(rich, 'italic', 2), rich
        assert 'monospace' in rich.get('span-fonts', ''), rich
        assert bounds(rich)[3]-bounds(rich)[1] > 250, 'Rich-text intrinsic height collapsed'
        code = find('  <Text text="🦀 &lt;"/>\n    second line\n')
        assert {'ff202830', 'ff135ea8', 'ffa83e13'} <= set(code.get('span-colors', '').split(',')), code
        assert not code.get('span-fonts', ''), 'Generated source must inherit its authored font'
        find('**plain** &lt;Text&gt;')
        result['checks'].append('Markdown spans / UTF-16 / source colors / no-wrap newlines / plain text')
        (args.output/'rich-text.png').write_bytes(adb('exec-out', 'screencap', '-p', binary=True))
        tap(find('Toggle emphasis'))
        rich = find(content)
        assert styled(rich, 'bold', 2) and styled(rich, 'italic', 1), 'Formatting-only update was lost'
        result['checks'].append('formatting-only update with unchanged displayed string')
        for _ in range(3):
            x0, y0, x1, _ = bounds(find('Go to route'))
            density = (x1-x0)/170
            adb('shell', 'input', 'tap', str(round(x0+15*density)), str(round(y0+10*density)))
            find('At destination')
            adb('shell', 'input', 'keyevent', 'KEYCODE_BACK')
            find('At home')
            assert adb('shell', 'pidof', APP).strip() == pid
        # Cargo metadata deliberately delegates this path to a server on web.
        # Native navigation still selects the shared Router branch in this app.
        result['checks'].append('native Markdown link / web policy ignored / three nested canvas route and Back cycles')
        # Select actual native characters, not a proxy control or app test hook.
        x0, y0, _, _ = bounds(find(content))
        x, y = str(round(x0+40*density)), str(round(y0+15*density))
        adb('shell', 'input', 'swipe', x, y, x, y, '1200')
        selected = find(content)
        assert int(selected.get('selection-end', '-1')) > int(selected.get('selection-start', '-1')) >= 0, 'Native text has no selection'
        (args.output/'selection.png').write_bytes(adb('exec-out', 'screencap', '-p', binary=True))
        adb('shell', 'input', 'keyevent', 'KEYCODE_BACK')
        result['checks'].append('native text selection')
        tap(find('Native text and IME'))
        adb('shell', 'input', 'keyevent', 'KEYCODE_MOVE_END')
        adb('shell', 'input', 'text', '-editable')
        find('Text edit: Native text and IME-editable')
        adb('shell', 'input', 'keyevent', 'KEYCODE_BACK')
        result['checks'].append('editable Text writes through its bound property')
    except Exception as error:
        result['failures'].append(str(error))
        raise
    finally:
        if pid:
            (args.output/'host.log').write_text(adb('logcat', '-d', '--pid='+pid, '-s', 'Pax', 'AndroidRuntime'))
            (args.output/'final.png').write_bytes(adb('exec-out', 'screencap', '-p', binary=True))
        (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
