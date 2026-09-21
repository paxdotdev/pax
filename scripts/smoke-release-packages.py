#!/usr/bin/env python3
"""Exercise actual candidate archives outside the checkout before publication.

Only unpacked .crate files are patched into the registry dependency graph. This
is candidate-package evidence, not a substitute for published-channel acceptance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tarfile
import tempfile
import time
import urllib.request


def run(command, cwd, env, *, capture=False):
    print('+', ' '.join(map(str, command)), flush=True)
    return subprocess.run(list(map(str, command)), cwd=cwd, env=env, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('candidate', type=Path)
    parser.add_argument('--target-dir', type=Path, help='Optional shared compilation cache')
    args = parser.parse_args()
    candidate = json.loads(args.candidate.read_text())
    version = candidate['version']
    root = Path(tempfile.mkdtemp(prefix=f'pax-{version}-package-smoke-')).resolve()
    archives = root / 'archives'
    archives.mkdir()
    patches = ['[patch.crates-io]']
    for package in candidate['packages']:
        if hashlib.sha256(Path(package['archive']).read_bytes()).hexdigest() != package['sha256']:
            raise RuntimeError(f"Candidate archive changed: {package['name']}")
        with tarfile.open(package['archive']) as tar:
            tar.extractall(archives, filter='data')
        directory = archives / f"{package['name']}-{version}"
        patches.append(f"{package['name']} = {{ path = {json.dumps(str(directory))} }}")
    (root / '.cargo').mkdir()
    (root / '.cargo/config.toml').write_text('\n'.join(patches) + '\n')
    env = dict(os.environ, PAX_TELEMETRY='off')
    env.pop('CARGO_TARGET_DIR', None)
    target = args.target_dir.resolve() if args.target_dir else root / 'target'
    run(['cargo', 'build', '--manifest-path', archives / f'pax-cli-{version}/Cargo.toml',
         '--bin', 'pax-cli', '--target-dir', target], root, env)
    cli = target / 'debug/pax-cli'
    reported = run([cli, '--version'], root, env, capture=True)
    if version not in reported:
        raise RuntimeError(f'Unexpected packaged CLI version: {reported}')
    sources = run([cli, 'docs', 'examples', 'living-quilt'], root, env, capture=True)
    if 'quilt_tile' not in sources or '<' not in sources:
        raise RuntimeError('Packaged CLI is missing embedded Living Quilt sources')
    (root / 'docs-example.txt').write_text(sources)
    run([cli, 'create', root / 'fresh-quilt'], root, env)
    project = root / 'fresh-quilt'
    if not (project / 'src/quilt_tile.pax').is_file():
        raise RuntimeError('Packaged CLI did not generate Living Quilt')
    meta = json.loads(run(['cargo', 'metadata', '--format-version', '1',
                           '--features', 'pax-kit/web,pax-kit/designtime'], project, env, capture=True))
    pax_packages = [p for p in meta['packages'] if p['name'].startswith('pax-')]
    if not pax_packages or any(p['version'] != version or not Path(p['manifest_path']).is_relative_to(archives)
                               for p in pax_packages):
        raise RuntimeError('Generated project did not resolve exclusively to candidate Pax archives')
    (root / 'metadata.json').write_text(json.dumps(meta, indent=2) + '\n')
    # Normal published users receive JS/CSS already built. Fail explicitly if
    # this smoke unexpectedly attempts to invoke either frontend build tool.
    blocked = root / 'blocked-frontend-tools'
    blocked.mkdir()
    for name in ('node', 'npm', 'npx'):
        path = blocked / name
        path.write_text('#!/bin/sh\necho "Unexpected frontend build dependency" >&2\nexit 97\n')
        path.chmod(0o755)
    env['PATH'] = str(blocked) + os.pathsep + env['PATH']
    run([cli, 'build', '--target=web'], project, env)
    log_path = root / 'run.log'
    with log_path.open('w') as log:
        process = subprocess.Popen([str(cli), 'run', '--target=web', '--hot-reload=off'],
                                   cwd=project, env=env, stdout=log, stderr=subprocess.STDOUT,
                                   start_new_session=True)
        try:
            deadline = time.monotonic() + 180
            while time.monotonic() < deadline:
                text = log_path.read_text()
                match = re.search(r'Server running at (http://[^\s\x1b]+)', text)
                if match:
                    with urllib.request.urlopen(match.group(1), timeout=10) as response:
                        if response.status != 200 or not response.read():
                            raise RuntimeError('Generated web server returned an empty/error response')
                    break
                if process.poll() is not None:
                    raise RuntimeError(f'Generated run exited before serving; see {log_path}')
                time.sleep(0.5)
            else:
                raise RuntimeError(f'Generated run did not become ready; see {log_path}')
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
    report = {'version': version, 'directory': str(root), 'cli': str(cli),
              'pax_dependencies': [p['name'] for p in pax_packages],
              'checks': ['CLI build from archives', 'embedded docs sources', 'Living Quilt create',
                         'candidate dependency provenance', 'web build without Node/npm', 'web run HTTP 200'],
              'limitation': 'Local archive patches; not published-channel proof'}
    (args.candidate.parent / 'package-smoke.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
