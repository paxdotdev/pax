#!/usr/bin/env python3
"""Qualify a running website Debug session, restoring its temporary source edit.

Start pax-cli run --target android --path pax-website first. The UI helper from
build-ui-probe.py must be installed; no other device probe may run concurrently.
"""
import argparse
import base64
import gzip
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import time
import xml.etree.ElementTree as ET


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, default=Path('target/debug/pax-cli'))
    parser.add_argument('--project', type=Path, default=Path('pax-website'))
    parser.add_argument('--adb', default=str(Path(os.environ.get('ANDROID_HOME', ''))/'platform-tools/adb'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output/'result.json').unlink(missing_ok=True)
    result = {'checks': [], 'failures': []}
    source = args.project/'src/hero_section.pax'
    original = source.read_text()
    before = 'This website is built in Pax.'
    after = 'Android template reload is live.'
    assert original.count(before) == 1, 'The website hero changed; update this probe deliberately'
    patched = original.replace(before, after)
    original_file = args.output/'hero-original.pax'
    patched_file = args.output/'hero-reload.pax'
    original_file.write_text(original)
    patched_file.write_text(patched)

    def adb(*command):
        return subprocess.check_output([args.adb, *command], text=True, timeout=40)

    def cli(*command, expect_error=None):
        process = subprocess.run([str(args.cli.resolve()), 'dev', *command,
                                  '--path', str(args.project.resolve())],
                                 capture_output=True, text=True, timeout=45)
        if expect_error:
            assert process.returncode != 0 and expect_error in process.stderr, process.stderr
            return
        assert process.returncode == 0, process.stdout+process.stderr
        return json.loads(process.stdout)

    def visible(text):
        deadline = time.monotonic()+30
        while time.monotonic() < deadline:
            status = adb('shell', 'am', 'instrument', '-w', 'dev.pax.android_probe/dev.pax.android_probe.Probe')
            encoded = re.search(r'INSTRUMENTATION_RESULT: hierarchy_gzip=([^\r\n]+)', status)
            if encoded:
                xml = gzip.decompress(base64.b64decode(encoded[1])).decode()
                if any(node.get('text') == text for node in ET.fromstring(xml).iter('node')):
                    return
            time.sleep(0.3)
        raise AssertionError(f'Native text did not update to {text!r}')

    def look(name, *extra):
        captured = cli('look', '--output-dir', str(args.output/name), '--timeout-ms', '30000', *extra)
        assert captured['captures'], captured
        for frame in captured['captures']:
            png = Path(frame['path']).read_bytes()
            assert png[:8] == b'\x89PNG\r\n\x1a\n'
            dimensions = struct.unpack('>II', png[16:24])
            assert dimensions == (frame['width'], frame['height']) and min(dimensions) > 100
        (args.output/(name+'.json')).write_text(json.dumps(captured, indent=2)+'\n')
        return captured

    edited = False
    try:
        session = cli('status')
        assert session['platform'] == 'android' and session['designtime'], session
        result['session'] = session['session_id']
        pid = adb('shell', 'pidof', 'dev.pax.pax_website').strip()
        assert pid
        visible(before)
        for name, command in [
            ('tree', ('inspect', 'tree', '--max-depth', '4')),
            ('selector', ('selector', '#hero_content')),
            ('ray-cast', ('ray-cast', '--x', '100', '--y', '300')),
        ]:
            response = cli(*command, '--timeout-ms', '30000')
            assert response['node_count'] > 0, response
            (args.output/(name+'.json')).write_text(json.dumps(response, indent=2)+'\n')
            result['checks'].append(name)
        look('window')
        sequence = look('sequence', '--scale', '0.5', '--period-ms', '500', '--duration-ms', '3000')
        assert len(sequence['captures']) >= 2
        cli('look', '--duration-ms', '60001', '--period-ms', '500', '--timeout-ms', '30000',
            expect_error='duration <= 60000')
        result['checks'].append('window capture, sequence, and duration limit')
        edited = True
        cli('touch', 'apply-component-source', '--component', 'HeroSection', '--source-file', str(patched_file))
        visible(after)
        assert adb('shell', 'pidof', 'dev.pax.pax_website').strip() == pid
        look('reloaded')
        result['checks'].append('template reload updates native text without restarting')
    except BaseException as error:
        result['failures'].append(str(error) or type(error).__name__)
        raise
    finally:
        try:
            if edited:
                current = source.read_text()
                if current == patched:
                    try:
                        cli('touch', 'apply-component-source', '--component', 'HeroSection', '--source-file', str(original_file))
                    finally:
                        # Even a lost dev connection must not leave test prose
                        # in the application. Preserve an unexpected user edit.
                        if source.read_text() == patched:
                            temporary = source.with_name(f'.hero-restore-{os.getpid()}.pax')
                            temporary.write_text(original)
                            temporary.replace(source)
                    visible(before)
                elif current != original:
                    raise AssertionError(f'Concurrent edit preserved; original saved at {original_file}')
        except BaseException as error:
            result['failures'].append('Restore: '+(str(error) or type(error).__name__))
            raise
        finally:
            (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
