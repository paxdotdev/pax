#!/bin/bash

set -euo pipefail
read -p "Enter a name for the example: " NAME
if [[ ! "$NAME" =~ ^[a-z][a-z0-9-]*$ ]]; then
    echo "Use a lowercase Cargo package name" >&2
    exit 1
fi
examples_dir=$(cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(dirname -- "$examples_dir")
cd "$repo_dir"
cargo run -p pax-cli -- create "$examples_dir/$NAME" --libdev

python3 - "$NAME" "$repo_dir" <<'PY'
from pathlib import Path
import re
import sys
name, directory = sys.argv[1:]
if not re.fullmatch(r"[a-z][a-z0-9-]*", name):
    raise SystemExit("Use a lowercase Cargo package name")
root = Path(directory)
manifest = root / "Cargo.toml"
dependency = f'{name} = {{ path = "examples/{name}", default-features = false }}\n'
text = manifest.read_text()
if dependency not in text:
    text = text.replace('[dev-dependencies]\n', '[dev-dependencies]\n' + dependency, 1)
    text = text.replace('[profile.release]', f'[profile.dev.package.{name}]\nopt-level = 0\n\n[profile.release]', 1)
    manifest.write_text(text)
# The new member inherits the workspace profiles; exported CLI projects retain
# their own copies for use outside this workspace.
app_manifest = root / "examples" / name / "Cargo.toml"
app_text = app_manifest.read_text()
app_text = re.sub(r'(?ms)^\[profile\.[^\n]*\]\n.*?(?=^\[|\Z)', '', app_text)
app_manifest.write_text(app_text)
(root / "examples" / f"{name}.rs").write_text(
    'fn main() -> Result<(), pax_kit::host::Error> {\n'
    f'    pax_kit::init!({name.replace("-", "_")})?.run()\n'
    '}\n'
)
PY
echo "$NAME is ready: cargo run -p $NAME or cargo run --example $NAME"
