#!/usr/bin/env python3
"""Inspect built APKs without a device, including each library's 16 KiB alignment."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import zipfile


def check_elf(data, name):
    assert data[:6] == b'\x7fELF\x02\x01', f'{name}: expected little-endian ELF64'
    assert struct.unpack_from('<H', data, 18)[0] == 183, f'{name}: expected AArch64'
    offset = struct.unpack_from('<Q', data, 32)[0]
    entry_size, count = struct.unpack_from('<HH', data, 54)
    assert entry_size >= 56 and offset + count * entry_size <= len(data), f'{name}: invalid program headers'
    segments = 0
    for i in range(count):
        kind, _, file_offset, address, _, _, _, alignment = struct.unpack_from('<IIQQQQQQ', data, offset+i*entry_size)
        if kind == 1:  # PT_LOAD must be mmap-compatible on a 16 KiB device.
            assert alignment >= 16384 and alignment & (alignment-1) == 0, f'{name}: PT_LOAD alignment {alignment} is below 16 KiB or invalid'
            assert (address-file_offset) % 16384 == 0, f'{name}: PT_LOAD offset/address mismatch'
            segments += 1
    assert segments, f'{name}: no loadable segments'
    return segments


def defined_exports(data):
    """Read the cartridge's dynamic symbols without requiring host ELF tools."""
    offset = struct.unpack_from('<Q', data, 40)[0]
    entry_size, count = struct.unpack_from('<HH', data, 58)
    assert entry_size >= 64 and offset + entry_size * count <= len(data), 'Invalid ELF sections'
    sections = [struct.unpack_from('<IIQQQQIIQQ', data, offset + i * entry_size) for i in range(count)]
    exports = set()
    for section in sections:
        if section[1] != 11:  # SHT_DYNSYM
            continue
        strings = sections[section[6]]
        names = data[strings[4]:strings[4] + strings[5]]
        assert section[9] >= 24 and section[4] + section[5] <= len(data), 'Invalid dynamic symbols'
        for at in range(section[4], section[4] + section[5], section[9]):
            name, info, visibility, defined, _, _ = struct.unpack_from('<IBBHQQ', data, at)
            if defined and info >> 4 in (1, 2) and visibility & 3 in (0, 3):
                end = names.index(b'\0', name)
                exports.add(names[name:end].decode())
    return exports


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('apk', type=Path)
    parser.add_argument('--profile', choices=['debug', 'release'], required=True)
    parser.add_argument('--sdk', type=Path, default=Path(os.environ.get('ANDROID_HOME', '')))
    parser.add_argument('--build-tools', default='36.0.0')
    parser.add_argument('--min-sdk', type=int, default=29)
    parser.add_argument('--target-sdk', type=int, default=36)
    args = parser.parse_args()
    tools = args.sdk/'build-tools'/args.build_tools
    badging = subprocess.check_output([str(tools/'aapt2'), 'dump', 'badging', str(args.apk)], text=True)
    assert f"minSdkVersion:'{args.min_sdk}'" in badging, 'Unexpected minimum SDK'
    assert f"targetSdkVersion:'{args.target_sdk}'" in badging, 'Unexpected target SDK'
    assert ('application-debuggable' in badging) == (args.profile == 'debug'), 'Wrong debuggable flag'
    assert "launchable-activity: name='dev.pax.PaxActivity'" in badging, 'Missing host Activity'
    manifest = subprocess.check_output([str(tools/'aapt2'), 'dump', 'xmltree', str(args.apk),
                                        '--file', 'AndroidManifest.xml'], text=True)
    assert re.search(r':extractNativeLibs\(0x[0-9a-f]+\)=true', manifest), 'Native library extraction must be enabled'
    libraries = {}
    with zipfile.ZipFile(args.apk) as archive:
        names = archive.namelist()
        assert len(names) == len(set(names)), 'Duplicate APK entries'
        assert 'classes.dex' in names, 'Missing Java host'
        config = json.loads(archive.read('assets/pax-app.json'))
        assert 'serverOwnedPrefixes' not in config, 'Web document policy leaked into native routing'
        cartridge = archive.read('lib/arm64-v8a/' + config['library'])
        check_elf(cartridge, config['library'])
        exported = defined_exports(cartridge)
        selected = config['nativeExports']
        assert set(selected) == {'abiVersion', 'programId', 'init', 'initWithData', 'initEntry'}, 'Incomplete program selection metadata'
        assert len(set(selected.values())) == 5, 'Program entrypoints must be distinct'
        assert set(selected.values()) <= exported, 'Selected program entrypoints are absent from the cartridge'
        assert not {'pax_init', 'pax_init_entry', 'pax_init_with_data', 'pax_cartridge_abi_version', 'pax_cartridge_program_id'} & exported, 'Generic program exports would collide across runnable libraries'

        assert 'lib/arm64-v8a/'+config['library'] in names, 'Missing selected cartridge'
        assert 'lib/arm64-v8a/libpax_host.so' in names, 'Missing JNI bridge'
        for root in config['assetRoots']:
            assert any(name.startswith('assets/'+root+'/') for name in names), f'Missing asset root {root}'
        for entry in archive.infolist():
            if not entry.filename.startswith('lib/') or not entry.filename.endswith('.so'):
                continue
            assert entry.filename.startswith('lib/arm64-v8a/'), 'Unexpected native ABI'
            # The current packager extracts compressed libraries at install time.
            # Direct mmap of uncompressed entries needs a separate ZIP alignment check.
            assert entry.compress_type == zipfile.ZIP_DEFLATED, 'Expected compressed/extracted native library'
            libraries[entry.filename] = check_elf(archive.read(entry), entry.filename)
    if args.profile == 'debug':
        subprocess.run([str(tools/'apksigner'), 'verify', str(args.apk)], check=True)
    print(json.dumps({'apk': str(args.apk), 'profile': args.profile,
                      'sha256': hashlib.sha256(args.apk.read_bytes()).hexdigest(),
                      'loadSegments': libraries, 'passed': True}))


if __name__ == '__main__':
    main()
