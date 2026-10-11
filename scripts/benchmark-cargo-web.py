#!/usr/bin/env python3
"""Measure a Rust-cold and two unchanged warm CLI web builds without cleaning caches.

Run in a Pax checkout with workstation tools and registry/tool caches available.
The target directory must be empty; generated .pax files and external tool caches
are intentionally retained. CLI installation/build is excluded; supply a built
CLI from this checkout. No server or browser is started.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pax-cli", required=True, type=Path)
    parser.add_argument("--target-dir", required=True, type=Path)
    parser.add_argument("--logs", required=True, type=Path)
    parser.add_argument("--package", default="pax-logo")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    cli = args.pax_cli.resolve()
    if not cli.is_file() or not os.access(cli, os.X_OK):
        parser.error("--pax-cli must name an executable built from this checkout")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--offline", "--format-version", "1"],
        cwd=root, text=True))
    packages = [p for p in metadata["packages"] if p["name"] == args.package]
    if len(packages) != 1:
        parser.error(f"--package must select one workspace package: {args.package}")
    project = Path(packages[0]["manifest_path"]).parent
    target = args.target_dir.resolve()
    logs = args.logs.resolve()
    if target.exists() and (not target.is_dir() or any(target.iterdir())):
        parser.error("--target-dir must be absent or empty for a cold Rust build")
    logs.mkdir(parents=True, exist_ok=True)
    if any((logs / name).exists() for name in ["cold.log", "warm.log", "warm-repeat.log", "results.json"]):
        parser.error("--logs already contains benchmark results; select a fresh directory")
    environment = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_NET_OFFLINE="true")
    command = [str(cli), "build", "--path", str(project), "--target", "web"]
    results = {
        "command": command,
        "source": str(root),
        "target_dir": str(target),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "cargo_profile_overrides": {key: value for key, value in environment.items()
                                    if key.startswith("CARGO_PROFILE_")},
        "runs": [],
    }
    for kind in ["cold", "warm", "warm-repeat"]:
        start = time.monotonic()
        with (logs / f"{kind}.log").open("x") as log:
            process = subprocess.run(command, cwd=root, env=environment,
                                     stdout=log, stderr=subprocess.STDOUT)
        result = {"run": kind, "seconds": round(time.monotonic() - start, 3),
                  "returncode": process.returncode}
        results["runs"].append(result)
        (logs / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        print(json.dumps(result), flush=True)
        if process.returncode:
            raise SystemExit(f"Build failed; see {logs / (kind + '.log')}")


if __name__ == "__main__":
    main()
