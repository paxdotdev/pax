#!/usr/bin/env python3
"""Build a caller-owned Swift host against a locally built Pax application.

This is a fixture bundler, not the production packaging path. It reuses the
cartridge, resources and host framework, replacing only the fixture executable
before signing. Usage: build.py SOURCE.app OUTPUT.app [macos|ios-simulator] [probe]
"""
import pathlib
import plistlib
import shutil
import subprocess
import sys

source, output = map(pathlib.Path, sys.argv[1:3])
mobile = len(sys.argv) > 3 and sys.argv[3] == "ios-simulator"
probe = "probe" in sys.argv[3:]
if output.exists():
    raise SystemExit(f"Refusing to replace an existing fixture: {output}")
shutil.copytree(source, output, symlinks=True)
base = output if mobile else output / "Contents"
info = base / "Info.plist"
metadata = plistlib.loads(info.read_bytes())
original_executable = metadata["CFBundleExecutable"]
metadata["CFBundleIdentifier"] = "dev.pax.host-lifecycle-probe" if probe else "dev.pax.host-embedding-proof"
metadata["CFBundleName"] = "Pax host embedding"
metadata["CFBundleDisplayName"] = "Pax host embedding"
metadata["CFBundleExecutable"] = "HostEmbeddingProof"
info.write_bytes(plistlib.dumps(metadata))
executable_dir = base if mobile else base / "MacOS"
(executable_dir / original_executable).unlink()
frameworks = base / "Frameworks"
sdk = "iphonesimulator" if mobile else "macosx"
target = "arm64-apple-ios15.0-simulator" if mobile else "arm64-apple-macos12.0"
sdk_path = subprocess.check_output(["xcrun", "--sdk", sdk, "--show-sdk-path"], text=True).strip()
subprocess.run([
    "xcrun", "--sdk", sdk, "swiftc", "-parse-as-library", "-sdk", sdk_path, "-target", target,
    str(pathlib.Path(__file__).with_name("LifecycleProbe.swift" if probe else "Host.swift")),
    "-Xcc", "-isysroot", "-Xcc", sdk_path,
    "-import-objc-header", str(pathlib.Path(__file__).resolve().parents[3] / "pax-host/include/pax_host.h"),
    "-F", str(frameworks),
    "-framework", "PaxHost", "-Xlinker", "-rpath", "-Xlinker",
    "@executable_path/Frameworks" if mobile else "@executable_path/../Frameworks",
    "-o", str(executable_dir / "HostEmbeddingProof"),
], check=True)
subprocess.run(["codesign", "--force", "--sign", "-", str(output)], check=True)
print(output)
