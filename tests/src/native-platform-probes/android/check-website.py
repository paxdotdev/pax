#!/usr/bin/env python3
"""Exercise the installed website through Android UI; retain failure evidence.

Performance targets are opt-in and require a physical device. Emulator runs
still enforce functional checks and bounded resource ownership.
"""
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

APP = 'dev.pax.pax_website'
ACTIVITY = APP + '/dev.pax.PaxActivity'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--adb', default=str(Path(os.environ.get('ANDROID_HOME', ''))/'platform-tools/adb'))
    parser.add_argument('--cycles', type=int, default=3)
    parser.add_argument('--lifecycle-cycles', type=int, default=1)
    parser.add_argument('--qualify-performance', action='store_true')
    parser.add_argument('--paxflix-presentation', choices=['overlay', 'routed'], default='overlay')
    args = parser.parse_args()
    assert 1 <= args.cycles <= 20
    assert 1 <= args.lifecycle_cycles <= 20
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output/'result.json').unlink(missing_ok=True)
    started = time.monotonic()
    result = {'checks': [], 'failures': [], 'memoryPssKiB': [], 'memorySamples': [],
              'presentation': [], 'paxflixPresentation': args.paxflix_presentation, 'demoCycles': args.cycles, 'lifecycleCycles': args.lifecycle_cycles}
    pid = ''

    def adb(*command, binary=False):
        return subprocess.check_output([args.adb, *command], text=not binary, timeout=40)

    def bounds(node):
        return list(map(int, re.findall(r'-?\d+', node.get('bounds'))))

    def snapshot():
        for _ in range(4):
            # Recreation can temporarily have no accessibility root. Never read
            # a stale dump from the previous Activity and count that as success.
            status = adb('shell', 'am', 'instrument', '-w', 'dev.pax.android_probe/dev.pax.android_probe.Probe')
            timings = dict((key, int(value)) for key, value in re.findall(
                r'INSTRUMENTATION_RESULT: (root_ms|tree_ms|node_count)=(\d+)', status))
            if timings:
                inspection = result.setdefault('inspection', {})
                for key, value in timings.items():
                    inspection[key+'_max'] = max(value, inspection.get(key+'_max', 0))
            encoded = re.search(r'INSTRUMENTATION_RESULT: hierarchy_gzip=([^\r\n]+)',status)
            if not encoded:
                if 'Unable to find instrumentation' in status:
                    raise AssertionError('Install the UI helper with build-ui-probe.py --install first')
                time.sleep(0.5)
                continue
            xml = gzip.decompress(base64.b64decode(encoded[1])).decode()
            (args.output/'last-ui.xml').write_text(xml)
            return list(ET.fromstring(xml).iter('node'))
        raise AssertionError('Android accessibility hierarchy remained unavailable')

    def matches(text):
        labels = {text} if isinstance(text, str) else set(text)
        return [n for n in snapshot() if n.get('text','').strip() in labels and n.get('visible')=='true' and bounds(n)[3]-bounds(n)[1] > 15]

    def wait_for(text, present=True, timeout=30):
        deadline = time.monotonic()+timeout
        while time.monotonic() < deadline:
            found = matches(text)
            if bool(found) == present:
                return found[0] if found else None
            time.sleep(0.3)
        raise AssertionError(f'{text!r} did not become {"visible" if present else "hidden"}')

    def tap(node):
        x0, y0, x1, y1 = bounds(node)
        adb('shell', 'input', 'tap', str((x0+x1)//2), str((y0+y1)//2))

    def image(name):
        (args.output/(name+'.png')).write_bytes(adb('exec-out', 'screencap', '-p', binary=True))

    def log():
        return adb('logcat', '-d', '--pid='+pid, '-s', 'Pax', 'AndroidRuntime')

    def rotate(value):
        previous = log().count('reattaching retained session')
        adb('shell', 'settings', 'put', 'system', 'user_rotation', str(value))
        deadline = time.monotonic()+30
        while time.monotonic() < deadline:
            if log().count('reattaching retained session') > previous:
                return
            time.sleep(0.25)
        raise AssertionError(f'Rotation {value} did not recreate the Activity within 30 seconds')

    def memory(name):
        text = adb('shell', 'dumpsys', 'meminfo', APP)
        (args.output/(name+'-memory.txt')).write_text(text)
        match = re.search(r'TOTAL PSS:\s+(\d+)', text)
        assert match, 'No memory sample from the website process'
        value = int(match[1])
        result['memoryPssKiB'].append(value)
        result['memorySamples'].append({'checkpoint': name, 'pssKiB': value,
                                        'elapsedSeconds': round(time.monotonic()-started, 2)})
        return value

    def seek(text, limit=60, forward=True):
        # The left margin belongs to the page, outside embedded app scrollers.
        for _ in range(limit):
            found = matches(text)
            if found:
                return found[0]
            start, end = (0.84, 0.43) if forward else (0.43, 0.64)
            adb('shell', 'input', 'swipe', str(round(width*0.025)), str(round(height*start)),
                str(round(width*0.025)), str(round(height*end)), '700')
        raise AssertionError(f'Could not reach {text!r} within {limit} page gestures')

    original = {key: adb('shell', 'settings', 'get', 'system', key).strip()
                for key in ('accelerometer_rotation', 'user_rotation')}
    result['device'] = {key: adb('shell', 'getprop', key).strip() for key in
                        ('ro.product.model', 'ro.build.version.sdk', 'ro.build.fingerprint', 'ro.kernel.qemu')}
    try:
        if args.qualify_performance:
            assert result['device']['ro.kernel.qemu'] != '1', 'Physical-device performance qualification cannot run on an emulator'
        adb('shell', 'settings', 'put', 'system', 'accelerometer_rotation', '0')
        adb('shell', 'settings', 'put', 'system', 'user_rotation', '0')
        size = adb('shell', 'wm', 'size')
        width, height = map(int, re.findall(r'(\d+)x(\d+)', size)[-1])
        adb('shell', 'am', 'force-stop', APP)
        adb('shell', 'am', 'start', '-W', '-n', ACTIVITY, '--ez', 'pax.diagnostics', 'true')
        pid = adb('shell', 'pidof', APP).strip()
        deadline = time.monotonic()+45
        while time.monotonic() < deadline:
            text = log()
            assert 'Android host failure' not in text, text
            first = re.search(r'first frame activated (\{[^\n]+)', text)
            if first:
                result['startup'] = json.loads(first[1])
                break
            time.sleep(0.5)
        else:
            raise AssertionError('Website did not present a first frame within 45 seconds')
        result['checks'].append('first frame')
        body = wait_for('This website is built in Pax.')
        assert bounds(body)[3]-bounds(body)[1] > 20, 'Autosized native text collapsed'
        image('hero')
        result['checks'].append('intrinsic text sizing')
        tap(wait_for('View Source'))
        wait_for('src/hero_section.pax')
        source = snapshot()
        code = [n for n in source if '<Text' in n.get('text', '') and '\n' in n.get('text', '')]
        assert code, 'Source drawer has no decoded Pax code'
        assert not any('<pre data-pax-code-markup' in n.get('text', '') for n in code), 'Source drawer displays markup literally'
        assert any(len(set(n.get('span-colors', '').strip(',').split(','))) >= 3
                   and not n.get('span-fonts', '') for n in code), 'Source drawer lost token colors or overrode its authored font'
        result['checks'].append('source drawer decoded text / token colors / authored font inheritance')
        image('source')
        tap(wait_for('x'))
        wait_for('View Source')
        routed = args.paxflix_presentation == 'routed'
        launcher = ('TRY PAXFLIX  ↗', 'OPEN PAXFLIX FULL SCREEN  ↗') if routed else 'Open Paxflix full screen ↗'
        close_label = '← BACK TO PAX' if routed else '× Close demo'
        title = 'THE LAST' if routed else 'Your next great watch'
        for cycle in range(args.cycles):
            tap(seek(launcher))
            wait_for(close_label)
            wait_for(title)
            if cycle == 0:
                image('paxflix')
                # Preserve the active demo through Activity recreation, whether
                # the website presents it as an overlay or a native route.
                adb('shell', 'input', 'keyevent', 'KEYCODE_HOME')
                time.sleep(2)
                adb('shell', 'am', 'start', '-W', '-n', ACTIVITY)
                wait_for(close_label)
                wait_for(title)
                rotate(1)
                rotate(0)
                wait_for(close_label)
                wait_for(title)
                assert adb('shell', 'pidof', APP).strip() == pid, 'Paxflix recreation restarted the process'
                assert log().count('explicit initialization complete') == 1, 'Paxflix recreation repeated user initialization'
                result['checks'].append(f'Paxflix {args.paxflix_presentation} retained across background / resume / recreation')
                # Accessibility can expose a button covered by a system bar.
                # Exercise the actual return target as well as Android Back.
                tap(wait_for(close_label))
                wait_for(close_label, present=False)
                tap(seek(launcher))
                wait_for(close_label)
                wait_for(title)
            adb('shell', 'input', 'keyevent', 'KEYCODE_BACK')
            wait_for(close_label, present=False)
            seek(launcher)
            assert adb('shell', 'pidof', APP).strip() == pid, 'Back destroyed the website instead of returning from Paxflix'
            time.sleep(2)
            memory(f'demo-cycle-{cycle+1}')
        result['checks'].append(f'Paxflix {args.paxflix_presentation} return target / repeated open / Back cycles')
        for cycle in range(args.lifecycle_cycles):
            adb('shell', 'input', 'keyevent', 'KEYCODE_HOME')
            time.sleep(2)
            adb('shell', 'am', 'start', '-W', '-n', ACTIVITY)
            wait_for(launcher)
            # Wait for each actual configuration change before reversing it; a
            # fixed delay can collapse the two requests under rendering/IME load.
            rotate(1)
            rotate(0)
            wait_for(launcher)
            assert adb('shell', 'pidof', APP).strip() == pid, 'Lifecycle cycle restarted the process'
            assert log().count('explicit initialization complete') == 1, 'Recreation repeated user initialization'
            time.sleep(2)
            memory(f'lifecycle-cycle-{cycle+1}')
        result['checks'].append('background / resume / recreation')
        seek('Meet your creative toolkit')
        image('gallery')
        seek('Your next interface starts here.')
        image('resources')
        seek('enter')
        tap(wait_for('CALCULATE'))
        for key in ('clear','7','+','8','enter'):
            # The upper '+' controls graph zoom; arithmetic '+' is in the
            # bottom keyboard row. Use actual native bounds after each action.
            choices=matches(key)
            assert choices, f'Calculator key {key!r} is not visible'
            tap(max(choices,key=lambda node: bounds(node)[1]))
        # A phone cannot show the entire calculator and keyboard together.
        # Bring the LCD history back below the site's fixed header after Enter.
        seek('15', limit=6, forward=False)
        wait_for('›')
        image('calculator')
        result['checks'].append('embedded Calculator: 7 + 8 = 15')
        seek('Early, open, and ready to build with.')
        image('footer')
        result['checks'].append('full website traversal')
        time.sleep(3)
        memory('full-traversal')
        lines = log()
        assert 'Android host failure' not in lines and 'FATAL EXCEPTION' not in lines, lines
        result['presentation'] = [json.loads(v) for v in re.findall(r'presentation (\{[^\n]+)', lines)]
        assert result['presentation'], 'No bounded presentation counters were emitted'
        for sample in result['presentation']:
            assert sample['pendingNativeBytes'] <= 16*1024*1024
            assert sample['gpu']['windows'] <= 64
            assert sample['decodedImageBytes'] <= 64*1024*1024
        result['checks'].append('bounded native / GPU / image queues')
        if args.qualify_performance:
            assert result['startup']['elapsedMs'] <= 3000, 'Activity-to-first-frame target: 3 seconds'
            assert max(result['memoryPssKiB']) <= 512*1024, 'Memory target: 512 MiB PSS'
            assert max(result['memoryPssKiB'][:args.cycles])-min(result['memoryPssKiB'][:args.cycles]) <= 32*1024, 'Repeated-demo growth target: 32 MiB'
            for sample in result['presentation'][2:]:
                assert sample['paxFrameMs'][1] <= 8, 'Pax UI-thread p95 target: 8 ms'
                assert sample['windowFrameMs'][1] <= 16.7, 'Window p95 target at 60 Hz: 16.7 ms'
    except BaseException as error:
        result['failures'].append(str(error) or type(error).__name__)
        raise
    finally:
        result['elapsedSeconds'] = round(time.monotonic()-started, 2)
        for key, value in original.items():
            adb('shell', 'settings', 'delete' if value == 'null' else 'put', 'system', key,
                *([] if value == 'null' else [value]))
        if pid:
            lines = log()
            (args.output/'logcat.txt').write_text(lines)
            result['presentation'] = [json.loads(v) for v in re.findall(r'presentation (\{[^\n]+)', lines)]
            image('final')
        (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    assert not result['failures'], result['failures']
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
