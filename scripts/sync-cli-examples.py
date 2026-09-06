#!/usr/bin/env python3
"""Build the deterministic crate-owned bundle consumed by `pax-cli create`."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import struct
import sys

try:
    import tomllib
    TomlDecodeError = tomllib.TOMLDecodeError
except ModuleNotFoundError:
    # release.py already depends on tomlkit, and macOS still ships Python 3.9.
    import tomlkit as tomllib
    from tomlkit.exceptions import ParseError as TomlDecodeError


ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "examples" / "bundled-cli-examples.toml"
OUTPUT = ROOT / "pax-compiler" / "files" / "new-project" / "bundled-examples.paxbundle"
MAGIC = b"PAXBUNDLE1\n"
EXCLUDED_PARTS = {".git", ".pax", "target", "__pycache__", "node_modules"}
EXCLUDED_NAMES = {".DS_Store", "Cargo.lock"}


def load_registry() -> dict:
    registry = tomllib.loads(REGISTRY.read_text())
    examples = registry.get("examples", [])
    names = [entry.get("name") for entry in examples]
    if not examples or len(names) != len(set(names)):
        raise ValueError("registry must contain uniquely named examples")
    if registry.get("default") not in names:
        raise ValueError("registry default must name a curated example")

    for entry in examples:
        name = entry["name"]
        if not name or any(char not in "abcdefghijklmnopqrstuvwxyz0123456789-" for char in name):
            raise ValueError(f"invalid example name: {name!r}")
        expected = PurePosixPath("examples", "src", name)
        source = PurePosixPath(entry["source"])
        if source != expected:
            raise ValueError(f"{name}: source must be {expected}")
        if not (ROOT / source).is_dir():
            raise ValueError(f"{name}: source directory does not exist")
        for field in ("description", "caveats"):
            if not entry.get(field):
                raise ValueError(f"{name}: missing {field}")
    return registry


def included_files(source: Path) -> list[tuple[str, bytes, int]]:
    files: list[tuple[str, bytes, int]] = []
    for path in sorted(source.rglob("*")):
        relative = path.relative_to(source)
        if any(part in EXCLUDED_PARTS for part in relative.parts):
            continue
        if path.is_symlink():
            raise ValueError(f"symlinks are not supported in bundled examples: {path}")
        if not path.is_file() or path.name in EXCLUDED_NAMES or path.suffix == ".pyc":
            continue
        mode = 0o755 if os.access(path, os.X_OK) else 0o644
        files.append((relative.as_posix(), path.read_bytes(), mode))
    if not any(path == "Cargo.toml" for path, _, _ in files):
        raise ValueError(f"{source}: missing Cargo.toml")
    return files


def example_digest(files: list[tuple[str, bytes, int]]) -> str:
    digest = hashlib.sha256()
    for path, data, mode in files:
        digest.update(path.encode())
        digest.update(b"\0")
        digest.update(str(mode).encode())
        digest.update(b"\0")
        digest.update(data)
    return digest.hexdigest()


def build_bundle() -> bytes:
    source_registry = load_registry()
    all_files: list[tuple[str, bytes, int]] = []
    bundled_examples = []
    for entry in source_registry["examples"]:
        files = included_files(ROOT / entry["source"])
        bundled_examples.append(
            {
                "name": entry["name"],
                "description": entry["description"],
                "caveats": entry["caveats"],
                "sha256": example_digest(files),
            }
        )
        all_files.extend((f"{entry['name']}/{path}", data, mode) for path, data, mode in files)

    manifest = json.dumps(
        {
            "generated": "DO NOT EDIT: run scripts/sync-cli-examples.py",
            "default": source_registry["default"],
            "examples": bundled_examples,
        },
        sort_keys=True,
        separators=(",", ":"),
    ).encode()

    payload = io.BytesIO()
    payload.write(MAGIC)
    payload.write(struct.pack(">I", len(manifest)))
    payload.write(manifest)
    payload.write(struct.pack(">I", len(all_files)))
    for path, data, mode in all_files:
        encoded_path = path.encode()
        payload.write(struct.pack(">IQI", len(encoded_path), len(data), mode))
        payload.write(encoded_path)
        payload.write(data)

    compressed = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=compressed, mtime=0, compresslevel=9) as archive:
        archive.write(payload.getvalue())
    return compressed.getvalue()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail when the checked-in bundle is stale")
    args = parser.parse_args()
    try:
        generated = build_bundle()
    except (KeyError, OSError, ValueError, TomlDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != generated:
            print(f"error: {OUTPUT.relative_to(ROOT)} is stale; run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"current: {OUTPUT.relative_to(ROOT)}")
        return 0

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_bytes(generated)
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(generated)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
