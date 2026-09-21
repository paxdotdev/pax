"""Release boundary tests: no network access, publication, or git mutation."""
import argparse
import hashlib
import importlib.util
import io
import json
from pathlib import Path
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
            record = {"packages": [{"name": "example", "sha256": "approved"}]}
            events = []
            def parity(*args, **kwargs):
                events.append(("parity", kwargs.get("package")))
            def run(command, **kwargs):
                events.append(("command", command[:2]))
            with patch.object(release, "validate_candidate", return_value=record), patch.object(release, "preflight"), patch.object(
                release, "registry_version", side_effect=[None, None, {"cksum": "approved"}]
            ), patch.object(release, "verify_archive_parity", parity), patch.object(release, "run", run), patch.object(release.time, "sleep"):
                release.publish(args)
            self.assertEqual(events[:3], [("parity", None), ("parity", "example"), ("command", ["cargo", "publish"])])
            for failed_package in (None, "example"):
                def fail_parity(*args, **kwargs):
                    if kwargs.get("package") == failed_package:
                        raise release.ReleaseError("parity mismatch")
                with patch.object(release, "validate_candidate", return_value=record), patch.object(release, "preflight"), patch.object(
                    release, "registry_version", return_value=None
                ), patch.object(release, "verify_archive_parity", fail_parity), patch.object(release, "run") as run:
                    with self.assertRaisesRegex(release.ReleaseError, "parity mismatch"):
                        release.publish(args)
                    run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
