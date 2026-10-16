#!/usr/bin/env python3
"""Read-only availability check before paying for an image builder.

This does not replace the guest's SHA256 and APT signature verification.
"""

import argparse
import json
from pathlib import Path
import subprocess
from urllib.parse import urlsplit
from urllib.request import Request, urlopen

from check_inputs import validate


def check_sources(data, aws, open_url=urlopen, windows_runtime_only=False):
    validate(data, data["guest_os"])
    if windows_runtime_only and data["guest_os"] != "windows":
        raise ValueError("Runtime-only source checks require a Windows manifest")
    if data["guest_os"] == "linux":
        for suite in ("noble", "noble-updates", "noble-security"):
            url = f'https://snapshot.ubuntu.com/ubuntu/{data["ubuntu_snapshot"]}/dists/{suite}/InRelease'
            try:
                with open_url(Request(url), timeout=30) as response:
                    if not response.read(64).startswith(b"-----BEGIN PGP SIGNED MESSAGE-----"):
                        raise ValueError("Expected signed APT metadata")
            except Exception as error:
                raise ValueError(f"Ubuntu snapshot {suite} metadata is unavailable; do not launch a Linux builder") from error
    for name, artifact in data["artifacts"].items():
        if windows_runtime_only and name not in {"vc_redist", "dcv", "winappsdk"}:
            continue
        uri = urlsplit(artifact["uri"])
        if uri.scheme == "s3":
            result = aws("s3api", "head-object", "--bucket", uri.netloc, "--key", uri.path.lstrip("/"))
            if result["ContentLength"] <= 0:
                raise ValueError(f"{name}: S3 object is empty")
            # The private archive is uploaded only after approval, with its
            # locally verified digest recorded as object metadata.
            if name == "vs_layout" and result.get("Metadata", {}).get("sha256") != artifact["sha256"]:
                raise ValueError("vs_layout: staged archive SHA256 metadata is missing or incorrect")
        else:
            try:
                with open_url(Request(artifact["uri"], method="HEAD"), timeout=30) as response:
                    if urlsplit(response.url).scheme != "https":
                        raise ValueError("Artifact redirected away from HTTPS")
                    if response.headers.get("Content-Length") == "0":
                        raise ValueError("Artifact is empty")
            except Exception as error:
                raise ValueError(f"{name}: HTTPS artifact is unavailable") from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--aws", default="aws")
    parser.add_argument("--windows-runtime-only", action="store_true", help="Check only the three installers used by setup-windows.ps1 -RuntimeOnly")
    args = parser.parse_args()

    def aws(*command):
        return json.loads(subprocess.check_output(
            [args.aws, "--profile", "pax", "--region", "us-west-2", "--no-cli-pager", "--output", "json", *command], text=True))

    check_sources(json.loads(args.manifest.read_text()), aws, windows_runtime_only=args.windows_runtime_only)
    print("Sources are available. Guest checksum/signature checks are still required.")


if __name__ == "__main__":
    main()
