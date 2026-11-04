#!/usr/bin/env python3
"""Qualify orthographic spatial-workbench native input; screenshots require visual review."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--profile', choices=('debug', 'release'), default='debug')
    parser.add_argument('--edge-on', action='store_true', help='also exercise exact edge-on tilt and recovery')
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[4]
    fixture = repository/'examples/fixtures/spatial-workbench/.pax'
    target = 'dev.pax.spatial_workbench'
    probe = 'SpatialProbe'
    host = fixture/f'build/android/{args.profile}/intermediates/host.jar'
    sdk = Path(os.environ['ANDROID_HOME'])
    java = Path(os.environ['JAVA_HOME'])/'bin'
    tools = sdk/'build-tools/36.0.0'
    platform = sdk/'platforms/android-36/android.jar'
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=True)

    def run(*command):
        subprocess.run([str(c) for c in command], check=True, timeout=90)

    classes = out/'classes'; classes.mkdir(exist_ok=True)
    run(java/'javac', '--release', '8', '-classpath', os.pathsep.join(map(str, (platform, host))),
        '-d', classes, Path(__file__).parent/f'spatial-probe/{probe}.java')
    jar = out/'probe.jar'
    with zipfile.ZipFile(jar, 'w') as archive:
        for file in classes.rglob('*.class'):
            archive.write(file, file.relative_to(classes))
    run(tools/'d8', '--min-api', '29', '--lib', platform, '--classpath', host, '--output', out, jar)
    manifest = out/'AndroidManifest.xml'
    manifest.write_text(f'''<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="dev.pax.android_spatial_probe">
      <uses-sdk android:minSdkVersion="29" android:targetSdkVersion="36"/>
      <application android:label="Pax spatial probe" android:testOnly="true"/>
      <instrumentation android:name="dev.pax.{probe}" android:targetPackage="{target}"/>
    </manifest>''')
    apk = out/'unsigned.apk'
    run(tools/'aapt2', 'link', '-o', apk, '--manifest', manifest, '-I', platform)
    with zipfile.ZipFile(apk, 'a', compression=zipfile.ZIP_DEFLATED) as archive:
        archive.write(out/'classes.dex', 'classes.dex')
    aligned = out/'aligned.apk'; run(tools/'zipalign', '-f', '4', apk, aligned)
    signed = out/'probe.apk'
    run(tools/'apksigner', 'sign', '--ks', fixture/'android-development.keystore',
        '--ks-key-alias', 'pax-development', '--ks-pass', 'pass:android', '--out', signed, aligned)
    adb = str(sdk/'platform-tools/adb')
    installed = subprocess.run([adb, 'install', '--no-incremental', '-r', '-t', str(signed)],
                               capture_output=True, text=True, timeout=90)
    if installed.returncode and 'INSTALL_FAILED_UPDATE_INCOMPATIBLE' in installed.stdout+installed.stderr:
        # This helper owns no user data. A cleaned fixture can have a new local key.
        run(adb, 'uninstall', 'dev.pax.android_spatial_probe')
        run(adb, 'install', '--no-incremental', '-t', signed)
    elif installed.returncode:
        raise RuntimeError(installed.stdout+installed.stderr)
    (out/'device.txt').write_text(subprocess.check_output([adb, 'shell', 'getprop'], text=True, timeout=10))
    try:
        result = subprocess.check_output([adb, 'shell', 'am', 'instrument', '-w', '-e', 'edgeOn', str(args.edge_on).lower(),
            f'dev.pax.android_spatial_probe/dev.pax.{probe}'], text=True, timeout=240)
    except subprocess.TimeoutExpired as error:
        (out/'instrumentation.txt').write_text('Instrumentation timed out after 240 seconds\n')
        run(adb, 'shell', 'am', 'force-stop', target)
        raise RuntimeError('Spatial probe timed out') from error
    (out/'instrumentation.txt').write_text(result)
    print(result)
    run(adb, 'pull', f'/sdcard/Android/data/{target}/files/spatial-acceptance', out/'captures')
    fields = dict(re.findall(r'INSTRUMENTATION_RESULT: ([^=\n]+)=([^\r\n]*)', result))
    passed = fields.get('passed') == 'true' and 'error' not in fields and 'INSTRUMENTATION_CODE: -1' in result
    (out/'result.json').write_text(json.dumps({'passed': passed, 'profile': args.profile, 'edgeOnRequested': args.edge_on,
        'requiresVisualReview': True,
        'instrumentation': fields}, indent=2)+'\n')
    assert passed, result


if __name__ == '__main__':
    main()
