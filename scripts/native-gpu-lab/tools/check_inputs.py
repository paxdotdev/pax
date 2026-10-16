#!/usr/bin/env python3
"""Validate a reviewed image manifest offline; never download or provision."""

import argparse
from datetime import datetime
import json
from pathlib import Path
import re
from urllib.parse import urlsplit

ARTIFACTS = {
    "linux": {"rustup", "grid", "dcv"},
    "windows": {"rustup", "grid", "dcv", "vs_layout", "git", "vc_redist", "winappsdk"},
}


def validate(data, guest_os):
    if data.get("schema") != 1 or data.get("guest_os") != guest_os:
        raise ValueError("Manifest schema or guest_os mismatch")
    if not re.fullmatch(r"\d+\.\d+\.\d+", data.get("rust_version", "")):
        raise ValueError("Pin an exact Rust release")
    if set(data.get("artifacts", {})) != ARTIFACTS[guest_os]:
        raise ValueError(f"Expected artifact IDs: {sorted(ARTIFACTS[guest_os])}")
    for name, artifact in data["artifacts"].items():
        digest = artifact.get("sha256", "")
        if not re.fullmatch(r"[0-9a-f]{64}", digest) or len(set(digest)) == 1:
            raise ValueError(f"{name}: supply a verified SHA256")
        version = artifact.get("version", "")
        if not re.fullmatch(r"[0-9][A-Za-z0-9._+-]*", version):
            raise ValueError(f"{name}: pin an explicit version")
        uri = urlsplit(artifact.get("uri", ""))
        if uri.scheme not in {"https", "s3"} or not uri.netloc or not uri.path.strip("/"):
            raise ValueError(f"{name}: expected an HTTPS URL or exact S3 key")
        if uri.username or uri.password or uri.query or uri.fragment:
            raise ValueError(f"{name}: no credentials, signed URLs, or queries in manifests")
        if re.search(r"latest|REPLACE|REVIEW_REQUIRED", artifact["uri"], re.I):
            raise ValueError(f"{name}: replace placeholders and moving aliases")
    if guest_os == "linux":
        datetime.strptime(data.get("ubuntu_snapshot", ""), "%Y%m%dT%H%M%SZ")
        if data.get("kernel_release") != "base-ami" and not re.fullmatch(r"[0-9][0-9A-Za-z.+-]*-aws", data.get("kernel_release", "")):
            raise ValueError("Pin an AWS kernel release, or inherit it from the exact base AMI with base-ami")
    return data


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("guest_os", choices=ARTIFACTS)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    try:
        validate(json.loads(args.manifest.read_text()), args.guest_os)
    except (ValueError, TypeError, KeyError) as error:
        parser.exit(1, f"Inputs are not ready: {error}\n")
    print("Manifest is structurally complete. Artifact provenance and downloads remain unverified.")
