#!/usr/bin/env python3
"""Snapshot canonical example source for registry-built CLI documentation.

This is a source-reading bundle, not a runnable project archive. Assets and
build output belong to the separate starter and hosted-example pipelines.
"""
import argparse
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXCLUDED = {".git", ".pax", "target", "node_modules", "__pycache__"}


def snapshot(workspace):
    files = {}
    examples = workspace / "examples/src"
    for example in sorted(examples.iterdir()):
        if example.is_symlink() or not (example / "Cargo.toml").is_file():
            continue
        for directory, directories, names in os.walk(example, followlinks=False):
            directories[:] = sorted(
                name for name in directories
                if name not in EXCLUDED and not (Path(directory) / name).is_symlink()
            )
            for name in sorted(names):
                path = Path(directory) / name
                if path.is_symlink():
                    continue
                if path != example / "Cargo.toml" and path.suffix not in {".rs", ".pax"}:
                    continue
                files[path.relative_to(workspace).as_posix()] = path.read_bytes().decode("utf-8")
    if not files:
        raise ValueError("No canonical example sources found")
    return (json.dumps({"schema": 1, "files": files}, ensure_ascii=False,
                       sort_keys=True, indent=2) + "\n").encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = args.workspace / "pax-docs/bundled-example-sources.json"
    expected = snapshot(args.workspace)
    if args.check:
        if not output.is_file() or output.read_bytes() != expected:
            raise SystemExit("Docs example sources are stale; run scripts/sync-docs-examples.py")
        print("Docs example source snapshot is current")
    else:
        output.write_bytes(expected)
        print(f"Wrote {output} ({len(expected)} bytes)")


if __name__ == "__main__":
    main()
