"""Release boundary tests: no network/uploads; Git writes only in temporary fixtures."""
import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("release", ROOT / "scripts/release.py")
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)


def package(name, dependencies=(), publish=None):
    return {"id": name, "name": name, "version": "0.39.0", "publish": publish,
            "dependencies": list(dependencies)}


def dependency(name, kind=None, req="^0.39.0", **extra):
    return {"name": name, "kind": kind, "req": req, **extra}


class ReleaseTests(unittest.TestCase):
    def test_discovers_independent_docs_new_crates_and_optional_target_build_edges(self):
        crates = [package("cli", [dependency("docs"), dependency("gpu", target="cfg(wasm32)", optional=True),
                                   dependency("build", kind="build")]),
                  package("docs"), package("gpu"), package("build"), package("pax", publish=[])]
        meta = {"workspace_members": [p["id"] for p in crates], "packages": crates}
        packages = release.release_packages(meta)
        order = release.publication_order(packages)
        self.assertEqual(set(order), {"cli", "docs", "gpu", "build"})
        for dep in ("docs", "gpu", "build"):
            self.assertLess(order.index(dep), order.index("cli"))

    def test_rejects_runtime_and_versioned_dev_cycles_but_omits_path_only_dev_edges(self):
        a = package("a", [dependency("b")])
        b = package("b", [dependency("a", kind="dev", path="/a")])
        with self.assertRaisesRegex(release.ReleaseError, "Publication cycle"):
            release.publication_order({"a": a, "b": b})
        b["dependencies"][0]["req"] = "*"
        self.assertEqual(release.publication_order({"a": a, "b": b}), ["b", "a"])

    def test_rejects_unpublished_runtime_dependency(self):
        a = package("a", [dependency("private", path="/private")])
        with self.assertRaisesRegex(release.ReleaseError, "unpublished"):
            release.release_packages({"workspace_members": ["a"], "packages": [a]})

    def test_cli_release_waits_for_generated_project_dependency(self):
        cli = package("cli")
        cli["metadata"] = {"pax": {"release": {"after": ["kit"]}}}
        self.assertEqual(release.publication_order({"cli": cli, "kit": package("kit")}), ["kit", "cli"])
        with self.assertRaisesRegex(release.ReleaseError, "unknown release prerequisite"):
            release.publication_order({"cli": cli})

    def test_rewrites_all_dependency_forms_even_when_package_version_already_matches(self):
        with tempfile.TemporaryDirectory() as tmp:
            workspace = Path(tmp)
            manifest = workspace / "Cargo.toml"
            manifest.write_text('''[package]
name = "fixture"
version = "0.39.0"
# keep this comment
[dependencies]
pax-kit = "0.38.3"
renamed = { package = "pax-runtime", version = "0.38.3", path = "../runtime" }
inherited = { package = "pax-message", workspace = true }
[build-dependencies.pax-message]
version = "0.38.3"
[dev-dependencies]
pax-runtime = { path = "../runtime" }
[target.'cfg(windows)'.dependencies.pax-kit]
version = "0.38.3"
[workspace.dependencies]
pax-message = { version = "0.38.3" }
''')
            with patch.object(release, "source_files", return_value=[Path("Cargo.toml")]):
                changed = release.rewrite_versions(workspace, {"pax-kit", "pax-runtime", "pax-message"}, "0.39.0")
                self.assertEqual(changed, ["Cargo.toml"])
                self.assertEqual(release.rewrite_versions(workspace, {"pax-kit", "pax-runtime", "pax-message"}, "0.39.0"), [])
            text = manifest.read_text()
            self.assertNotIn("0.38.3", text)
            self.assertIn("# keep this comment", text)
            self.assertEqual(release.tomllib.loads(text)["dev-dependencies"]["pax-runtime"], {"path": "../runtime"})

    def test_prepare_is_local_and_builds_docs_before_cargo_packages(self):
        args = release.parse_args(["prepare", "0.39.0"])
        self.assertIn("--no-upload", release.docs_command(args))
        self.assertNotIn("--skip-build", release.docs_command(args))
        args.docs_no_latest = True
        self.assertIn("--no-latest", release.docs_command(args))
        args.docs_distribution_id = "TEST"
        published = release.docs_command(args, prepared=True)
        self.assertIn("--no-latest", published)
        self.assertIn("--verify-live", published)
        self.assertIn("--skip-build", published)

    def test_publish_requires_explicit_commit_and_preflight_stops_before_mutations(self):
        with self.assertRaises(SystemExit):
            release.parse_args(["publish", "0.39.0"])
        args = release.parse_args(["publish", "0.39.0", "--approved-commit", "abc"])
        with patch.object(release, "validate_candidate", return_value={}), patch.object(
            release, "preflight", side_effect=release.ReleaseError("invalid AWS identity")
        ), patch.object(release, "run") as run:
            with self.assertRaisesRegex(release.ReleaseError, "AWS"):
                release.publish(args)
            run.assert_not_called()

    def test_resume_skips_only_matching_registry_checksums(self):
        with tempfile.TemporaryDirectory() as tmp:
            args = release.parse_args(["publish", "0.39.0", "--approved-commit", "abc", "--output", tmp])
            record = {"packages": [{"name": "pax-example", "sha256": "approved"}]}
            with patch.object(release, "validate_candidate", return_value=record), patch.object(release, "preflight"), patch.object(
                release, "registry_version", return_value={"cksum": "different"}
            ), patch.object(release, "run") as run:
                with self.assertRaisesRegex(release.ReleaseError, "differs"):
                    release.publish(args)
                run.assert_not_called()
            with patch.object(release, "validate_candidate", return_value=record), patch.object(release, "preflight"), patch.object(
                release, "registry_version", return_value={"cksum": "approved"}
            ), patch.object(release, "verify_archive_parity"), patch.object(release, "run") as run:
                release.publish(args)
                commands = [call.args[0] for call in run.call_args_list]
                self.assertEqual(len(commands), 1)
                self.assertIn("--verify-live", commands[0])
            journal = json.loads((Path(tmp) / "publication.json").read_text())
            self.assertEqual(journal["crates"]["pax-example"], "verified in registry")

    def test_archive_mismatch_stops_before_publication(self):
        with tempfile.TemporaryDirectory() as tmp:
            args = release.parse_args(["verify", "0.39.0", "--output", tmp])
            release.write_json(Path(tmp) / "candidate.json", {
                "schema": 2, "package_inputs": {"example": []}, "version": "0.39.0",
                "docs_no_latest": False, "source_sha256": "old", "package_smoke": {"checks": []}})
            with patch.object(release, "source_digest", return_value="new"), patch.object(release, "assert_cargo_version"):
                with self.assertRaisesRegex(release.ReleaseError, "Source"):
                    release.validate_candidate(args)

    def test_inventory_binds_ignored_files_contents_permissions_and_membership(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "Cargo.toml").write_text('[package]\nname="example"\nversion="0.39.0"\n')
            ignored = root / ".fingerprint"
            ignored.write_text("prepared")
            packages = {"example": {"manifest_path": str(root / "Cargo.toml")}}
            listing = "Cargo.toml\nCargo.toml.orig\nCargo.lock\n.cargo_vcs_info.json\n.fingerprint\n"
            with patch.object(release, "run", return_value=listing):
                prepared = release.packaging_inputs(root, packages)
                ignored.write_text("changed")
                self.assertNotEqual(prepared, release.packaging_inputs(root, packages))
                ignored.write_text("prepared")
                ignored.chmod(0o755)
                self.assertNotEqual(prepared, release.packaging_inputs(root, packages))
                ignored.unlink()
                with self.assertRaisesRegex(release.ReleaseError, "Cannot bind"):
                    release.packaging_inputs(root, packages)
            ignored.write_text("prepared")
            (root / "new-ignored-file").write_text("new")
            with patch.object(release, "run", return_value=listing + "new-ignored-file\n"):
                self.assertNotEqual(prepared, release.packaging_inputs(root, packages))
            with patch.object(release, "run", return_value=listing.replace('.fingerprint\n', '')):
                self.assertNotEqual(prepared, release.packaging_inputs(root, packages))

    def test_cargo_version_change_fails_before_packaging_or_upload(self):
        with patch.object(release, "run", return_value="cargo different\n") as run:
            with self.assertRaisesRegex(release.ReleaseError, "Cargo changed"):
                release.assert_cargo_version(ROOT, {"cargo": "cargo prepared"})
            self.assertEqual(run.call_args.args[0], ["cargo", "--version"])

    def test_candidate_retains_archives_outside_cargo_target(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            archive = root / "target/package/example-0.39.0.crate"
            archive.parent.mkdir(parents=True)
            with tarfile.open(archive, "w:gz") as tar:
                contents = b'[package]\nname="example"\nversion="0.39.0"\n'
                member = tarfile.TarInfo("example-0.39.0/Cargo.toml")
                member.size = len(contents)
                tar.addfile(member, io.BytesIO(contents))
            inventory = release.inspect_archives(root, {"example": package("example")}, "0.39.0", root / "candidate")
            retained = Path(inventory[0]["archive"])
            self.assertNotEqual(archive, retained)
            archive.write_bytes(b"overwritten by Cargo")
            self.assertEqual(release.sha256(retained), inventory[0]["sha256"])

    def test_archive_cleanup_preserves_other_versions_caches_and_candidates(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            target = root / "target"
            stale = target / "package/example-0.39.0.crate"
            preserved = [target / "package/example-0.38.3.crate",
                         target / "package/other-0.39.0.crate",
                         target / "package/example-0.39.0/src/lib.rs",
                         target / "debug/deps/compiled.rlib",
                         target / "release-candidate/0.39.0/archives/example-0.39.0.crate"]
            for path in [stale, *preserved]:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"preserved")
            release.clear_package_archives(target, ["example"], "0.39.0")
            self.assertFalse(stale.exists())
            for path in preserved:
                self.assertEqual(path.read_bytes(), b"preserved")
            release.clear_package_archives(target, ["example", "missing"], "0.39.0")

    @unittest.skipUnless(shutil.which("cargo"), "Cargo is required for the real archive regression")
    def test_real_cargo_repackaging_discards_stale_trailing_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("pub fn example() {}\n")
            (root / "Cargo.toml").write_text(
                '[package]\nname="archive-fixture"\nversion="0.39.0"\nedition="2021"\n'
                'description="Local archive regression"\nlicense="MIT"\n[workspace]\n')
            env = {**os.environ, "CARGO_HOME": str(root / "cargo-home"), "CARGO_NET_OFFLINE": "true"}
            def cargo(*args):
                result = subprocess.run(["cargo", *map(str, args)], cwd=root, env=env,
                                        text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                self.assertEqual(result.returncode, 0, result.stdout)
            cargo("generate-lockfile", "--offline")
            target = root / "target"
            command = ["package", "--offline", "--locked", "--no-verify", "--allow-dirty",
                       "--registry", "crates-io", "--target-dir", target]
            cargo(*command)
            archive = target / "package/archive-fixture-0.39.0.crate"
            expected = archive.read_bytes()
            archive.write_bytes(expected + b"stale bytes from a longer previous archive")
            release.clear_package_archives(target, ["archive-fixture"], "0.39.0")
            cargo(*command)
            self.assertEqual(archive.read_bytes(), expected)

    @unittest.skipUnless(shutil.which("cargo") and shutil.which("git"), "Cargo and Git are required")
    def test_publication_accepts_reviewed_ignored_outputs_but_rejects_source_and_input_drift(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            target = root / "target"
            args = release.parse_args(["publish", "0.39.0", "--approved-commit", "pending"])
            args.workspace, args.output = root, target / "candidate"
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("pub fn fixture() {}\n")
            (root / "Cargo.toml").write_text(
                '[package]\nname="release-fixture"\nversion="0.39.0"\nedition="2021"\n'
                'description="Local release regression"\nlicense="MIT"\n'
                'include=["Cargo.toml", "src/**", "generated.js"]\n[workspace]\n')
            (root / ".gitignore").write_text("/target/\n/generated.js\n")
            generated = root / "generated.js"
            generated.write_text("// reviewed generated output\n")
            env = {**{key: value for key, value in os.environ.items() if not key.startswith("GIT_")},
                   "CARGO_HOME": str(target / "cargo-home"), "CARGO_NET_OFFLINE": "true",
                   "GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1"}
            with patch.dict(os.environ, env, clear=True):
                def command(*argv):
                    result = subprocess.run(argv, cwd=root, text=True, stdout=subprocess.PIPE,
                                            stderr=subprocess.STDOUT)
                    self.assertEqual(result.returncode, 0, result.stdout)
                    return result.stdout.strip()
                command("cargo", "generate-lockfile", "--offline")
                command("git", "init", "--quiet")
                command("git", "add", "Cargo.toml", "Cargo.lock", ".gitignore", "src/lib.rs")
                command("git", "-c", "user.name=Release Test", "-c", "user.email=release@example.invalid",
                        "-c", "commit.gpgsign=false", "-c", f"core.hooksPath={root / 'no-hooks'}",
                        "commit", "--quiet", "-m", "Temporary release fixture")
                args.approved_commit = command("git", "rev-parse", "HEAD")
                self.assertEqual(command("git", "status", "--porcelain"), "")
                package_command = ["cargo", "package", "--offline", "--locked", "--no-verify",
                                   "--registry", "crates-io", "--target-dir", str(target)]
                rejected = subprocess.run(package_command, cwd=root, text=True,
                                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                self.assertNotEqual(rejected.returncode, 0)
                self.assertIn("generated.js", rejected.stdout)
                command(*package_command, "--allow-dirty")
                packages = release.release_packages(release.metadata(root))
                record = {"schema": 2, "version": args.version, "source_commit": args.approved_commit,
                          "source_dirty": False, "source_sha256": release.source_digest(root),
                          "docs_sha256": "fixture-docs", "docs_no_latest": False,
                          "cargo": command("cargo", "--version"),
                          "package_smoke": {"checks": ["fixture only"]},
                          "packages": release.inspect_archives(root, packages, args.version, args.output),
                          "package_inputs": release.packaging_inputs(root, packages)}
                release.write_json(args.output / "candidate.json", record)
                docs = argparse.Namespace(validate_build_record=lambda _: None, output_digest=lambda: "fixture-docs")
                real_run, uploads = release.run, []
                def no_upload(command, **kwargs):
                    if command[:2] == ["cargo", "publish"]:
                        self.assertIn("--allow-dirty", command)
                        self.assertIn("--locked", command)
                        self.assertNotIn("--no-verify", command)
                        uploads.append(command)
                        return ""
                    if len(command) > 1 and Path(command[1]).name == "publish_versioned_docs.py":
                        return ""
                    return real_run(command, **kwargs)
                with patch.object(release, "docs_module", return_value=docs), patch.object(
                    release, "run", no_upload
                ), patch.object(release, "preflight"), patch.object(release.time, "sleep"), patch.object(
                    release, "registry_version", side_effect=[None, None, {"cksum": record["packages"][0]["sha256"]}]
                ):
                    # Real validation and both real Cargo parity passes use phase=publish.
                    # Only external publication/preflight/registry operations are intercepted.
                    release.publish(args)
                    self.assertEqual(len(uploads), 1)
                    self.assertEqual(command("git", "status", "--porcelain"), "")
                    record["source_dirty"] = True
                    release.write_json(args.output / "candidate.json", record)
                    with self.assertRaisesRegex(release.ReleaseError, "exact approved clean commit"):
                        release.validate_candidate(args, publishing=True)
                    record["source_dirty"] = False
                    release.write_json(args.output / "candidate.json", record)
                    args.approved_commit = "wrong-commit"
                    with self.assertRaisesRegex(release.ReleaseError, "exact approved clean commit"):
                        release.validate_candidate(args, publishing=True)
                    args.approved_commit = record["source_commit"]
                    for relative in ("src/lib.rs", "untracked-source.txt"):
                        path = root / relative
                        original = path.read_bytes() if path.exists() else None
                        path.write_text("unreviewed source\n")
                        with self.assertRaisesRegex(release.ReleaseError, "Source or bundled artifacts changed"):
                            release.validate_candidate(args, publishing=True)
                        if original is None:
                            path.unlink()
                        else:
                            path.write_bytes(original)
                    generated.write_text("// unreviewed generated output\n")
                    with self.assertRaisesRegex(release.ReleaseError, "Packaging inputs changed"):
                        release.validate_candidate(args, publishing=True)
                    self.assertEqual(len(uploads), 1)

    def test_repackaging_mismatch_and_input_drift_abort_without_upload(self):
        with tempfile.TemporaryDirectory() as tmp:
            args = release.parse_args(["verify", "0.39.0"])
            args.workspace = Path(tmp)
            crate = package("example")
            meta = {"workspace_members": ["example"], "packages": [crate]}
            record = {"cargo": "cargo prepared", "source_sha256": "source", "package_inputs": {"example": []},
                      "packages": [{"name": "example", "sha256": hashlib.sha256(b"approved").hexdigest()}]}
            assembled = b"different"
            commands = []
            def run(command, **kwargs):
                commands.append(command)
                if command == ["cargo", "--version"]:
                    return "cargo prepared"
                self.assertEqual(command[:2], ["cargo", "package"])
                self.assertIn("--locked", command)
                self.assertIn("--no-verify", command)
                out = Path(command[command.index("--target-dir") + 1]) / "package"
                out.mkdir()
                (out / "example-0.39.0.crate").write_bytes(assembled)
            with patch.object(release, "run", run), patch.object(release, "metadata", return_value=meta), patch.object(
                release, "source_digest", return_value="source"
            ), patch.object(release, "packaging_inputs", return_value={"example": []}) as inputs:
                with self.assertRaisesRegex(release.ReleaseError, "Repackaged archive differs"):
                    release.verify_archive_parity(args, record)
                assembled = b"approved"
                release.verify_archive_parity(args, record)
                inputs.side_effect = [{"example": []}, {"example": ["changed during packaging"]}]
                with self.assertRaisesRegex(release.ReleaseError, "Packaging inputs changed"):
                    release.verify_archive_parity(args, record)
            self.assertFalse(any(command[:2] == ["cargo", "publish"] for command in commands))

    def test_publish_checks_full_set_and_each_crate_before_upload(self):
        with tempfile.TemporaryDirectory() as tmp:
            args = release.parse_args(["publish", "0.39.0", "--approved-commit", "abc", "--output", tmp])
            args.workspace = Path(tmp)
            record = {"packages": [{"name": "example", "sha256": "approved"}]}
            stale = args.workspace / "target/package/example-0.39.0.crate"
            stale.parent.mkdir(parents=True)
            stale.write_bytes(b"old archive with trailing bytes")
            events = []
            def parity(*args, **kwargs):
                events.append(("parity", kwargs.get("package")))
            def run(command, **kwargs):
                events.append(("command", command[:2]))
                if command[:2] == ["cargo", "publish"]:
                    self.assertIn("--allow-dirty", command)
                    self.assertFalse(stale.exists())
                    self.assertEqual(Path(command[command.index("--target-dir") + 1]), args.workspace / "target")
            with patch.object(release, "validate_candidate", return_value=record), patch.object(release, "preflight"), patch.object(
                release, "registry_version", side_effect=[None, None, {"cksum": "approved"}]
            ), patch.object(release, "verify_archive_parity", parity), patch.object(release, "run", run), patch.object(release.time, "sleep"):
                release.publish(args)
            self.assertEqual(events[:3], [("parity", None), ("parity", "example"), ("command", ["cargo", "publish"])])
            for failed_package in (None, "example"):
                stale.write_bytes(b"leave untouched on pre-upload failure")
                def fail_parity(*args, **kwargs):
                    if kwargs.get("package") == failed_package:
                        raise release.ReleaseError("parity mismatch")
                with patch.object(release, "validate_candidate", return_value=record), patch.object(release, "preflight"), patch.object(
                    release, "registry_version", return_value=None
                ), patch.object(release, "verify_archive_parity", fail_parity), patch.object(release, "run") as run:
                    with self.assertRaisesRegex(release.ReleaseError, "parity mismatch"):
                        release.publish(args)
                    run.assert_not_called()
                self.assertEqual(stale.read_bytes(), b"leave untouched on pre-upload failure")


if __name__ == "__main__":
    unittest.main()
