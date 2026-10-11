#!/bin/bash

set -euo pipefail
category="fixtures"
if [[ "${1:-}" == "--showcase" ]]; then
    category=""
    shift
fi
if [[ $# -gt 1 ]]; then
    echo "Usage: $0 [--showcase] [name]" >&2
    exit 1
fi
NAME="${1:-}"
if [[ -z "$NAME" ]]; then
    read -p "Enter a name for the example: " NAME
fi
if [[ ! "$NAME" =~ ^[a-z][a-z0-9-]*$ ]]; then
    echo "Use a lowercase Cargo package name" >&2
    exit 1
fi
examples_dir=$(cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(dirname -- "$examples_dir")
cd "$repo_dir"
if [[ -e "$examples_dir/$NAME" || -e "$examples_dir/fixtures/$NAME" ]]; then
    echo "An example named $NAME already exists" >&2
    exit 1
fi
example_parent="$examples_dir${category:+/$category}"
mkdir -p "$example_parent"
cargo run -p pax-cli -- create "$example_parent/$NAME" --libdev

python3 - "$NAME" "$repo_dir" "$category" <<'PY'
from pathlib import Path
import re
import sys
name, directory, category = sys.argv[1:]
if not re.fullmatch(r"[a-z][a-z0-9-]*", name):
    raise SystemExit("Use a lowercase Cargo package name")
root = Path(directory)
manifest = root / "Cargo.toml"
relative = Path("examples") / category / name
dependency = f'{name} = {{ path = "{relative.as_posix()}", default-features = false }}\n'
text = manifest.read_text()
if dependency not in text:
    text = text.replace('[dev-dependencies]\n', '[dev-dependencies]\n' + dependency, 1)
    text = text.replace('[profile.release]', f'[profile.dev.package.{name}]\nopt-level = 0\n\n[profile.release]', 1)
    if not category:
        text = text.replace('members = [\n', f'members = [\n    "{relative.as_posix()}",\n', 1)
    text += f'\n[[example]]\nname = "{name}"\npath = "{relative.as_posix()}.rs"\n'
    manifest.write_text(text)
# The new member inherits the workspace profiles; exported CLI projects retain
# their own copies for use outside this workspace.
app_manifest = root / relative / "Cargo.toml"
app_text = app_manifest.read_text()
app_text = re.sub(r'(?ms)^\[profile\.[^\n]*\]\n.*?(?=^\[|\Z)', '', app_text)
app_manifest.write_text(app_text)
(root / relative.with_suffix(".rs")).write_text(
    'fn main() -> Result<(), pax_kit::host::Error> {\n'
    f'    pax_kit::init!({name.replace("-", "_")})?.run()\n'
    '}\n'
)
PY
echo "$NAME is ready: pax-cli run --path examples/${category:+$category/}$NAME --target web"
echo "On macOS: cargo run -p $NAME or cargo run --example $NAME"
