#!/usr/bin/env python3
"""Wrap an existing x86_64 Pax Linux package in an experimental local Flatpak.

This is a packaging step, not a source build or a runtime installer. The runtime
commit check records packaging provenance; Flatpak applications depend on a
runtime branch and continue receiving that branch's installed updates.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import sys

RUNTIME = "org.gnome.Platform/x86_64/50"


class PackageError(Exception):
    pass


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def relative_resource(package, value):
    if not isinstance(value, str) or not value:
        raise PackageError("Package resources must be nonempty relative paths")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in ("", ".", "..") for part in value.split("/")):
        raise PackageError("Package resources must stay inside the package")
    result = package.joinpath(*path.parts)
    if not result.resolve(strict=True).is_relative_to(package):
        raise PackageError("Package resource escapes the package")
    return result


def inventory(package):
    entries = []
    for path in sorted(package.rglob("*")):
        relative = path.relative_to(package).as_posix()
        mode = path.lstat().st_mode
        if stat.S_ISLNK(mode):
            target = os.readlink(path)
            if os.path.isabs(target) or not path.resolve(strict=True).is_relative_to(package):
                raise PackageError(f"Symlink escapes the package: {relative}")
            entries.append({"path": relative, "symlink": target})
        elif stat.S_ISREG(mode):
            entries.append({"path": relative, "bytes": path.stat().st_size,
                            "executable": bool(mode & 0o111), "sha256": digest(path)})
        elif stat.S_ISDIR(mode):
            entries.append({"path": relative, "directory": True})
        else:
            raise PackageError(f"Unsupported package file type: {relative}")
    return entries


def validate_package(package, launcher):
    recipe = json.loads((package / "pax-app.json").read_text())
    if not isinstance(recipe, dict) or not isinstance(recipe.get("assets"), list):
        raise PackageError("pax-app.json must contain a cartridge and an assets array")
    libraries = [relative_resource(package, recipe.get("cartridge")), package / "libpax_host.so"]
    for resource in recipe["assets"]:
        if not relative_resource(package, resource).is_dir():
            raise PackageError("Each asset root must be a directory")
    executable = package / launcher
    if not executable.is_file() or not os.access(executable, os.X_OK):
        raise PackageError("Launcher must be an executable file in the package root")
    for path in [executable, *libraries]:
        with path.open("rb") as stream:
            header = stream.read(20)
        # ELF64, little endian, EM_X86_64. Reject a wrong-target package before
        # producing an apparently installable bundle with misleading metadata.
        if len(header) != 20 or header[:6] != b"\x7fELF\x02\x01" or header[18:20] != b"\x3e\x00":
            raise PackageError(f"Expected an x86_64 Linux ELF file: {path.name}")


def capture(*args):
    return subprocess.check_output(["flatpak", *args], text=True).strip()


def run(*args):
    subprocess.run(["flatpak", *args], check=True)


def package_flatpak(args):
    source = Path(args.package).resolve(strict=True)
    output = Path(args.output).resolve()
    if output.exists() or Path(args.output).is_symlink():
        raise PackageError("Output must not already exist; use a fresh directory")
    if output.is_relative_to(source) or source.is_relative_to(output):
        raise PackageError("Package input and output directories must not overlap")
    if not re.fullmatch(r"(?:[A-Za-z_][A-Za-z0-9_]*\.){2,}[A-Za-z_][A-Za-z0-9_-]*", args.app_id):
        raise PackageError("Use a reverse-DNS Flatpak app ID, such as org.example.App")
    if not re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9_.-]*", args.launcher):
        raise PackageError("Launcher must be a plain executable filename")
    if not re.fullmatch(r"[0-9a-f]{64}", args.runtime_commit):
        raise PackageError("Runtime commit must be a full lowercase OSTree checksum")
    if args.timestamp < 0:
        raise PackageError("Timestamp must be a nonnegative Unix timestamp")
    timestamp = datetime.datetime.fromtimestamp(args.timestamp, datetime.timezone.utc).isoformat()
    inputs = inventory(source)
    validate_package(source, args.launcher)
    actual_commit = capture("info", "--user", "--show-commit", RUNTIME)
    if actual_commit != args.runtime_commit:
        raise PackageError(f"Installed user runtime differs: expected {args.runtime_commit}, got {actual_commit}")
    provenance = {
        "schema": 1, "appId": args.app_id, "launcher": args.launcher,
        "runtime": RUNTIME, "packagingRuntimeCommit": actual_commit,
        "runtimeOrigin": capture("info", "--user", "--show-origin", RUNTIME),
        "flatpakVersion": capture("--version"), "timestamp": timestamp, "inputs": inputs,
    }
    build = output / "build"
    payload = build / "files/lib/pax"
    payload.parent.mkdir(parents=True)
    shutil.copytree(source, payload, symlinks=True)
    if inventory(payload) != inputs or inventory(source) != inputs:
        raise PackageError("Package changed while copying; output is incomplete")
    (build / "files/bin").mkdir()
    launch = build / "files/bin/pax-launch"
    launch.write_text(f'#!/bin/sh\nset -eu\ncd /app/lib/pax\nexec ./{args.launcher} "$@"\n')
    launch.chmod(0o755)
    (build / "metadata").write_text(
        f"[Application]\nname={args.app_id}\nruntime={RUNTIME}\nsdk=org.gnome.Sdk/x86_64/50\n"
        "\n[Context]\nshared=ipc;\nsockets=wayland;fallback-x11;\ndevices=dri;\n"
        "\n[Environment]\nGSK_RENDERER=ngl\n"
    )
    (output / "package-provenance.json").write_text(json.dumps(provenance, indent=2, sort_keys=True) + "\n")
    run("build-finish", "--command=pax-launch", "--no-inherit-permissions", str(build))
    # Normalize modes as well as dates so the same input contents produce the
    # same local repository commit across checkout umasks and file mtimes.
    for path in sorted(build.rglob("*")):
        if not path.is_symlink():
            path.chmod(0o755 if path.is_dir() or path.stat().st_mode & 0o111 else 0o644)
    # New output means a fresh repository, with no changing parent commit.
    run("build-export", f"--timestamp={timestamp}", str(output / "repo"), str(build), "pilot")
    app_ref = f"app/{args.app_id}/x86_64/pilot"
    # Read the ref in the repository we just created. Unlike bundle hashes,
    # this identifies installed contents independently of the static delta's
    # creation timestamp, without requiring the separate ostree CLI package.
    app_commit = (output / "repo/refs/heads" / app_ref).read_text(encoding="ascii").strip()
    if not re.fullmatch(r"[0-9a-f]{64}", app_commit):
        raise PackageError("Exported application ref does not contain an OSTree checksum")
    bundle = output / f"{args.app_id}.flatpak"
    run("build-bundle", "--arch=x86_64", str(output / "repo"), str(bundle), args.app_id, "pilot")
    report = {"bundle": bundle.name, "sha256": digest(bundle), "bytes": bundle.stat().st_size,
              "appRef": app_ref, "appCommit": app_commit, "runtime": RUNTIME,
              "packagingRuntimeCommit": actual_commit, "inputBytes": sum(item.get("bytes", 0) for item in inputs)}
    (output / "result.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    return report


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", required=True, help="Existing pax-cli Linux Release package directory")
    parser.add_argument("--launcher", required=True, help="Launcher filename inside that directory")
    parser.add_argument("--app-id", required=True)
    parser.add_argument("--runtime-commit", required=True, help="Expected installed user GNOME 50 runtime OSTree checksum")
    parser.add_argument("--timestamp", required=True, type=int, help="Stable Unix timestamp for this application build")
    parser.add_argument("--output", required=True, help="New output directory; never overwritten")
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        print(json.dumps(package_flatpak(parse_args()), indent=2))
    except (PackageError, OSError, ValueError, OverflowError, subprocess.CalledProcessError) as error:
        print(f"Flatpak packaging failed: {error}", file=sys.stderr)
        sys.exit(1)
