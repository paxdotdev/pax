#!/usr/bin/env python3
"""Check target-owned spatial capability diagnostics in debug and baked cartridges."""
from pathlib import Path
import os
import subprocess

fixture = Path(__file__).resolve().parent
root = fixture.parents[3]
env = dict(os.environ, CARGO_TARGET_DIR=str(root / "target"))
for release in (False, True):
    for spatial in (False, True):
        command = ["cargo", "check", "--offline", "--manifest-path", str(fixture / "Cargo.toml"),
                   "--target", "wasm32-unknown-unknown"]
        if release:
            command.append("--release")
        if spatial:
            command.extend(["--features", "3d"])
        result = subprocess.run(command, env=env, text=True, capture_output=True)
        if spatial:
            assert result.returncode == 0, result.stderr
        else:
            assert result.returncode != 0, "spatial syntax unexpectedly compiled without 3d"
            assert "Pax spatial properties require the `3d` feature" in result.stderr, result.stderr
        print(f"{'release' if release else 'debug'} 3d={spatial}: expected result", flush=True)
