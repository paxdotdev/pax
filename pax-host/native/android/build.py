#!/usr/bin/env python3
"""Package the explicit Android host with the already compiled ARM64 cartridge.

Uses Android's SDK tools directly; no Gradle daemon or network restore is needed.
Release output is unsigned unless an explicit signing environment is supplied.
Local runs always use a separate development-signed APK.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile
from xml.sax.saxutils import quoteattr


def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)


def main():
    config = json.loads(Path(sys.argv[1]).read_text())
    source = Path(__file__).resolve().parent
    output = Path(config['destination'])
    sdk = Path(config['sdk'])
    settings = config['android']
    tools = sdk / 'build-tools' / settings['build_tools']
    platform = sdk / f"platforms/android-{settings['compile_sdk']}/android.jar"
    java_home = os.environ.get('JAVA_HOME')
    if not java_home:
        raise SystemExit('Set JAVA_HOME to JDK 17 or newer (Android Studio includes a JDK).')
    java = Path(java_home) / 'bin'
    for tool in (tools/'aapt2', tools/'d8', tools/'apksigner', tools/'zipalign', platform,
                 java/'javac', java/'java', java/'keytool', java/'jarsigner'):
        if not tool.is_file():
            raise SystemExit(f'Missing Android build prerequisite: {tool}')
    work = output / 'intermediates'
    # Own this generated directory completely: removed classes and assets must
    # not leak into a later APK assembled from the same output directory.
    if work.exists():
        shutil.rmtree(work)
    # A previous signing/format choice must not leave a stale distributable.
    for name in ('app-unsigned.apk', 'app-unsigned.aab', 'app-release.apk',
                 'app-release.aab', 'app-debug.apk', 'app-development.apk'):
        (output/name).unlink(missing_ok=True)
        (output/(name+'.idsig')).unlink(missing_ok=True)
    classes = work/'classes'
    assets = work/'assets'
    lib = work/'lib/arm64-v8a'
    for path in (classes, assets, lib):
        path.mkdir(parents=True, exist_ok=True)
    library = Path(config['library'])
    shutil.copy2(library, lib/library.name)
    roots = []
    for i, root in enumerate(config['assets']):
        name = f'pax-assets/{i}'
        shutil.copytree(root, assets/name, dirs_exist_ok=True)
        roots.append(name)
    (assets/'pax-app.json').write_text(json.dumps({'library': library.name, 'assetRoots': roots,
        'baseUrl': config.get('baseUrl', ''), 'nativeExports': config['nativeExports']}))
    app = config['applicationId']
    release = config['release']
    manifest = work/'AndroidManifest.xml'
    resources = []
    theme = '@android:style/Theme.Material.Light.NoActionBar'
    bar_icons = settings.get('system_bar_icons')
    if bar_icons is not None:
        if bar_icons not in ('light', 'dark'):
            raise SystemExit('android.system_bar_icons must be light or dark')
        styles = work/'res/values/styles.xml'
        styles.parent.mkdir(parents=True)
        # Android's "light bar" attributes request dark foreground icons.
        light_bar = str(bar_icons == 'dark').lower()
        styles.write_text(f'''<resources><style name="PaxTheme" parent="android:style/Theme.Material.Light.NoActionBar">
  <item name="android:windowLightStatusBar">{light_bar}</item>
  <item name="android:windowLightNavigationBar">{light_bar}</item>
</style></resources>''')
        theme = '@style/PaxTheme'
    icon_attribute = ''
    if settings.get('icon'):
        icon = work/'res/drawable/pax_icon.png'
        icon.parent.mkdir(parents=True)
        shutil.copy2(Path(config['project'])/settings['icon'], icon)
        icon_attribute = ' android:icon="@drawable/pax_icon"'
    if (work/'res').exists():
        compiled = work/'resources.zip'
        run(tools/'aapt2', 'compile', '--dir', work/'res', '-o', compiled)
        resources.append(compiled)
    permissions = '\n'.join(f'<uses-permission android:name={quoteattr(p)}/>' for p in sorted(set(settings['permissions'])))
    manifest.write_text(f'''<manifest xmlns:android="http://schemas.android.com/apk/res/android" package={quoteattr(app)} android:versionCode="{settings['version_code']}" android:versionName={quoteattr(config['version'])}>
  <uses-sdk android:minSdkVersion="{settings['min_sdk']}" android:targetSdkVersion="{settings['target_sdk']}"/>
  {permissions}
  <uses-feature android:name="android.hardware.vulkan.level" android:version="0" android:required="true"/>
  <application{icon_attribute} android:label={quoteattr(config['title'])} android:theme={quoteattr(theme)} android:hardwareAccelerated="true" android:allowBackup="false" android:extractNativeLibs="true" android:debuggable="{str(not release).lower()}">
    <activity android:name="dev.pax.PaxActivity" android:exported="true" android:enableOnBackInvokedCallback="true" android:windowSoftInputMode="adjustResize">
      <intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter>
    </activity>
  </application>
</manifest>''')
    run(Path(config['ndkBin'])/'aarch64-linux-android29-clang', '-std=c11', '-D_GNU_SOURCE', '-DPAX_DEBUG_HOST='+('0' if release else '1'), '-shared', '-fPIC', '-O2', '-Wall', '-Wextra', '-Werror', '-Wl,-z,max-page-size=16384', source/'host.c', '-landroid', '-ldl', '-o', lib/'libpax_host.so')
    run(java/'javac', '--release', '8', '-classpath', platform, '-d', classes, *sorted((source/'java').rglob('*.java')))
    # A jar keeps the dex invocation below command-line length limits.
    jar = work/'host.jar'
    with zipfile.ZipFile(jar, 'w') as archive:
        for file in sorted(classes.rglob('*.class')):
            archive.write(file, file.relative_to(classes).as_posix())
    run(tools/'d8', '--min-api', settings['min_sdk'], '--lib', platform, '--output', work, jar)
    resource_apk = work/'resources.apk'
    run(tools/'aapt2', 'link', '-o', resource_apk, '--manifest', manifest, '-I', platform, '-A', assets, *resources)
    assembled = work/'assembled.apk'
    shutil.copy2(resource_apk, assembled)
    with zipfile.ZipFile(assembled, 'a', compression=zipfile.ZIP_DEFLATED) as archive:
        archive.write(work/'classes.dex', 'classes.dex')
        for file in sorted(lib.glob('*.so')):
            archive.write(file, 'lib/arm64-v8a/'+file.name)
    unsigned = output/'app-unsigned.apk'
    run(tools/'zipalign', '-f', '4', assembled, unsigned)
    bundle = None
    if settings['format'] in ('aab', 'both'):
        bundletool = os.environ.get('PAX_ANDROID_BUNDLETOOL')
        if not bundletool or not Path(bundletool).is_file():
            raise SystemExit('Set PAX_ANDROID_BUNDLETOOL to bundletool-all.jar to build App Bundles.')
        proto = work/'resources-proto.apk'
        run(tools/'aapt2', 'convert', '--output-format', 'proto', '-o', proto, resource_apk)
        module = work/'base.zip'
        with zipfile.ZipFile(proto) as inputs, zipfile.ZipFile(module, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
            for name in inputs.namelist():
                archive.writestr('manifest/AndroidManifest.xml' if name == 'AndroidManifest.xml' else name, inputs.read(name))
            archive.write(work/'classes.dex', 'dex/classes.dex')
            for file in sorted(lib.glob('*.so')):
                archive.write(file, 'lib/arm64-v8a/'+file.name)
        bundle = output/'app-unsigned.aab'
        run(java/'java', '-jar', bundletool, 'build-bundle', f'--modules={module}', f'--output={bundle}', '--overwrite')
        run(java/'java', '-jar', bundletool, 'validate', f'--bundle={bundle}')
    if release and os.environ.get('PAX_ANDROID_KEYSTORE'):
        keystore = os.environ['PAX_ANDROID_KEYSTORE']
        alias = os.environ.get('PAX_ANDROID_KEY_ALIAS')
        if not alias or not os.environ.get('PAX_ANDROID_STORE_PASSWORD') or not os.environ.get('PAX_ANDROID_KEY_PASSWORD'):
            raise SystemExit('Signing needs PAX_ANDROID_KEY_ALIAS, PAX_ANDROID_STORE_PASSWORD and PAX_ANDROID_KEY_PASSWORD.')
        if settings['format'] != 'aab':
            signed = output/'app-release.apk'
            run(tools/'apksigner', 'sign', '--ks', keystore, '--ks-key-alias', alias,
                '--ks-pass', 'env:PAX_ANDROID_STORE_PASSWORD', '--key-pass', 'env:PAX_ANDROID_KEY_PASSWORD', '--out', signed, unsigned)
            run(tools/'apksigner', 'verify', signed)
        if bundle:
            signed_bundle = output/'app-release.aab'
            shutil.copy2(bundle, signed_bundle)
            run(java/'jarsigner', '-keystore', keystore, '-storepass:env', 'PAX_ANDROID_STORE_PASSWORD',
                '-keypass:env', 'PAX_ANDROID_KEY_PASSWORD', signed_bundle, alias)
            run(java/'jarsigner', '-verify', signed_bundle)
    if not release or config['run']:
        keystore = Path(config['keystore'])
        if not keystore.exists():
            run(java/'keytool', '-genkeypair', '-keystore', keystore, '-storepass', 'android', '-keypass', 'android', '-alias', 'pax-development', '-dname', 'CN=Pax Development', '-keyalg', 'RSA', '-validity', '3650')
        apk = output/('app-development.apk' if release else 'app-debug.apk')
        run(tools/'apksigner', 'sign', '--ks', keystore, '--ks-key-alias', 'pax-development', '--ks-pass', 'pass:android', '--key-pass', 'pass:android', '--out', apk, unsigned)
        run(tools/'apksigner', 'verify', apk)
        if config['run']:
            adb = sdk/'platform-tools/adb'
            # Launch only after every APK block is on the device. Incremental
            # installation can retain a host-side streaming server after adb
            # returns, coupling later launches to this build process's lifetime.
            run(adb, 'install', '--no-incremental', '-r', apk)
            run(adb, 'shell', 'am', 'force-stop', app)
            extras = ['--es', 'pax.design_server', config['designServer']] if config.get('designServer') and not release else []
            run(adb, 'shell', 'am', 'start', '-W', '-n', app+'/dev.pax.PaxActivity', *extras)
    if settings['format'] == 'aab':
        unsigned.unlink()
    print(f'Android {"Release" if release else "Debug"} package: {output}')


if __name__ == '__main__':
    main()
