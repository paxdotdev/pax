#!/usr/bin/env python3
"""Exercise real composing InputConnection edits on an emulator; restore its IME."""
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
IME = 'dev.pax.android_ime/.CompositionIme'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True, type=Path)
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
        assert encoded, status
        xml = gzip.decompress(base64.b64decode(encoded[1])).decode()
        (args.output/'last-ui.xml').write_text(xml)
        return list(ET.fromstring(xml).iter('node'))

    def find(text):
        deadline = time.monotonic()+20
        while time.monotonic() < deadline:
            for node in snapshot():
                if node.get('text') == text:
                    return node
            time.sleep(0.2)
        raise AssertionError(f'Native editing did not produce {text!r}')

    def step(value):
        response = adb('shell', 'am', 'broadcast', '-a', 'dev.pax.android_ime.STEP',
                       '-p', 'dev.pax.android_ime', '--es', 'step', value)
        assert 'result=-1' in response and 'data="ok"' in response, response

    assert adb('shell', 'getprop', 'ro.kernel.qemu').strip() == '1', 'This test changes the keyboard only on an emulator'
    original = adb('shell', 'settings', 'get', 'secure', 'default_input_method').strip()
    assert original and original != 'null', 'No original keyboard to restore'
    enabled = adb('shell', 'settings', 'get', 'secure', 'enabled_input_methods').strip()
    already_enabled = any(item.split(';')[0] == IME for item in enabled.split(':'))
    result['originalIme'] = original
    try:
        adb('shell', 'ime', 'enable', IME)
        adb('shell', 'ime', 'set', IME)
        for mode, prefix in [('ime', 'Rust: '), ('rich-text', 'Text edit: ')]:
            adb('shell', 'am', 'force-stop', APP)
            adb('shell', 'am', 'start', '-W', '-n', APP+'/dev.pax.PaxActivity', '--es', 'pax.startup', mode)
            node = find('Native text and IME')
            x0, y0, x1, y1 = map(int, re.findall(r'\d+', node.get('bounds')))
            adb('shell', 'input', 'tap', str((x0+x1)//2), str((y0+y1)//2))
            deadline = time.monotonic()+10
            while True:
                try:
                    step('ready')
                    break
                except AssertionError:
                    if time.monotonic() >= deadline:
                        raise
                    time.sleep(0.2)
            step('reset')
            for operation, text in [('start', 'に'), ('update', '日本'), ('commit', '日本語'),
                                    ('unicode', '日本語 e\u0301 مرحبا 👩‍💻'),
                                    ('delete-emoji', '日本語 e\u0301 مرحبا ')]:
                step(operation)
                find(prefix+text)
                # Allow runtime frames between composing updates. An echo that
                # replaces the editable buffer would lose its composing range.
                time.sleep(0.25)
            result['checks'].append(mode+': composing replacement / commit / combining marks / RTL / emoji deletion')
            (args.output/(mode+'.png')).write_bytes(adb('exec-out', 'screencap', '-p', binary=True))
    except Exception as error:
        result['failures'].append(str(error))
        raise
    finally:
        try:
            adb('shell', 'ime', 'set', original)
            if not already_enabled:
                adb('shell', 'ime', 'disable', IME)
            restored = adb('shell', 'settings', 'get', 'secure', 'default_input_method').strip()
            result['restoredIme'] = restored
            assert restored == original, 'Original keyboard was not restored'
        finally:
            (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
