"""Create alternate/rejected packages and prove the base bytes stay unchanged.

Run after build.py --output <root>/split. Outputs are disposable evidence.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--interface', type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parent
output = args.output.resolve()
source = output / 'split'
variant = output / 'alternate'
shutil.copytree(source, variant, dirs_exist_ok=True)
base_bytes = (source / 'pax-cartridge_bg.wasm').read_bytes()
for sentinel in [b'Child sees parent:', b'Module-local clicks:', b'local_clicks', b'probe_bundle::Counter']:
    assert sentinel not in base_bytes, sentinel
    assert sentinel in (source / 'child.wasm').read_bytes(), sentinel
schema = json.loads((root / 'component/descriptor.json').read_text())
schema['fields'].append({'name': 'bundle_revision', 'kind': 'u32', 'import': None, 'default': 2026})
schema['handlers'][2]['name'] = 'advance'
(variant / 'descriptor.json').write_text(json.dumps(schema))
template = (root / 'component/counter.pax').read_text().replace('@button_click=increment', '@button_click=advance')
template = template.replace('"Module-local clicks: " + local_clicks', '"Bundle revision: " + bundle_revision + " · Clicks: " + local_clicks')
template = template.replace('"Child adds "', '"Bundle v2 adds "')
(variant / 'counter.pax').write_text(template)
subprocess.run(['python3', str(root / 'build.py'), '--child-only', '--output', str(variant),
                '--interface', str(args.interface), '--template', str(variant / 'counter.pax'),
                '--descriptor', str(variant / 'descriptor.json')], check=True)
assert (variant / 'pax-cartridge_bg.wasm').read_bytes() == base_bytes
assert (variant / 'child.wasm').read_bytes() != (source / 'child.wasm').read_bytes()
assert b'bundle_revision' not in base_bytes
assert b'bundle_revision' in (variant / 'child.wasm').read_bytes()

# Valid WASM with inconsistent package metadata must fail before mounting and
# allow a subsequent valid package. Build these through the same child toolchain.
for kind in ['schema', 'route', 'definition']:
    bad = json.loads((root / 'component/descriptor.json').read_text())
    if kind == 'schema':
        bad['fields'][0]['import'] = 'missing_parent_property'
    if kind == 'route':
        bad['handlers'][2]['name'] = 'missing_handler'
    descriptor = output / f'bad-{kind}.json'
    descriptor.write_text(json.dumps(bad))
    definition = source / 'component.paxm'
    if kind == 'definition':
        definition = output / 'bad-definition.paxm'
        definition.write_bytes((source / 'component.paxm').read_bytes()[:100])
    env = dict(os.environ, PROBE_DEFINITION=str(definition), PROBE_DESCRIPTOR=str(descriptor))
    subprocess.run(['rustc', str(root / 'child.rs'), '--crate-type', 'cdylib', '--target',
                    'wasm32-unknown-unknown', '--edition', '2021', '-C', 'opt-level=z', '-C',
                    'panic=abort', '-C', 'strip=symbols', '-C', 'link-arg=-zstack-size=65536',
                    '-o', str(source / f'bad-{kind}.wasm')], env=env, check=True)
summary = {'baseSha256': hashlib.sha256(base_bytes).hexdigest(),
           'alternateBaseIdentical': True, 'childSentinelsAbsentFromBase': True,
           'alternateChildSha256': hashlib.sha256((variant / 'child.wasm').read_bytes()).hexdigest()}
(output / 'package-proof.json').write_text(json.dumps(summary, indent=2))
print(json.dumps(summary))
