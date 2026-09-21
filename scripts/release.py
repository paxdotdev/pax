#!/usr/bin/env python3
"""Prepare and publish Pax releases. No command stages, commits, tags, or pushes.

Requires Python 3.11+, tomlkit, and Cargo 1.93+ (workspace packaging).
See scripts/RELEASING.md for the approval boundary and recovery procedure.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
import urllib.error
import urllib.request

import tomlkit

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
DEPENDENCIES = ("dependencies", "build-dependencies", "dev-dependencies")
REQUIRED_FILES = {
    "pax-compiler": {
        "files/interfaces/web/public/pax-interface-web.js",
        "files/interfaces/web/public/pax-interface-web.css",
        "files/new-project/bundled-examples.paxbundle",
        "files/new-project/AGENTS.md",
        "files/swift/pax-swift-common/Package.swift",
        "files/interfaces/ios/pax-app-ios/pax-app-ios/Assets.xcassets/AppIcon.appiconset/pax-icon-ios-1024.png",
    },
    "pax-docs": {"bundled-example-sources.json", "build.rs", "book/src/SUMMARY.md",
                 "book/src/getting-started.md", "book/src/versions.json"},
}


class ReleaseError(RuntimeError):
    pass


class DocsPublicationError(ReleaseError):
    pass


def run(command, *, cwd=ROOT, capture=False):
    print("+", " ".join(map(str, command)), flush=True)
    return subprocess.run(list(map(str, command)), cwd=cwd, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None).stdout


def docs_module(workspace):
    spec = importlib.util.spec_from_file_location(
        "pax_docs_publisher", workspace / "pax-docs/scripts/publish_versioned_docs.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.BOOK_DIR = workspace / "pax-docs/book"
    module.BOOK_OUTPUT = module.BOOK_DIR / "book"
    module.VERSIONS_MANIFEST = module.BOOK_DIR / "src/versions.json"
    return module


def metadata(workspace):
    return json.loads(run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                          cwd=workspace, capture=True))


def release_packages(meta):
    members = set(meta["workspace_members"])
    packages = {p["name"]: p for p in meta["packages"]
                if p["id"] in members and p.get("publish") != []}
    if not packages:
        raise ReleaseError("Workspace contains no publishable packages")
    for name, package in packages.items():
        if package.get("publish") and "crates-io" not in package["publish"]:
            raise ReleaseError(f"{name} cannot publish to crates.io")
        for dep in package["dependencies"]:
            if dep.get("path") and dep["name"] not in packages and dep["kind"] != "dev":
                raise ReleaseError(f"{name} depends on unpublished local crate {dep['name']}")
    return packages


def publication_order(packages):
    """Include every target and optional/build edge, plus versioned dev edges."""
    visiting, visited, order = [], set(), []

    def visit(name):
        if name in visiting:
            raise ReleaseError("Publication cycle: " + " -> ".join(visiting + [name]))
        if name in visited:
            return
        visiting.append(name)
        after = (packages[name].get("metadata") or {}).get("pax", {}).get("release", {}).get("after", [])
        for prerequisite in after:
            if prerequisite not in packages:
                raise ReleaseError(f"{name} has unknown release prerequisite {prerequisite}")
            visit(prerequisite)
        for dep in sorted(packages[name]["dependencies"], key=lambda d: d["name"]):
            if dep["name"] not in packages:
                continue
            # Cargo drops path-only development dependencies when packaging.
            if dep["kind"] == "dev" and dep.get("path") and dep["req"] == "*":
                continue
            visit(dep["name"])
        visiting.pop()
        visited.add(name)
        order.append(name)

    for name in sorted(packages):
        visit(name)
    return order


def dependency_tables(doc):
    for owner in [doc, doc.get("workspace", {}), *doc.get("target", {}).values()]:
        for kind in DEPENDENCIES:
            if kind in owner:
                yield kind, owner[kind]


def source_files(workspace):
    # Include new source files during candidate review; ignored build trees stay out.
    names = run(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
                cwd=workspace, capture=True)
    return sorted({Path(name) for name in names.split("\0") if name})


def rewrite_versions(workspace, packages, version):
    changed = []
    manifests = [p for p in source_files(workspace) if p.name == "Cargo.toml"]
    local_names = {tomlkit.parse((workspace / p).read_text()).get("package", {}).get("name")
                   for p in manifests}
    for relative in manifests:
        path = workspace / relative
        original = path.read_text()
        doc = tomlkit.parse(original)
        package = doc.get("package", {})
        # All repository examples/fixtures are kept at the coordinated version.
        if package and not isinstance(package.get("version"), dict):
            package["version"] = version
        if "package" in doc.get("workspace", {}) and "version" in doc["workspace"]["package"]:
            doc["workspace"]["package"]["version"] = version
        for kind, table in dependency_tables(doc):
            for alias, dep in table.items():
                name = dep.get("package", alias) if isinstance(dep, dict) else alias
                local_example = isinstance(dep, dict) and dep.get("path") and name in local_names
                if name not in packages and not local_example:
                    if name == "pax-pixels":
                        raise ReleaseError(f"Retired pax-pixels dependency in {relative}")
                    continue
                if isinstance(dep, str):
                    table[alias] = version
                elif not dep.get("workspace"):
                    if kind == "dev-dependencies" and "path" in dep and "version" not in dep:
                        continue
                    dep["version"] = version
        updated = tomlkit.dumps(doc)
        if updated != original:
            path.write_text(updated)
            changed.append(str(relative))
    return changed


def assert_versions(packages, version):
    for name, package in packages.items():
        if package["version"] != version:
            raise ReleaseError(f"{name} is {package['version']}, expected {version}")
        for dep in package["dependencies"]:
            if dep["name"] in packages:
                if dep["kind"] == "dev" and dep.get("path") and dep["req"] == "*":
                    continue
                if dep["req"] != "^" + version:
                    raise ReleaseError(f"{name} -> {dep['name']} has unexpected requirement {dep['req']}")


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_digest(workspace):
    digest = hashlib.sha256()
    extra = {Path("Cargo.lock"), *(Path("pax-compiler") / p for p in REQUIRED_FILES["pax-compiler"])}
    for relative in sorted(set(source_files(workspace)) | extra):
        path = workspace / relative
        digest.update(relative.as_posix().encode() + b"\0")
        digest.update((sha256(path) if path.is_file() else "missing").encode())
    return digest.hexdigest()


def packaging_inputs(workspace, packages):
    """Inventory Cargo's actual inputs, including ignored, explicitly included files."""
    result = {}
    for name, package in sorted(packages.items()):
        root = Path(package["manifest_path"]).parent
        listing = run(["cargo", "package", "-p", name, "--list", "--allow-dirty", "--locked"],
                      cwd=workspace, capture=True)
        entries = []
        for filename in sorted(listing.splitlines()):
            if filename in (".cargo_vcs_info.json", "Cargo.lock"):
                # Git state and the workspace lockfile are bound separately.
                entries.append({"archive_path": filename, "generated": True})
                continue
            source = root / ("Cargo.toml" if filename == "Cargo.toml.orig" else filename)
            # Cargo can copy an explicitly named README/license from outside
            # the package root and rename it to its basename in the archive.
            for field in ("readme", "license_file"):
                external = package.get(field)
                if external and filename == Path(external).name:
                    source = root / external
            if not source.is_file() or not source.resolve().is_relative_to(workspace.resolve()):
                raise ReleaseError(f"Cannot bind packaging input {name}/{filename}: {source}")
            entries.append({"archive_path": filename,
                            "source_path": source.relative_to(workspace).as_posix(),
                            "sha256": sha256(source),
                            "executable": bool(source.stat().st_mode & 0o111),
                            "symlink": os.readlink(source) if source.is_symlink() else None})
        result[name] = entries
    return result


def assert_cargo_version(workspace, record):
    current = run(["cargo", "--version"], cwd=workspace, capture=True).strip()
    if current != record.get("cargo"):
        raise ReleaseError(f"Cargo changed since preparation ({record.get('cargo')} -> {current}); run prepare again.")


def verify_archive_parity(args, record, *, package=None):
    """Repackage without uploading; compare bytes before publication can begin."""
    assert_cargo_version(args.workspace, record)
    packages = release_packages(metadata(args.workspace))
    selected = {name: value for name, value in packages.items() if package is None or name == package}
    if not selected:
        raise ReleaseError(f"No candidate packages selected for parity: {package}")
    expected = {name: record["package_inputs"][name] for name in selected}

    def check_inputs():
        if source_digest(args.workspace) != record["source_sha256"] or packaging_inputs(args.workspace, selected) != expected:
            raise ReleaseError("Packaging inputs changed; run prepare again before publishing.")

    check_inputs()
    # Keep the reviewed archives separate from Cargo's scratch/target output.
    # Compilation was already verified in prepare; this pass checks assembly.
    with tempfile.TemporaryDirectory(prefix="pax-release-parity-") as tmp:
        command = ["cargo", "package", *[flag for item in record["packages"]
                    if item["name"] in selected for flag in ("-p", item["name"])],
                   "--registry", "crates-io", "--locked", "--no-verify", "--target-dir", tmp]
        if args.phase != "publish":
            command.append("--allow-dirty")
        run(command, cwd=args.workspace)
        for item in record["packages"]:
            if item["name"] not in selected:
                continue
            archive = Path(tmp) / "package" / f"{item['name']}-{args.version}.crate"
            if not archive.is_file() or sha256(archive) != item["sha256"]:
                raise ReleaseError(f"Repackaged archive differs from candidate: {item['name']}; nothing uploaded by this check.")
    check_inputs()
    assert_cargo_version(args.workspace, record)
    print(f"Repackaging parity passed for {len(selected)} crate(s); no upload performed.")


def docs_command(args, *, prepared=False):
    command = [sys.executable, args.workspace / "pax-docs/scripts/publish_versioned_docs.py",
               args.version, "--workspace", args.workspace]
    if args.docs_no_latest:
        command.append("--no-latest")
    if prepared:
        command += ["--skip-build", "--bucket", args.docs_bucket,
                    "--distribution-id", args.docs_distribution_id,
                    "--site-url", args.docs_site_url, "--verify-live"]
    else:
        command.append("--no-upload")
    return command


def inspect_archives(workspace, packages, version, destination):
    inventory = []
    destination.mkdir(parents=True, exist_ok=True)
    for name in publication_order(packages):
        archive = workspace / "target/package" / f"{name}-{version}.crate"
        if not archive.is_file():
            raise ReleaseError(f"Missing candidate archive: {archive}")
        with tarfile.open(archive) as tar:
            prefix = f"{name}-{version}/"
            files = {member.name.removeprefix(prefix) for member in tar.getmembers() if member.isfile()}
            missing = REQUIRED_FILES.get(name, set()) - files
            if missing:
                raise ReleaseError(f"{name} missing packaged files: {sorted(missing)}")
            doc = tomllib.loads(tar.extractfile(prefix + "Cargo.toml").read().decode())
            if doc["package"]["name"] != name or doc["package"]["version"] != version:
                raise ReleaseError(f"Archive identity differs from the release plan: {name}")
            for _, table in dependency_tables(doc):
                for alias, dep in table.items():
                    actual = dep.get("package", alias)
                    if "path" in dep or "git" in dep:
                        raise ReleaseError(f"{name} has a non-registry dependency: {alias}")
                    if actual in packages and dep.get("version") != version:
                        raise ReleaseError(f"{name} packaged stale dependency {actual}")
            if any("node_modules/" in file or "/.pax/" in file or "/target/" in file for file in files):
                raise ReleaseError(f"{name} contains transient build files")
            (destination / f"{name}.files.txt").write_text("\n".join(sorted(files)) + "\n")
        retained = destination / "archives" / archive.name
        retained.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(archive, retained)
        inventory.append({"name": name, "version": version, "archive": str(retained),
                          "sha256": sha256(archive), "files": len(files), "bytes": archive.stat().st_size})
    return inventory


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def prepare(args):
    workspace = args.workspace
    packages = release_packages(metadata(workspace))
    publication_order(packages)  # Fail before rewriting on incomplete/cyclic graphs.
    record_path = args.output / "candidate.json"
    record_path.unlink(missing_ok=True)  # A failed retry must not leave a ready stamp.
    changed = rewrite_versions(workspace, packages, args.version)
    print(f"Updated {len(changed)} manifests.")
    run(["bash", "./build-interface.sh"], cwd=workspace / "pax-compiler/files/interfaces/web")
    for script in ("sync-cli-examples.py", "sync-docs-examples.py"):
        run([sys.executable, workspace / "scripts" / script], cwd=workspace)
        run([sys.executable, workspace / "scripts" / script, "--check"], cwd=workspace)
    run(docs_command(args), cwd=workspace)
    packages = release_packages(metadata(workspace))
    assert_versions(packages, args.version)
    expected_source = source_digest(workspace)
    expected_inputs = packaging_inputs(workspace, packages)
    cargo_version = run(["cargo", "--version"], cwd=workspace, capture=True).strip()
    docs = docs_module(workspace)
    docs.validate_build_record(args.version)
    expected_docs = docs.output_digest()
    # Workspace packaging stages unpublished siblings in Cargo's temporary local
    # registry and verifies the actual archives without any crates.io upload.
    selection = [flag for name in publication_order(packages) for flag in ("-p", name)]
    run(["cargo", "package", *selection, "--registry", "crates-io", "--locked",
         "--allow-dirty", "--target-dir", workspace / "target"], cwd=workspace)
    inventory = inspect_archives(workspace, packages, args.version, args.output)
    if source_digest(workspace) != expected_source or docs.output_digest() != expected_docs:
        raise ReleaseError("Sources/docs changed during archive verification; run prepare again.")
    record = {
        "schema": 2, "version": args.version,
        "source_commit": run(["git", "rev-parse", "HEAD"], cwd=workspace, capture=True).strip(),
        "source_dirty": bool(run(["git", "status", "--porcelain"], cwd=workspace, capture=True).strip()),
        "source_sha256": expected_source,
        "docs_sha256": expected_docs,
        "docs_no_latest": args.docs_no_latest,
        "packages": inventory,
        "package_inputs": expected_inputs,
        "cargo": cargo_version,
    }
    smoke_input = args.output / "package-input.json"
    write_json(smoke_input, record)
    run([sys.executable, workspace / "scripts/smoke-release-packages.py", smoke_input,
         "--target-dir", workspace / "target"], cwd=workspace)
    record["package_smoke"] = json.loads((args.output / "package-smoke.json").read_text())
    if source_digest(workspace) != expected_source or docs.output_digest() != expected_docs:
        raise ReleaseError("Sources/docs changed during candidate smoke; run prepare again.")
    verify_archive_parity(args, record)
    write_json(record_path, record)
    print(f"Prepared locally: {record_path}. No publication or git write performed.")


def validate_candidate(args, *, publishing=False):
    path = args.output / "candidate.json"
    if not path.is_file():
        raise ReleaseError("No complete candidate record; run prepare first.")
    record = json.loads(path.read_text())
    if record.get("schema") != 2 or not record.get("package_inputs"):
        raise ReleaseError("Candidate lacks the packaging-input inventory; run prepare again.")
    assert_cargo_version(args.workspace, record)
    if record.get("version") != args.version or record.get("docs_no_latest") != args.docs_no_latest:
        raise ReleaseError("Candidate version/latest policy differs; run prepare again.")
    if not record.get("package_smoke"):
        raise ReleaseError("Candidate package smoke has not completed; run prepare again.")
    if record["source_sha256"] != source_digest(args.workspace):
        raise ReleaseError("Source or bundled artifacts changed; run prepare again.")
    packages = release_packages(metadata(args.workspace))
    assert_versions(packages, args.version)
    if [p["name"] for p in record["packages"]] != publication_order(packages):
        raise ReleaseError("Candidate package graph changed")
    if packaging_inputs(args.workspace, packages) != record["package_inputs"]:
        raise ReleaseError("Packaging inputs changed; run prepare again.")
    for package in record["packages"]:
        if sha256(Path(package["archive"])) != package["sha256"]:
            raise ReleaseError(f"Candidate archive changed: {package['name']}")
    docs = docs_module(args.workspace)
    docs.validate_build_record(args.version)
    if record["docs_sha256"] != docs.output_digest():
        raise ReleaseError("Candidate docs changed")
    if publishing:
        current = run(["git", "rev-parse", "HEAD"], cwd=args.workspace, capture=True).strip()
        if current != args.approved_commit or record["source_commit"] != current or record["source_dirty"]:
            raise ReleaseError("Publish requires preparation from the exact approved clean commit.")
        if run(["git", "status", "--porcelain"], cwd=args.workspace, capture=True).strip():
            raise ReleaseError("Publish requires a clean source checkout.")
    return record


def registry_version(name, version):
    prefix = name[:2] + "/" + name[2:4] if len(name) >= 4 else ("3/" + name[0] if len(name) == 3 else str(len(name)))
    url = f"https://index.crates.io/{prefix}/{name}"
    request = urllib.request.Request(url, headers={"User-Agent": "pax-release-preflight", "Cache-Control": "no-cache"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            entries = response.read().decode().splitlines()
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise
    return next((json.loads(line) for line in entries if json.loads(line)["vers"] == version), None)


def preflight(args):
    if not args.docs_distribution_id:
        raise ReleaseError("Set --docs-distribution-id (required for complete docs publication).")
    docs = docs_module(args.workspace)
    docs.preflight_publication(argparse.Namespace(
        workspace=args.workspace, bucket=args.docs_bucket, distribution_id=args.docs_distribution_id,
        site_url=args.docs_site_url, no_latest=args.docs_no_latest))
    # Never print credentials. Presence is a local configuration check, not a
    # claim that crates.io will authorize every crate or a new crate name.
    credentials = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))) / "credentials.toml"
    token = bool(os.environ.get("CARGO_REGISTRY_TOKEN") or os.environ.get("CARGO_REGISTRIES_CRATES_IO_TOKEN"))
    if credentials.is_file():
        config = tomllib.loads(credentials.read_text())
        token |= bool(config.get("registry", {}).get("token") or
                      config.get("registries", {}).get("crates-io", {}).get("token"))
    if not token:
        raise ReleaseError("No crates.io token configured; run cargo login before publication.")
    print("crates.io token configured (authorization is checked by Cargo during publication).")


def publish(args):
    record = validate_candidate(args, publishing=True)
    preflight(args)  # All read-only checks precede the first irreversible operation.
    # Detect a conflicting partial release anywhere in the set before uploading
    # its first missing dependency. Recheck each entry at its own upload boundary.
    for package in record["packages"]:
        existing = registry_version(package["name"], args.version)
        if existing and (existing["cksum"] != package["sha256"] or existing.get("yanked")):
            raise ReleaseError(f"Existing {package['name']} {args.version} differs from approved artifact or is yanked.")
    verify_archive_parity(args, record)
    validate_candidate(args, publishing=True)
    journal_path = args.output / "publication.json"
    if journal_path.exists():
        journal = json.loads(journal_path.read_text())
        if journal["commit"] != args.approved_commit or journal["version"] != args.version:
            raise ReleaseError("Publication journal belongs to a different release.")
    else:
        journal = {"version": args.version, "commit": args.approved_commit, "crates": {}, "docs": "pending"}
    for package in record["packages"]:
        name = package["name"]
        existing = registry_version(name, args.version)
        if existing:
            if existing["cksum"] != package["sha256"] or existing.get("yanked"):
                raise ReleaseError(f"Existing {name} {args.version} differs from approved artifact or is yanked.")
            journal["crates"][name] = "verified in registry"
            write_json(journal_path, journal)
            continue
        # Now that prerequisites are public, also compare the single-crate
        # registry resolution used by cargo publish with the reviewed archive.
        verify_archive_parity(args, record, package=name)
        journal["crates"][name] = "upload started; recheck registry if interrupted"
        write_json(journal_path, journal)
        # Cargo reassembles the archive. Keep this checkout/toolchain untouched
        # during publication; parity and input checks precede every upload.
        run(["cargo", "publish", "-p", name, "--registry", "crates-io", "--locked"], cwd=args.workspace)
        existing = registry_version(name, args.version)
        if not existing or existing["cksum"] != package["sha256"]:
            raise ReleaseError(f"Registry checksum/visibility not confirmed for {name}; stop and inspect before retrying.")
        journal["crates"][name] = "verified in registry"
        write_json(journal_path, journal)
        time.sleep(args.publish_interval)
    journal["docs"] = "upload started; retry exact prepared tree if interrupted"
    write_json(journal_path, journal)
    try:
        run(docs_command(args, prepared=True), cwd=args.workspace)
    except (subprocess.CalledProcessError, OSError) as error:
        journal["docs"] = "failed; crate publication may be complete"
        write_json(journal_path, journal)
        raise DocsPublicationError("Docs publication/verification failed; rerun the same approved publish command. " + str(error)) from error
    journal["docs"] = "uploaded, invalidation completed, HTTPS content verified"
    write_json(journal_path, journal)
    print("Crates and docs published. Complete the public-channel workstation/iOS/AI smoke handoffs before closing the release.")


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["plan", "prepare", "verify", "preflight", "publish"])
    parser.add_argument("version")
    parser.add_argument("--workspace", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path, help="Ignored local evidence directory (default: target/release-candidate/VERSION)")
    parser.add_argument("--docs-no-latest", action="store_true")
    parser.add_argument("--docs-bucket", default=os.environ.get("PAX_DOCS_S3_BUCKET", "docs.pax.dev"))
    parser.add_argument("--docs-distribution-id", default=os.environ.get("PAX_DOCS_CLOUDFRONT_DISTRIBUTION_ID"))
    parser.add_argument("--docs-site-url", default="https://docs.pax.dev")
    parser.add_argument("--approved-commit", help="Exact full commit approved by Zack; required for publish")
    parser.add_argument("--publish-interval", type=int, default=60, help="Seconds between uploads (default: 60)")
    args = parser.parse_args(argv)
    args.workspace = args.workspace.resolve()
    args.output = (args.output or args.workspace / "target/release-candidate" / args.version).resolve()
    docs_module(args.workspace).release_version(args.version)
    if args.phase == "publish" and not args.approved_commit:
        parser.error("publish requires --approved-commit after Zack's explicit approval")
    if args.publish_interval < 0:
        parser.error("--publish-interval must be nonnegative")
    return args


def main(argv=None):
    args = parse_args(argv)
    if args.phase == "plan":
        meta = metadata(args.workspace)
        packages = release_packages(meta)
        print(json.dumps({"version": args.version, "publication_order": publication_order(packages),
                          "excluded": [p["name"] for p in meta["packages"] if p.get("publish") == []]}, indent=2))
    elif args.phase == "prepare":
        prepare(args)
    elif args.phase == "verify":
        record = validate_candidate(args)
        verify_archive_parity(args, record)
        print("Candidate source, crate archives, and docs match the completed preparation record.")
    elif args.phase == "preflight":
        preflight(args)
    else:
        publish(args)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ReleaseError, subprocess.CalledProcessError, OSError, ValueError) as error:
        print(f"Release stopped: {error}", file=sys.stderr)
        raise SystemExit(3 if isinstance(error, DocsPublicationError) else 1)
