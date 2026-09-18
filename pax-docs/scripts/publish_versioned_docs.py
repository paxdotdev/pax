#!/usr/bin/env python3
"""Build and publish a versioned Pax docs site.

The version manifest is deliberately explicit instead of discovered from S3:
CloudFront/S3 listings are not browser-friendly, and docs should expose only
versions we intentionally publish.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOOK_DIR = ROOT / "pax-docs" / "book"
BOOK_OUTPUT = BOOK_DIR / "book"
VERSIONS_MANIFEST = BOOK_DIR / "src" / "versions.json"
DEFAULT_BUCKET = os.environ.get("PAX_DOCS_S3_BUCKET", "docs.pax.dev")
DEFAULT_DISTRIBUTION_ID = os.environ.get("PAX_DOCS_CLOUDFRONT_DISTRIBUTION_ID")
BUILD_RECORD = "_pax_docs_build.json"
SEMVER = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
)


def main() -> int:
    global BOOK_DIR, BOOK_OUTPUT, VERSIONS_MANIFEST
    args = parse_args()
    workspace = args.workspace.resolve()
    args.workspace = workspace
    BOOK_DIR = workspace / "pax-docs" / "book"
    BOOK_OUTPUT = BOOK_DIR / "book"
    VERSIONS_MANIFEST = BOOK_DIR / "src" / "versions.json"

    # The release script uses this combination before packaging/committing.
    # It deliberately does not require, stamp, or validate a built site.
    if args.skip_build and args.no_upload:
        update_versions_manifest(args.version, promote=not args.no_latest)
        print("Version manifest updated only; no build, validation, or upload performed.")
        return 0

    if args.skip_build:
        validate_build_record(args.version)

    update_versions_manifest(args.version, promote=not args.no_latest)

    if not args.skip_build:
        build_docs(workspace, args.skip_examples)

    validate_output()
    # A reused build may carry an older catalog. Publish exactly the catalog
    # just prepared, even when no mdBook rebuild was requested.
    (BOOK_OUTPUT / "versions.json").write_bytes(VERSIONS_MANIFEST.read_bytes())
    if not args.skip_build:
        write_build_record(args.version)

    if args.no_upload:
        print(f"Docs built and validated locally at {BOOK_OUTPUT}; upload skipped.")
        return 0

    publish_docs(args)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build and publish versioned Pax docs")
    parser.add_argument("version", type=release_version, help="Release SemVer, e.g. 0.39.0 (no v prefix)")
    parser.add_argument(
        "--workspace",
        type=Path,
        default=ROOT,
        help="Path to the Pax workspace (defaults to this checkout)",
    )
    parser.add_argument(
        "--bucket",
        default=DEFAULT_BUCKET,
        help="S3 bucket for docs output (default: PAX_DOCS_S3_BUCKET or docs.pax.dev)",
    )
    parser.add_argument(
        "--distribution-id",
        default=DEFAULT_DISTRIBUTION_ID,
        help="CloudFront distribution to invalidate (default: PAX_DOCS_CLOUDFRONT_DISTRIBUTION_ID)",
    )
    parser.add_argument(
        "--skip-examples",
        action="store_true",
        help="Skip rebuilding runnable example bundles while building docs",
    )
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="Reuse a validated publisher build of this version; with --no-upload, only update the source manifest",
    )
    parser.add_argument(
        "--no-upload",
        action="store_true",
        help="Build the versioned docs locally without uploading to S3",
    )
    parser.add_argument(
        "--no-latest",
        action="store_true",
        help="Publish only /<version>/ and versions.json, leaving the root latest docs untouched",
    )
    return parser.parse_args()


def update_versions_manifest(version: str, *, promote: bool = True) -> None:
    manifest = read_manifest()
    today = dt.datetime.now(dt.timezone.utc).date().isoformat()
    existing = normalize_version_entries(manifest.get("versions", []))

    by_version = {entry["version"]: entry for entry in existing}
    entry = by_version.get(version, {})
    entry.update(
        {
            "version": version,
            "label": entry.get("label") or version,
            "path": entry.get("path") or f"/{version}/",
            "released": entry.get("released") or today,
        }
    )
    by_version[version] = entry

    versions = sorted(by_version.values(), key=lambda item: version_key(item["version"]), reverse=True)
    manifest = {
        "schema": 1,
        "latest": version if promote else manifest.get("latest"),
        "versions": versions,
    }
    VERSIONS_MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    VERSIONS_MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Updated {VERSIONS_MANIFEST} for docs version {version}.")


def read_manifest() -> dict:
    if not VERSIONS_MANIFEST.exists():
        return {"schema": 1, "latest": None, "versions": []}
    return json.loads(VERSIONS_MANIFEST.read_text())


def normalize_version_entries(raw_entries: list) -> list[dict]:
    entries = []
    seen = set()
    for raw in raw_entries:
        if isinstance(raw, str):
            raw = {"version": raw}
        if not isinstance(raw, dict):
            continue
        version = raw.get("version")
        if not version or version in seen:
            continue
        seen.add(version)
        entries.append(dict(raw))
    return entries


def version_key(version: str) -> tuple:
    match = SEMVER.fullmatch(version)
    if match is None:
        raise ValueError(f"Invalid release SemVer: {version!r}")
    major, minor, patch, prerelease, _build = match.groups()
    identifiers = []
    for part in prerelease.split(".") if prerelease else []:
        if part.isdigit():
            if len(part) > 1 and part.startswith("0"):
                raise ValueError(f"Leading zero in SemVer prerelease: {version!r}")
            identifiers.append((0, int(part)))
        else:
            identifiers.append((1, part))
    # Stable releases outrank prereleases; build metadata has no precedence.
    return (int(major), int(minor), int(patch), prerelease is None, tuple(identifiers))


def release_version(value: str) -> str:
    try:
        version_key(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError(str(error)) from error
    return value


def build_docs(workspace: Path, skip_examples: bool) -> None:
    docs_args = ["cargo", "run", "-p", "pax-cli", "--", "docs", "build"]
    if skip_examples:
        docs_args.append("--skip-examples")
    run(docs_args, cwd=workspace)
    run(["mdbook", "build", str(BOOK_DIR), "--dest-dir", str(BOOK_OUTPUT)], cwd=workspace)
    run([
        "node", str(workspace / "pax-docs/scripts/check_publication_output.mjs"),
        str(BOOK_OUTPUT),
    ], cwd=workspace)


def validate_output() -> None:
    required = ["index.html", "getting-started.html", "theme/pax-version.js"]
    missing = [path for path in required if not (BOOK_OUTPUT / path).is_file()]
    if missing:
        raise SystemExit(f"Incomplete docs output at {BOOK_OUTPUT}: missing {', '.join(missing)}")
    collector = ExampleCollector()
    for page in BOOK_OUTPUT.rglob("*.html"):
        collector.feed(page.read_text())
        collector.close()
        collector.reset()
    for name in sorted(collector.examples):
        validate_example(name)


class ExampleCollector(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.examples: set[str] = set()

    def handle_starttag(self, tag: str, attrs: list) -> None:
        if tag == "pax-example":
            name = dict(attrs).get("path") or ""
            if not name or "\\" in name or any(part in ("", ".", "..") for part in name.split("/")):
                raise SystemExit(f"Invalid embedded example path: {name!r}")
            self.examples.add(name)


def validate_example(name: str) -> None:
    directory = BOOK_OUTPUT / "_pax_examples" / name
    try:
        manifest = json.loads((directory / "manifest.json").read_text())
    except (OSError, ValueError) as error:
        raise SystemExit(f"Missing/invalid example manifest for {name}") from error
    if not isinstance(manifest, dict):
        raise SystemExit(f"Invalid example manifest for {name}")
    app = manifest.get("app") or {}
    if not app.get("available") or app.get("build_error"):
        raise SystemExit(f"Example {name} has no successful web build: {app.get('build_error')}")
    if not manifest.get("built_fingerprint") or manifest["built_fingerprint"] != manifest.get("source_fingerprint"):
        raise SystemExit(f"Example {name} has stale compiled source; rebuild without --skip-examples.")
    if app.get("index") != "app/index.html" or not (directory / "app/index.html").is_file():
        raise SystemExit(f"Missing example entry document for {name}")
    if not any((directory / "app").rglob("*.wasm")):
        raise SystemExit(f"Missing compiled Wasm for example {name}")
    if not manifest.get("files") or not all(isinstance(file.get("contents"), str) for file in manifest["files"]):
        raise SystemExit(f"Missing embedded example source for {name}")


def output_digest() -> str:
    digest = hashlib.sha256()
    for path in sorted(BOOK_OUTPUT.rglob("*")):
        if path.is_symlink():
            raise SystemExit(f"Docs output must not contain symlinks: {path}")
        if not path.is_file():
            continue
        relative = path.relative_to(BOOK_OUTPUT).as_posix()
        # Catalog changes do not require rebuilding the articles or examples.
        if relative in (BUILD_RECORD, "versions.json"):
            continue
        digest.update(relative.encode() + b"\0")
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
        digest.update(b"\0")
    return digest.hexdigest()


def write_build_record(version: str) -> None:
    record = {"schema": 1, "version": version, "content_sha256": output_digest()}
    (BOOK_OUTPUT / BUILD_RECORD).write_text(json.dumps(record, indent=2) + "\n")


def validate_build_record(version: str) -> None:
    validate_output()
    path = BOOK_OUTPUT / BUILD_RECORD
    try:
        record = json.loads(path.read_text())
    except (OSError, ValueError) as error:
        raise SystemExit("Missing/invalid docs build record; run the publisher without --skip-build first.") from error
    if not isinstance(record, dict) or record.get("schema") != 1 or record.get("version") != version:
        raise SystemExit(f"Docs build is not recorded for {version}; rebuild without --skip-build.")
    if record.get("content_sha256") != output_digest():
        raise SystemExit("Docs output changed since its recorded build; rebuild without --skip-build.")


def publish_docs(args: argparse.Namespace) -> None:
    validate_build_record(args.version)

    version_prefix = f"s3://{args.bucket}/{args.version}/"
    root_prefix = f"s3://{args.bucket}/"
    # Copy every file so unchanged objects also receive the new cache policy.
    # A sync alone skips those objects and can retain a previous immutable header.
    copy_tree(version_prefix, args.workspace)
    run(
        [
            "aws",
            "s3",
            "sync",
            str(BOOK_OUTPUT),
            version_prefix,
            "--delete",
            "--cache-control",
            "no-cache",
            "--no-follow-symlinks",
        ],
        cwd=args.workspace,
    )

    if not args.no_latest:
        # Never delete at the bucket root: it also contains historical versions.
        # The root catalog is promoted only after both content uploads succeed.
        copy_tree(root_prefix, args.workspace, exclude_catalog=True)

    run(
        [
            "aws",
            "s3",
            "cp",
            str(BOOK_OUTPUT / "versions.json"),
            f"s3://{args.bucket}/versions.json",
            "--cache-control",
            "no-cache",
        ],
        cwd=args.workspace,
    )

    if args.distribution_id:
        paths = [f"/{args.version}/*", "/versions.json"] if args.no_latest else ["/*"]
        run(
            [
                "aws",
                "cloudfront",
                "create-invalidation",
                "--distribution-id",
                args.distribution_id,
                "--paths",
                *paths,
            ],
            cwd=args.workspace,
        )


def copy_tree(destination: str, workspace: Path, *, exclude_catalog: bool = False) -> None:
    command = [
        "aws", "s3", "cp", str(BOOK_OUTPUT), destination, "--recursive",
        "--no-follow-symlinks", "--cache-control", "no-cache",
    ]
    if exclude_catalog:
        command.extend(["--exclude", "versions.json"])
    run(command, cwd=workspace)


def run(command: list[str], cwd: Path) -> None:
    print("+", " ".join(command))
    subprocess.run(command, cwd=cwd, check=True)


if __name__ == "__main__":
    raise SystemExit(main())
