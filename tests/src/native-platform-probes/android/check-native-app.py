#!/usr/bin/env python3
"""Exercise an installed native-android-smoke APK through Android's real UI.

Run with ANDROID_SERIAL when multiple devices are connected. This only operates
on the fixture package; it restores the device's rotation settings on exit.
"""
import argparse
import base64
import json
import os
from pathlib import Path
import re
import subprocess
import time
import xml.etree.ElementTree as ET

APP = 'dev.pax.native_android_smoke'
ACTIVITY = APP + '/dev.pax.PaxActivity'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--adb', default=str(Path(os.environ.get('ANDROID_HOME', '')) / 'platform-tools/adb'))
    parser.add_argument('--remote-assets', action='store_true', help='Also qualify HTTPS PNGs and a Google Fonts CSS source; needs Internet access')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output/'result.json').unlink(missing_ok=True)

    def adb(*command, binary=False):
        return subprocess.check_output([args.adb, *command], timeout=30, text=not binary)

    def snapshot():
        adb('shell', 'uiautomator', 'dump', '/sdcard/pax-smoke.xml')
        return ET.fromstring(adb('exec-out', 'cat', '/sdcard/pax-smoke.xml'))

    def find(text=None, kind=None):
        for node in snapshot().iter('node'):
            if (text is None or node.get('text') == text) and (kind is None or node.get('class') == kind):
                return node
        raise AssertionError(f'Missing native control text={text!r} class={kind!r}')

    def tap(node):
        x0, y0, x1, y1 = map(int, re.findall(r'\d+', node.get('bounds')))
        assert x1 > x0 and y1 > y0
        adb('shell', 'input', 'tap', str((x0+x1)//2), str((y0+y1)//2))

    def type_text(text):
        encoded = base64.b64encode(text.encode()).decode()
        status = adb('shell', 'am', 'instrument', '-w', '-e', 'type_text_base64', encoded,
                     'dev.pax.android_probe/dev.pax.android_probe.Probe')
        assert 'INSTRUMENTATION_CODE: -1' in status and 'INSTRUMENTATION_RESULT: key_events=' in status, status

    def wait_first_frame(pid):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            log = adb('logcat', '-d', '--pid='+pid, '-s', 'Pax')
            if 'Android host failure' in log:
                raise AssertionError(log)
            if 'first frame activated' in log:
                return log
            time.sleep(0.25)
        raise AssertionError('No complete Vulkan frame within 30 seconds')

    adb('shell', 'am', 'force-stop', APP)
    adb('shell', 'am', 'start', '-W', '-n', ACTIVITY)
    pid = adb('shell', 'pidof', APP).strip()
    log = wait_first_frame(pid)

    find('Initializations: 1')
    button = find('Count: 0', 'android.widget.Button')
    assert int(re.findall(r'\d+', button.get('bounds'))[1]) > 100, 'Native bounds must include the visual translation'
    tap(button)
    find('Count: 1')
    tap(find(kind='android.widget.CheckBox'))
    find('Checked: yes')
    tap(find(kind='android.widget.EditText'))
    adb('shell', 'input', 'keyevent', 'KEYCODE_MOVE_END')
    type_text('-tested')
    find('Rust: Native text and IME-tested')
    type_text('-abcdefghijklmnopqrstuvwxyz0123456789')
    find('Rust: Native text and IME-tested-abcdefghijklmnopqrstuvwxyz0123456789')
    (args.output/'editing.png').write_bytes(adb('exec-out', 'screencap', '-p', binary=True))
    adb('shell', 'input', 'keyevent', 'KEYCODE_BACK')
    adb('shell', 'input', 'keyevent', 'KEYCODE_HOME')
    adb('shell', 'am', 'start', '-W', '-n', ACTIVITY)
    find('Count: 1')
    find('Native text and IME-tested-abcdefghijklmnopqrstuvwxyz0123456789', 'android.widget.EditText')

    original = {key: adb('shell', 'settings', 'get', 'system', key).strip()
                for key in ('accelerometer_rotation', 'user_rotation')}
    try:
        adb('shell', 'settings', 'put', 'system', 'accelerometer_rotation', '0')
        adb('shell', 'settings', 'put', 'system', 'user_rotation', '1')
        time.sleep(1)
        find('Initializations: 1')
        find('Count: 1')
        find('Native text and IME-tested-abcdefghijklmnopqrstuvwxyz0123456789', 'android.widget.EditText')
        (args.output/'recreated.png').write_bytes(adb('exec-out', 'screencap', '-p', binary=True))
        log = adb('logcat', '-d', '--pid='+pid, '-s', 'Pax')
        assert 'reattaching retained session' in log, 'Rotation did not exercise Activity recreation'
        assert log.count('explicit initialization complete') == 1
        assert 'Android host failure' not in log
    finally:
        for key, value in original.items():
            adb('shell', 'settings', 'delete' if value == 'null' else 'put', 'system', key, *([] if value == 'null' else [value]))
    # Each variant starts a fresh process so fixture initialization is explicit.
    def launch(mode):
        adb('shell', 'am', 'force-stop', APP)
        adb('shell', 'am', 'start', '-W', '-n', ACTIVITY, '--es', 'pax.startup', mode,
            '--ez', 'pax.diagnostics', 'true')
        wait_first_frame(adb('shell', 'pidof', APP).strip())

    def bounds(node):
        return list(map(int, re.findall(r'\d+', node.get('bounds'))))

    launch('uppercase')
    tap(find(kind='android.widget.EditText'))
    adb('shell','input','keyevent','KEYCODE_MOVE_END')
    type_text('-app')
    find('NATIVE TEXT AND IME-APP', 'android.widget.EditText')
    find('Rust: NATIVE TEXT AND IME-APP')

    launch('native-only')
    tap(find('Native count: 0', 'android.widget.Button'))
    find('Native count: 1')

    launch('choices')
    tap(find(kind='android.widget.Spinner'))
    tap(find('Second'))
    tap(find('Blue', 'android.widget.RadioButton'))
    find('Choice: 1, radio: 2')
    # Recreating native choice widgets must preserve selection without emitting
    # a default selection that overwrites the retained Rust properties.
    try:
        adb('shell','settings','put','system','accelerometer_rotation','0')
        adb('shell','settings','put','system','user_rotation','1')
        time.sleep(1)
        find('Choice: 1, radio: 2')
    finally:
        for key,value in original.items():
            adb('shell','settings','delete' if value=='null' else 'put','system',key,*([] if value=='null' else [value]))

    if args.remote_assets:
        launch('assets')
        asset_pid=adb('shell','pidof',APP).strip()
        deadline=time.monotonic()+45
        while time.monotonic()<deadline:
            asset_log=adb('logcat','-d','--pid='+asset_pid,'-s','Pax')
            assert 'Android host failure' not in asset_log, asset_log
            samples=[json.loads(raw) for raw in re.findall(r'presentation (\{[^\n]+)',asset_log)]
            if samples and samples[-1]['uploadedImageBytes']>=2*512*512*4:
                assert samples[-1]['assetFailures']==0, asset_log
                assert samples[-1]['loadedFonts']>=1, asset_log
                break
            time.sleep(0.5)
        else:
            raise AssertionError('Remote assets did not settle within 45 seconds')
        text=find('Remote Space Grotesk')
        assert bounds(text)[3]-bounds(text)[1]>20
        (args.output/'remote-assets.log').write_text(asset_log)
        (args.output/'remote-assets.png').write_bytes(adb('exec-out','screencap','-p',binary=True))

    launch('composition')
    button = find('Masked count: 0', 'android.widget.Button')
    x0, y0, x1, y1 = bounds(button)
    density = (x1-x0)/280
    adb('shell', 'input', 'tap', str(round(x0+130*density)), str(round(y0+50*density)))
    find('Masked count: 0')
    adb('shell', 'input', 'tap', str(round(x0+20*density)), str(round(y0+50*density)))
    find('Masked count: 1')
    textbox = find(kind='android.widget.EditText')
    bx0, _, bx1, _ = bounds(textbox)
    assert bx1-bx0 > 250*density, 'Rotated native bounds were clipped by an intermediate View'
    tap(textbox)
    adb('shell', 'input', 'keyevent', 'KEYCODE_MOVE_END')
    type_text('-rotated')
    find('Rust: Native text and IME-rotated')
    adb('shell', 'input', 'keyevent', 'KEYCODE_BACK')
    x0,y0,x1,y1 = bounds(find(kind='android.widget.SeekBar'))
    adb('shell', 'input', 'swipe', str(x0+(x1-x0)//10), str((y0+y1)//2),
        str(x1-(x1-x0)//10), str((y0+y1)//2), '600')
    level = next(n.get('text') for n in snapshot().iter('node') if n.get('text','').startswith('Level: '))
    assert float(level.split(': ')[1]) >= 8, level
    (args.output/'composition.png').write_bytes(adb('exec-out', 'screencap', '-p', binary=True))

    launch('scroll')
    scrollers = sorted([n for n in snapshot().iter('node') if n.get('class')=='android.widget.ScrollView'],
                       key=lambda n: bounds(n)[3]-bounds(n)[1])
    assert len(scrollers) == 2
    def swipe_up(node):
        x0,y0,x1,y1 = bounds(node)
        adb('shell', 'input', 'swipe', str((x0+x1)//2), str(y1-(y1-y0)//8),
            str((x0+x1)//2), str(y0+(y1-y0)//8), '500')
    for _ in range(4): swipe_up(scrollers[0])
    destination = find('Inner destination')
    assert bounds(destination)[3]-bounds(destination)[1] > 20*density
    find('Scroll: 0.00')
    # Start outer gestures below the nested scroller.
    outer = bounds(scrollers[1])
    bottom = None
    # Injected gesture distance varies with device load and native inertia.
    # Bound the search by gesture count, while asserting the actual destination.
    for _ in range(20):
        adb('shell','input','swipe',str((outer[0]+outer[2])//2),str(outer[3]-80),
            str((outer[0]+outer[2])//2),str((outer[1]+outer[3])//2),'450')
        candidates = [n for n in snapshot().iter('node') if n.get('text')=='Bottom increment']
        if candidates and bounds(candidates[0])[3]-bounds(candidates[0])[1] > 40*density:
            bottom = candidates[0]
            break
    assert bottom is not None, 'Outer scrolling did not reach its native button'
    tap(bottom)
    find('Count: 1')
    scroll_text = next(n.get('text') for n in snapshot().iter('node') if n.get('text','').startswith('Scroll: '))
    assert float(scroll_text.split(': ')[1]) > 1000, scroll_text
    # Recreate while scrolled, then restore portrait and check retained position.
    try:
        adb('shell','settings','put','system','accelerometer_rotation','0')
        adb('shell','settings','put','system','user_rotation','1')
        time.sleep(1)
        find('Count: 1')
        adb('shell','settings','put','system','user_rotation','0')
        time.sleep(1)
        find('Bottom increment')
    finally:
        for key,value in original.items():
            adb('shell','settings','delete' if value=='null' else 'put','system',key,*([] if value=='null' else [value]))
    (args.output/'scroll.png').write_bytes(adb('exec-out','screencap','-p',binary=True))
    time.sleep(3)
    scroll_log = adb('logcat','-d','--pid='+adb('shell','pidof',APP).strip(),'-s','Pax')
    assert 'Android host failure' not in scroll_log, scroll_log
    (args.output/'scroll.log').write_text(scroll_log)
    records = [json.loads(line.split('presentation ',1)[1]) for line in scroll_log.splitlines()
               if 'presentation ' in line]
    assert records, 'Missing bounded presentation diagnostics'
    actual = max(records[-1]['scrolls'], key=lambda scroll: scroll['contentHeight'])['y']
    displayed = next(n.get('text') for n in snapshot().iter('node') if n.get('text','').startswith('Scroll: '))
    assert abs(float(displayed.split(': ')[1])-actual) <= 1/density+0.005, (displayed,actual)
    assert records[-1]['pendingNativeBytes'] == 0 and records[-1]['retired'] == 0, records[-1]


    launch('cross-axis')
    rows = [n for n in snapshot().iter('node') if n.get('class')=='android.widget.HorizontalScrollView']
    row = min(rows, key=lambda n: bounds(n)[3]-bounds(n)[1])
    x0,y0,x1,y1 = bounds(row)
    destination = None
    # Gesture distance varies with emulator load and native fling scheduling.
    # Require the real destination within a bounded number of drags.
    for _ in range(12):
        destination = next((node for node in snapshot().iter('node')
                            if node.get('text') == 'Row destination'), None)
        if destination is not None:
            break
        adb('shell','input','swipe',str(x1-(x1-x0)//8),str((y0+y1)//2),
            str(x0+(x1-x0)//8),str((y0+y1)//2),'500')
    if destination is None:
        destination = find('Row destination')
    tap(destination)
    find('Count: 1')
    find('Scroll: 0.00')
    # A vertical drag starts inside a horizontal-only child, as in the website
    # gallery. The child's native wrappers must release the vertical parent.
    adb('shell','input','swipe',str((x0+x1)//2),str(y1-(y1-y0)//8),
        str((x0+x1)//2),str(y0+(y1-y0)//8),'700')
    scroll_text = next(n.get('text') for n in snapshot().iter('node') if n.get('text','').startswith('Scroll: '))
    assert float(scroll_text.split(': ')[1]) > 50, scroll_text
    (args.output/'cross-axis.png').write_bytes(adb('exec-out','screencap','-p',binary=True))

    # Clearing the task finishes the old Activity; force-stop would bypass close.
    launch('')
    find('Initializations: 1')
    pid = adb('shell','pidof',APP).strip()
    adb('shell','am','start','-W','-f','0x10008000','-n',ACTIVITY)
    find('Initializations: 2')
    deadline = time.monotonic()+15
    while time.monotonic()<deadline:
        close_log = adb('logcat','-d','--pid='+pid,'-s','Pax')
        if 'session drained' in close_log: break
        time.sleep(0.2)
    else: raise AssertionError('Activity finish did not drain its native session')
    assert 'Android host failure' not in close_log, close_log
    (args.output/'close.log').write_text(close_log)

    (args.output/'pax.log').write_text(log)
    result = {'package': APP, 'status': 'passed', 'completed_at': time.time(), 'checks': [
        'complete Vulkan frame', 'native button event', 'checkbox binding',
        'rapid EditText input, Rust echo, and application transformation', 'background/resume state',
        'Activity recreation retains initializer and application state',
        'native-only scene', 'dropdown and radio binding across recreation',
        'opaque mask hit rejection', 'rotated native editing and bounds', 'slider binding',
        'nested native scrolling and bound outer position',
        'horizontal scrolling and vertical parent handoff', 'Activity finish drains session'],
        'device': adb('shell', 'getprop', 'ro.build.fingerprint').strip()}
    if args.remote_assets:
        result['checks'].append('HTTPS images and CSS font loading')
    (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
