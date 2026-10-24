#!/usr/bin/env python3
"""Build/install the test-only UiAutomation helper, without Gradle or dependencies."""
import argparse
import os
from pathlib import Path
import subprocess
import zipfile


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--install',action='store_true')
    parser.add_argument('--ime',action='store_true',help='Build the separate, test-only composition keyboard instead of UI instrumentation')
    args=parser.parse_args()
    sdk=Path(os.environ['ANDROID_HOME'])
    java=Path(os.environ['JAVA_HOME'])/'bin'
    tools=sdk/'build-tools/36.0.0'
    platform=sdk/'platforms/android-36/android.jar'
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    app='dev.pax.android_ime' if args.ime else 'dev.pax.android_probe'
    name='ime' if args.ime else 'probe'

    def run(*command):
        subprocess.run([str(c) for c in command],check=True)

    classes=out/'classes';classes.mkdir(exist_ok=True)
    repository=Path(__file__).resolve().parents[4]
    run(java/'javac','--release','8','-classpath',platform,'-d',classes,
        *sorted((Path(__file__).parent/'ui-probe').glob('*.java')),
        repository/'pax-host/native/android/java/dev/pax/PendingUpdates.java',
        repository/'pax-host/native/android/java/dev/pax/NativePath.java',
        repository/'pax-host/native/android/java/dev/pax/NativeRoutes.java',
        repository/'pax-host/native/android/java/dev/pax/SystemFonts.java')
    with zipfile.ZipFile(out/'probe.jar','w') as archive:
        for file in classes.rglob('*.class'): archive.write(file,file.relative_to(classes))
    run(tools/'d8','--min-api','29','--lib',platform,'--output',out,out/'probe.jar')
    xml=out/'res/xml';xml.mkdir(parents=True,exist_ok=True)
    (xml/'composition_ime.xml').write_text('''<input-method xmlns:android="http://schemas.android.com/apk/res/android" android:supportsSwitchingToNextInputMethod="true">
      <subtype android:imeSubtypeLocale="en_US" android:imeSubtypeMode="keyboard"/>
    </input-method>''')
    run(tools/'aapt2','compile','--dir',out/'res','-o',out/'resources.zip')
    manifest=out/'AndroidManifest.xml'
    service='''<service android:name="dev.pax.android_ime.CompositionIme" android:permission="android.permission.BIND_INPUT_METHOD" android:exported="true">
      <intent-filter><action android:name="android.view.InputMethod"/></intent-filter>
      <meta-data android:name="android.view.im" android:resource="@xml/composition_ime"/>
    </service>''' if args.ime else ''
    instrumentation='' if args.ime else '''<instrumentation android:name="dev.pax.android_probe.Probe" android:targetPackage="dev.pax.android_probe"/>
      <instrumentation android:name="dev.pax.HostTests" android:targetPackage="dev.pax.android_probe"/>'''
    # Instrumentation may restart its target package. Keep the keyboard in a
    # different package so UI snapshots never replace its active connection.
    manifest.write_text(f'''<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="{app}">
      <uses-sdk android:minSdkVersion="29" android:targetSdkVersion="36"/>
      <application android:label="Pax acceptance helper" android:testOnly="true" android:debuggable="true">
        {service}
      </application>
      {instrumentation}
    </manifest>''')
    apk=out/'probe-unsigned.apk'
    run(tools/'aapt2','link','-o',apk,'--manifest',manifest,'-I',platform,out/'resources.zip')
    with zipfile.ZipFile(apk,'a',compression=zipfile.ZIP_DEFLATED) as archive:
        archive.write(out/'classes.dex','classes.dex')
    aligned=out/'probe-aligned.apk';run(tools/'zipalign','-f','4',apk,aligned)
    key=out/'probe.keystore'
    if not key.exists():
        run(java/'keytool','-genkeypair','-keystore',key,'-storepass','android','-keypass','android',
            '-alias','probe','-dname','CN=Pax acceptance helper','-keyalg','RSA','-validity','3650')
    signed=out/(name+'.apk')
    run(tools/'apksigner','sign','--ks',key,'--ks-pass','pass:android','--out',signed,aligned)
    if args.install:
        # This package contains only test instrumentation and stores no user data.
        # A cleaned build directory can have a different disposable signer.
        installed=subprocess.run([str(sdk/'platform-tools/adb'),'install','--no-incremental','-r','-t',str(signed)],capture_output=True,text=True)
        if installed.returncode and 'INSTALL_FAILED_UPDATE_INCOMPATIBLE' in installed.stdout+installed.stderr:
            run(sdk/'platform-tools/adb','uninstall',app)
            run(sdk/'platform-tools/adb','install','--no-incremental','-t',signed)
        elif installed.returncode:
            raise RuntimeError(installed.stdout+installed.stderr)
        if not args.ime:
            tests=subprocess.check_output([str(sdk/'platform-tools/adb'),'shell','am','instrument','-w',
                'dev.pax.android_probe/dev.pax.HostTests'],text=True,timeout=60)
            if 'INSTRUMENTATION_RESULT: error=' in tests or 'INSTRUMENTATION_CODE: -1' not in tests:
                raise RuntimeError(tests)
            print(tests)
    print(signed)


if __name__=='__main__': main()
