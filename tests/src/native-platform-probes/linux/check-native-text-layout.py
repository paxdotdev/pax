#!/usr/bin/env python3
"""Compare the shared Text fixture's engine snapshots with GTK allocations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
from native_process import NativeProcess, add_launch_arguments


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('app', type=Path)
    add_launch_arguments(parser)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--check-text-ink', action='store_true',
                        help='Require opt-in GTK/Pango diagnostics and check this untransformed fixture for clipped text')
    font_mode = parser.add_mutually_exclusive_group()
    font_mode.add_argument('--live-font-dpi', action='store_true',
                           help='Use a temporary, otherwise unowned XSettings source for live 96→144→96 font-DPI changes')
    font_mode.add_argument('--cold-font-dpi', type=int, choices=[144],
                           help='Launch at the specified font DPI without live changes, then restore the unowned display')
    font_mode.add_argument('--portal-font-scale', action='store_true',
                           help='Use the installed GTK portal and isolated real GSettings for live 100→150→100 percent changes')
    parser.add_argument('--wayland', action='store_true',
                        help='Require a native Wayland display and drive genuine controls through AT-SPI')
    args = parser.parse_args()
    if args.wayland and (args.live_font_dpi or args.cold_font_dpi):
        parser.error('XSettings font-DPI options require X11')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report, records = out / 'state.json', out / 'layout.log'
    capture = out / 'capture.png'
    checks, snapshots = [], {}
    completed, failure = False, None
    log = (out / 'app.log').open('w')
    env = dict(os.environ, GSK_RENDERER='ngl', PAX_NATIVE_TEST_REPORT=str(report),
               PAX_TEXT_LAYOUT_LOG=str(records), PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    if args.wayland:
        env.update(GDK_BACKEND='wayland', GTK_A11Y='atspi')
        env.pop('DISPLAY', None)
    if args.portal_font_scale:
        env['GTK_USE_PORTAL'] = '1'
    settings = None
    try:
        if args.live_font_dpi or args.cold_font_dpi:
            from xsettings_test import XSettingsFontDpi
            settings = XSettingsFontDpi(args.cold_font_dpi or 96)
        elif args.portal_font_scale:
            from portal_font_scale import PortalFontScale
            settings = PortalFontScale(out)
        app = NativeProcess(args, env, log, out)
    except BaseException:
        if settings is not None:
            settings.close()
        log.close()
        raise

    def check(ok, name):
        checks.append({'check': name, 'passed': bool(ok)})
        print('PASS' if ok else 'FAIL', name, flush=True)

    def state():
        if settings is not None:
            settings.check_owner()
        if app.poll() is not None:
            raise RuntimeError(f'Fixture exited {app.returncode}')
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(predicate, label, seconds=15):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            value = state()
            if predicate(value):
                return value
            time.sleep(.05)
        raise RuntimeError('Timeout: ' + label)

    def xdo(*args):
        return subprocess.check_output(['xdotool', *map(str, args)], text=True, timeout=10).strip()

    def click(text):
        if args.wayland:
            import pyatspi
            desktop = pyatspi.Registry.getDesktop(0)
            native = next((a for a in desktop if app.matches_accessible(a)), None)
            if native is None:
                raise RuntimeError('Exact application is missing from AT-SPI')
            controls = []
            def visit(node, depth=0):
                if depth > 24 or len(controls) > 512:
                    return
                controls.append(node)
                for child in node:
                    visit(child, depth + 1)
            visit(native)
            control = next(a for a in controls if a.getRoleName() == 'push button' and a.name == text)
            if not control.queryAction().doAction(0):
                raise RuntimeError('Native Wayland button action failed: ' + text)
            time.sleep(.5)
            return
        node = next(n for n in state()['nodes'] if n['kind'] == 'Button' and n.get('text') == text)
        x, y, w, h = node['bounds']
        xdo('mousemove', '--window', window, round(x + w / 2), round(y + h / 2))
        xdo('click', 1)
        time.sleep(.5)

    pattern = re.compile(r'pax-text-layout: name=(\w+) id=(\d+) width=([-\d.e]+) height=([-\d.e]+) x=([-\d.e]+) y=([-\d.e]+).* expanded=(true|false) narrow=(true|false)')

    def snapshot(name, expanded, narrow):
        time.sleep(.5)
        previous = records.read_text() if records.exists() else ''
        click('Record layout')
        end = time.monotonic() + 10
        nodes = {}
        while time.monotonic() < end:
            fresh = records.read_text()[len(previous):] if records.exists() else ''
            nodes = {m[1]: {'id': int(m[2]), 'width': float(m[3]), 'height': float(m[4]),
                           'x': float(m[5]), 'y': float(m[6]), 'expanded': m[7] == 'true',
                           'narrow': m[8] == 'true'} for m in pattern.finditer(fresh)}
            if len(nodes) == 10:
                break
            time.sleep(.05)
        check(len(nodes) == 10, f'{name}: all ten engine nodes observed')
        if len(nodes) != 10:
            raise RuntimeError('Incomplete engine snapshot')
        native = state()
        origin = native.get('sceneBoundsInSurface', [0, 0])[:2]
        if args.wayland and 'sceneBoundsInSurface' not in native:
            raise RuntimeError('Wayland layout comparisons require the measured scene surface origin')
        by_id = {n['id']: n for n in native['nodes']}
        check(all(n['expanded'] == expanded and n['narrow'] == narrow for n in nodes.values()), f'{name}: requested content and width state')
        for key in ['intrinsic', 'fixed', 'paragraph', 'stack_text', 'frame_text', 'repeat_text_0', 'repeat_text_1']:
            n = nodes[key]
            gtk = by_id.get(n['id'])
            check(gtk is not None and gtk['kind'] == 'Text', f'{name}: {key} is a real GTK Text')
            if gtk is not None:
                bounds = list(gtk['bounds'])
                bounds[0] -= origin[0]
                bounds[1] -= origin[1]
                check(all(abs(a-b) <= 1.1 for a, b in zip(bounds, [n['x'], n['y'], n['width'], n['height']])), f'{name}: {key} GTK allocation agrees with engine')
                if args.check_text_ink:
                    layout = gtk.get('textLayout')
                    check(isinstance(layout, dict), f'{name}: {key} exposes GTK text layout diagnostics')
                    if isinstance(layout, dict):
                        x, y, width, height = layout['inkBounds']
                        # This fixed-height, default clip=false control is the
                        # deliberate overflow case at enlarged font DPI.
                        fixed_overflow = key == 'fixed' and native.get('gtkFontDpi', 0) > 96 * 1024
                        if not fixed_overflow:
                            check(x >= -1 and y >= -1 and x + width <= n['width'] + 1 and y + height <= n['height'] + 1,
                                  f'{name}: {key} glyph ink fits engine bounds')
                            check(layout['controlWidth'] <= n['width'] + 1 and layout['controlHeight'] <= n['height'] + 1,
                                  f'{name}: {key} child allocation fits engine bounds')
                        check(layout['unknownGlyphs'] == 0 and not layout['ellipsized'] and layout['lineCount'] > 0,
                              f'{name}: {key} has complete known glyphs without ellipsis')
            check(n['width'] > 0 and n['height'] > 0, f'{name}: {key} has positive dimensions')
        a, b = nodes['repeat_text_0'], nodes['repeat_text_1']
        check(a['id'] != b['id'] and abs(a['height']-b['height']) <= 1 and abs(a['width']-b['width']) <= 1, f'{name}: repeated nodes measure independently and agree')
        check(abs(nodes['fixed']['height'] - 30) <= 1, f'{name}: explicit height stays fixed')
        check(abs(nodes['follower']['y'] - nodes['stack_text']['y'] - nodes['stack_text']['height'] - 12) <= 1, f'{name}: Stacker follower tracks text height and gutter')
        check(abs(nodes['frame']['height'] - nodes['frame_text']['height'] - 24) <= 1, f'{name}: Frame height includes both padding edges')
        snapshots[name] = {'engine': nodes, 'native': native}
        (out / f'{name}.json').write_text(json.dumps(snapshots[name], indent=2)+'\n')
        Path(str(capture)+'.request').touch()
        end = time.monotonic()+10
        while time.monotonic()<end and Path(str(capture)+'.request').exists():
            time.sleep(.05)
        if capture.exists():
            capture.rename(out / f'{name}.png')
        return nodes

    def process_start_ticks():
        return Path(f'/proc/{app.pid}/stat').read_text().rsplit(')', 1)[1].split()[19]

    def live_change(dpi, previous, direction):
        # Observe a static app before requesting the engine snapshot; the
        # Record layout click must not be what triggers remeasurement.
        time.sleep(1)
        before = state()
        intrinsic_id = previous['intrinsic']['id']
        old_height = next(n for n in before['nodes'] if n['id'] == intrinsic_id)['bounds'][3]
        settings.set_dpi(dpi)

        def changed(value):
            intrinsic = next((n for n in value.get('nodes', []) if n['id'] == intrinsic_id), None)
            return (value.get('gtkFontDpi') == dpi * 1024 and intrinsic is not None and
                    (intrinsic['bounds'][3] - old_height) * direction > 1 and
                    value.get('textMetricsInvalidations', 0) > before.get('textMetricsInvalidations', 0))

        observed = wait(changed, f'external font DPI {dpi} changes static native text before Record layout')
        (out / f'live-{dpi}-before-record.json').write_text(json.dumps(observed, indent=2)+'\n')
        check(process_start_ticks() == start_ticks, f'live {dpi}: same running process')
        check(observed['widgetScaleFactor'] == before['widgetScaleFactor'], f'live {dpi}: widget scale factor unchanged')
        check(True, f'live {dpi}: native text changed before any content, width, resize or snapshot click')
        current = snapshot(f'live-{dpi}', False, False)
        for key in ['intrinsic', 'paragraph', 'stack_text', 'frame_text', 'repeat_text_0', 'repeat_text_1']:
            check((current[key]['height'] - previous[key]['height']) * direction > 1,
                  f'live {dpi}: {key} auto height follows font metrics')
        return current

    try:
        wait(lambda s: any(n.get('text') == 'Record layout' for n in s.get('nodes', [])), 'native fixture controls')
        if args.wayland:
            check(state().get('displayBackend') == 'GdkWaylandDisplay', 'native Wayland display without X11 fallback')
        else:
            window = app.x11_window(title='Pax native text layout', only_visible=True)
        maps = Path(f'/proc/{app.pid}/maps').read_text()
        libraries = sorted({line.split()[-1] for line in maps.splitlines() if 'libpax_host.so' in line})
        cartridges = sorted({line.split()[-1] for line in maps.splitlines() if 'libnative_text_layout.so' in line})
        mounted_root = Path(f'/proc/{app.pid}/root')
        library_hash = lambda name: hashlib.sha256((mounted_root / name.lstrip('/')).read_bytes()).hexdigest()
        identity = {'pid': app.pid, 'app': str(args.app.resolve()), 'appSha256': hashlib.sha256(args.app.read_bytes()).hexdigest(),
                    'loadedAppSha256': hashlib.sha256(Path(f'/proc/{app.pid}/exe').read_bytes()).hexdigest(),
                    'hostLibraries': {name: library_hash(name) for name in libraries},
                    'cartridgeLibraries': {name: library_hash(name) for name in cartridges},
                    'checkerSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    'processHelperSha256': hashlib.sha256(Path(__file__).with_name('native_process.py').read_bytes()).hexdigest()}
        if settings is not None:
            helper = 'portal_font_scale.py' if args.portal_font_scale else 'xsettings_test.py'
            identity['settingsHelperSha256'] = hashlib.sha256(Path(__file__).with_name(helper).read_bytes()).hexdigest()
        if identity['appSha256'] != identity['loadedAppSha256']:
            raise RuntimeError('Loaded executable differs from the intended package')
        (out / 'identity.json').write_text(json.dumps(identity, indent=2)+'\n')
        start_ticks = process_start_ticks()
        check(len(libraries) == 1, 'one exact loaded GTK host identified')
        if not args.wayland:
            xdo('windowsize', '--sync', window, 1000, 1000)
            xdo('windowfocus', '--sync', window)
        initial = snapshot('short-wide', False, False)
        if args.cold_font_dpi:
            check(state().get('gtkFontDpi') == args.cold_font_dpi * 1024,
                  f'cold launch GTK font DPI is exactly {args.cold_font_dpi}')
        if args.live_font_dpi or args.portal_font_scale:
            check(state().get('gtkFontDpi') == 96 * 1024, 'initial GTK font DPI is exactly 96')
            enlarged = live_change(144, initial, 1)
            restored = live_change(96, enlarged, -1)
            check(all(abs(restored[key][axis] - initial[key][axis]) <= 1
                      for key in initial for axis in ['width', 'height']),
                  'restored font DPI returns all initial engine dimensions')
        click('Toggle content')
        long = snapshot('long-wide', True, False)
        check(long['paragraph']['height'] > initial['paragraph']['height'], 'content growth reflows paragraph')
        check(long['stack']['height'] > initial['stack']['height'], 'content growth expands Stacker')
        check(long['repeat_text_0']['height'] > initial['repeat_text_0']['height'], 'content growth expands repeated Text')
        click('Toggle width')
        narrow = snapshot('long-narrow', True, True)
        check(narrow['paragraph']['height'] > long['paragraph']['height'], 'narrow width grows paragraph height')
        check(abs(narrow['paragraph']['width']-180) <= 1, 'paragraph honors explicit narrow width')
        click('Toggle content')
        short = snapshot('short-narrow', False, True)
        check(short['paragraph']['height'] < narrow['paragraph']['height'], 'shorter content shrinks paragraph')
        app.terminate()
        check(app.wait(timeout=20) == 0, 'fixture closes normally')
        log.flush()
        text = (out / 'app.log').read_text()
        check('PAX_LINUX_CLOSED' in text and 'gl_refs=0 gl_bytes=0' in text, 'owned GL publications retired')
        completed = True
    except Exception as error:
        failure = str(error)
        raise
    finally:
        try:
            if app.poll() is None:
                app.terminate()
                app.wait(timeout=20)
        finally:
            if settings is not None:
                try:
                    settings.close()
                except Exception as error:
                    failure = f'{failure + "; " if failure else ""}settings restoration failed: {error}'
                    completed = False
            log.close()
            (out / 'checks.json').write_text(json.dumps({'completed': completed, 'passed': completed and all(c['passed'] for c in checks), 'error': failure, 'pid': app.pid, 'exitCode': app.poll(), 'app': str(args.app.resolve()), 'checks': checks}, indent=2)+'\n')
    if not completed or failure or not all(c['passed'] for c in checks):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
