#!/usr/bin/env python3
"""Tap the installed Gboard UI in the emulator's native controls fixture."""
import argparse
import base64
import gzip
import json
import re
import subprocess
import time
from pathlib import Path
import xml.etree.ElementTree as ET

APP = 'dev.pax.native_android_smoke'
GBOARD = 'com.google.android.inputmethod.latin'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--adb', required=True)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--profile', required=True, choices=['debug', 'release'])
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    result = {'profile': args.profile, 'checks': [], 'failures': []}
    sequence = 0

    def adb(*command, binary=False):
        return subprocess.check_output([args.adb, *map(str, command)], text=not binary, timeout=45)

    def shell(*command):
        return adb('shell', *command)

    def snapshot():
        nonlocal sequence
        status = shell('am', 'instrument', '-w', '-e', 'preserve_services', 'true', '-e', 'all_windows', 'true',
                       'dev.pax.android_probe/dev.pax.android_probe.Probe')
        encoded = re.search(r'INSTRUMENTATION_RESULT: hierarchy_gzip=([^\r\n]+)', status)
        assert encoded, status
        xml = gzip.decompress(base64.b64decode(encoded[1])).decode()
        (args.output/f'{sequence:03d}.xml').write_text(xml)
        sequence += 1
        return list(ET.fromstring(xml).iter('node'))

    def find(nodes, **attributes):
        return next(n for n in nodes if all(n.get(k.replace('_', '-')) == v for k, v in attributes.items()))

    def tap(node):
        x0, y0, x1, y1 = map(int, re.findall(r'-?\d+', node.get('bounds')))
        assert x1 > x0 and y1 > y0
        shell('input', 'tap', (x0+x1)//2, (y0+y1)//2)

    def wait_for(**attributes):
        deadline = time.monotonic()+12
        while True:
            nodes = snapshot()
            try:
                return nodes, find(nodes, **attributes)
            except StopIteration:
                if time.monotonic() >= deadline:
                    raise AssertionError(f'Gboard transition did not expose {attributes}')
                time.sleep(0.15)

    def check_value(expected, prefix, *, echo=False):
        # A freshly connected UiAutomation can enumerate a window before its
        # root is available. Wait for the editor, without retrying wrong values.
        nodes, field = wait_for(package=APP, focused='true')
        actual = field.get('text')
        assert actual == expected, (actual, expected)
        if echo:
            find(nodes, package=APP, text=prefix+expected)
        return nodes

    def type_keys(nodes, text):
        # Bounds come from Gboard's live keys. No synthetic key events or
        # accessibility SET_TEXT actions stand in for software-keyboard input.
        keys = {n.get('content-desc').lower(): n for n in nodes
                if n.get('package') == GBOARD and 'key_pos_' in n.get('resource-id', '')}
        for letter in text:
            tap(keys[letter])
            time.sleep(0.08)

    try:
        assert shell('getprop', 'ro.kernel.qemu').strip() == '1', 'Use this probe only on an emulator'
        assert shell('settings', 'get', 'secure', 'default_input_method').startswith(GBOARD+'/'), 'Select Gboard first'
        assert shell('settings', 'get', 'secure', 'accessibility_enabled').strip() == '0', 'Run without TalkBack; it changes touch gestures'
        package = shell('dumpsys', 'package', GBOARD)
        result['keyboardVersion'] = re.search(r'versionName=([^\n]+)', package)[1]
        result['device'] = shell('getprop', 'ro.build.fingerprint').strip()
        for mode, prefix in [('ime', 'Rust: '), ('rich-text', 'Text edit: '), ('uppercase', 'Rust: ')]:
            shell('am', 'force-stop', APP)
            shell('am', 'start', '-W', '-n', APP+'/dev.pax.PaxActivity', '--es', 'pax.startup', mode)
            deadline = time.monotonic()+25
            while True:
                nodes = snapshot()
                try:
                    field = find(nodes, package=APP, text='Native text and IME')
                    assert field.get('editable') == 'true', 'Native editor must advertise editability before focus'
                    assert field.get('class') == 'android.widget.EditText', 'Native editor must expose its editing role'
                    break
                except StopIteration:
                    if time.monotonic() >= deadline:
                        raise AssertionError('Native editor did not appear')
                    time.sleep(0.25)
            tap(field)
            wait_for(package=APP, focused='true')
            nodes, _ = wait_for(package=GBOARD, resource_id=GBOARD+':id/key_pos_1_5')
            # Clear the fixture's initial value before testing Gboard itself.
            shell('input', 'keycombination', 'KEYCODE_CTRL_LEFT', 'KEYCODE_A')
            shell('input', 'keyevent', 'KEYCODE_DEL')
            check_value('', prefix)
            if mode == 'uppercase':
                type_keys(nodes, 'pax')
                shell('input', 'keyevent', 'KEYCODE_BACK')
                check_value('PAX', prefix, echo=True)
                result['checks'].append('Gboard input with application uppercase transformation')
            else:
                type_keys(nodes, 'hel')
                check_value('hel', prefix)
                _, suggestion = wait_for(package=GBOARD, content_desc='hello')
                tap(suggestion)
                check_value('hello ', prefix)
                _, emoji_button = wait_for(package=GBOARD, content_desc='Emoji button')
                tap(emoji_button)
                _, emoji = wait_for(package=GBOARD, content_desc='😀')
                tap(emoji)
                _, letters = wait_for(package=GBOARD, content_desc='Letter keyboard')
                tap(letters)
                nodes, delete = wait_for(package=GBOARD, content_desc='Delete')
                check_value('hello 😀', prefix)
                tap(delete)
                shell('input', 'keyevent', 'KEYCODE_BACK')
                check_value('hello ', prefix, echo=True)
                result['checks'].append(mode+': visible Gboard keys, suggestion replacement, emoji insertion/deletion, Rust binding')
            (args.output/(mode+'.png')).write_bytes(adb('exec-out', 'screencap', '-p', binary=True))
        result['passed'] = True
    except Exception as error:
        result['passed'] = False
        result['failures'].append(repr(error))
        raise
    finally:
        (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
