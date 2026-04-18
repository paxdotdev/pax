#!/usr/bin/env python3
"""Build and publish a versioned Pax docs site.

The version manifest is deliberately explicit instead of discovered from S3:
CloudFront/S3 listings are not browser-friendly, and docs should expose only
versions we intentionally publish.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOOK_DIR = ROOT / "pax-docs" / "book"
BOOK_OUTPUT = BOOK_DIR / "book"
VERSIONS_MANIFEST = BOOK_DIR / "src" / "versions.json"
DEFAULT_BUCKET = os.environ.get("PAX_DOCS_S3_BUCKET", "docs.pax.dev")
DEFAULT_DISTRIBUTION_ID = os.environ.get("PAX_DOCS_CLOUDFRONT_DISTRIBUTION_ID")


def main() -> int:
    args = parse_args()
    workspace = args.workspace.resolve()
    if workspace != ROOT:
        global BOOK_DIR, BOOK_OUTPUT, VERSIONS_MANIFEST
        BOOK_DIR = workspace / "pax-docs" / "book"
        BOOK_OUTPUT = BOOK_DIR / "book"
        VERSIONS_MANIFEST = BOOK_DIR / "src" / "versions.json"

    update_versions_manifest(args.version)

    if not args.skip_build:
        build_docs(workspace, args.skip_examples)

    if args.no_upload:
        print("Docs built locally; upload skipped.")
        return 0

    publish_docs(args)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build and publish versioned Pax docs")
    parser.add_argument("version", help="Release version, e.g. 0.39.0")
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
        help="Only update the manifest and publish the existing mdBook output",
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


def update_versions_manifest(version: str) -> None:
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
        "latest": version,
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
    parts = re.split(r"[.+-]", version.lstrip("v"))
    key = []
    for part in parts:
        if part.isdigit():
            key.append((1, int(part)))
        else:
            key.append((0, part))
    return tuple(key)


def build_docs(workspace: Path, skip_examples: bool) -> None:
    docs_args = ["cargo", "run", "-p", "pax-cli", "--", "docs", "build"]
    if skip_examples:
        docs_args.append("--skip-examples")
    run(docs_args, cwd=workspace)
    run(["mdbook", "build", str(BOOK_DIR)], cwd=workspace)


def publish_docs(args: argparse.Namespace) -> None:
    if not BOOK_OUTPUT.exists():
        raise SystemExit(f"mdBook output not found: {BOOK_OUTPUT}")

    version_prefix = f"s3://{args.bucket}/{args.version}/"
    root_prefix = f"s3://{args.bucket}/"
    run(
        [
            "aws",
            "s3",
            "sync",
            str(BOOK_OUTPUT),
            version_prefix,
            "--delete",
            "--cache-control",
            "public,max-age=31536000,immutable",
        ],
        cwd=args.workspace,
    )

    if not args.no_latest:
        # Intentionally avoid --delete at the bucket root so immutable version
        # directories are never removed by a latest-docs deploy.
        run(
            [
                "aws",
                "s3",
                "sync",
                str(BOOK_OUTPUT),
                root_prefix,
                "--cache-control",
                "no-cache",
            ],
            cwd=args.workspace,
        )

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
        paths = ["/versions.json"] if args.no_latest else ["/versions.json", "/*"]
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


def run(command: list[str], cwd: Path) -> None:
    print("+", " ".join(command))
    subprocess.run(command, cwd=cwd, check=True)


if __name__ == "__main__":
    raise SystemExit(main())
