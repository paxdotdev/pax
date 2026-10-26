"""Build the isolated probe without modifying the production CLI or loader."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--interface', type=Path, required=True,
                    help='Built web interface from the same Pax revision')
parser.add_argument('--mode', choices=['split', 'monolithic'], default='split')
parser.add_argument('--debug', action='store_true')
parser.add_argument('--child-only', action='store_true')
parser.add_argument('--template', type=Path)
parser.add_argument('--descriptor', type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parent
output = args.output.resolve()
output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ)
env['RUSTFLAGS'] = (env.get('RUSTFLAGS', '') + ' -C link-arg=--export-table').strip()
env['PAX_BUILD_TARGET'] = 'web'
env['PAX_BUILD_DESIGNTIME'] = '0'
definition = output / 'component.paxm'
descriptor = (args.descriptor or root / 'component/descriptor.json').resolve()
subprocess.run(['cargo', 'run', '--manifest-path', str(root / 'Cargo.toml'),
                '-p', 'probe-component-packer', '--',
                str((args.template or root / 'component/counter.pax').resolve()),
                str(descriptor), str(definition)], check=True)
env['PROBE_DEFINITION'] = str(definition)
env['PROBE_DESCRIPTOR'] = str(descriptor)
command = ['wasm-pack', 'build', str(root / 'host'), '--target', 'web', '--out-name',
           'pax-cartridge', '--out-dir', str(output), '--dev' if args.debug else '--release', '--no-opt']
if args.mode == 'monolithic':
    command += ['--features', 'monolithic']
if not args.child_only:
    subprocess.run(command, env=env, check=True)
glue = output / 'pax-cartridge.js'
source = glue.read_text()
entries = re.findall(r'export function (__pax_init_web_[a-f0-9]+)\(', source)
assert len(entries) == 1, entries
if ' as init_web };' not in source:
    glue.write_text(source + '\nexport { ' + entries[0] + ' as init_web };\n')
for name in ['pax-interface-web.js', 'pax-interface-web.css']:
    shutil.copy2(args.interface / name, output / name)
for path in (root / 'web').iterdir():
    shutil.copy2(path, output / path.name)
subprocess.run(['rustc', str(root / 'child.rs'), '--crate-type', 'cdylib', '--target',
                'wasm32-unknown-unknown', '--edition', '2021', '-C',
                'opt-level=1' if args.debug else 'opt-level=z', '-C', 'panic=abort',
                '-C', 'strip=symbols', '-C', 'link-arg=-zstack-size=65536',
                '-o', str(output / 'child.wasm')], env=env, check=True)
(output / 'probe-build.json').write_text(json.dumps({
    'command': None if args.child_only else command, 'mode': args.mode, 'debug': args.debug,
    'child_only': args.child_only, 'template': str((args.template or root / 'component/counter.pax').resolve()),
    'descriptor': str(descriptor),
    'rustflags': env['RUSTFLAGS'], 'interface': str(args.interface.resolve()),
}, indent=2))
