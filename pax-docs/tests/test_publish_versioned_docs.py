"""Publication regressions. All external commands are intercepted; no AWS access."""

import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "publisher", ROOT / "pax-docs/scripts/publish_versioned_docs.py"
)
publisher = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(publisher)


class PublicationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.workspace = Path(temporary.name)
        self.book = self.workspace / "pax-docs/book"
        self.output = self.book / "book"
        self.source_manifest = self.book / "src/versions.json"
        self.source_manifest.parent.mkdir(parents=True)
        self.source_manifest.write_text(json.dumps({
            "schema": 1, "latest": "0.38.3", "versions": [
                {"version": "0.38.3", "path": "/0.38.3/", "released": "2026-01-01"}
            ],
        }))
        for name, value in {
            "BOOK_DIR": self.book, "BOOK_OUTPUT": self.output,
            "VERSIONS_MANIFEST": self.source_manifest,
        }.items():
            self.enterContext(patch.object(publisher, name, value))
        self.commands = []
        self.objects = {"0.38.3/old.html": (b"historical", "immutable")}
        self.fail_destination = None
        self.enterContext(patch.object(publisher, "run", self.run_command))

    def create_output(self, text="new docs"):
        for name in ("index.html", "getting-started.html", "theme/pax-version.js"):
            path = self.output / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        (self.output / "versions.json").write_bytes(self.source_manifest.read_bytes())

    def run_command(self, command, cwd, *, capture=False):
        """Emulate object copies/deletes, capturing bytes at upload time."""
        self.commands.append(command)
        if command[0] != "aws":
            return
        if command[1] == "cloudfront":
            return json.dumps({"Invalidation": {"Id": "TEST-INVALIDATION"}})
        destination = command[4]
        if destination == self.fail_destination:
            raise subprocess.CalledProcessError(1, command)
        prefix = destination.removeprefix("s3://test-bucket/")
        cache = command[command.index("--cache-control") + 1]
        source = Path(command[3])
        if source.is_file():
            self.objects[prefix] = (source.read_bytes(), cache)
            return
        names = {path.relative_to(source).as_posix(): path
                 for path in source.rglob("*") if path.is_file()}
        if "--exclude" in command:
            names.pop(command[command.index("--exclude") + 1], None)
        if "--delete" in command:
            for key in list(self.objects):
                if key.startswith(prefix) and key[len(prefix):] not in names:
                    del self.objects[key]
        for name, path in names.items():
            self.objects[prefix + name] = (path.read_bytes(), cache)

    def main(self, *flags, version="0.39.0"):
        argv = ["publish", version, "--workspace", str(self.workspace),
                "--bucket", "test-bucket", "--distribution-id", "TEST", *flags]
        with patch.object(sys, "argv", argv):
            return publisher.main()

    def catalog(self):
        return json.loads(self.source_manifest.read_text())

    def build(self, _workspace, _skip_examples):
        self.create_output()

    def test_semver_order_and_build_metadata(self):
        ordered = ["1.0.0-alpha", "1.0.0-alpha.1", "1.0.0-alpha.beta",
                   "1.0.0-beta", "1.0.0-beta.2", "1.0.0-beta.11",
                   "1.0.0-rc.1", "1.0.0", "1.0.1", "1.10.0"]
        self.assertEqual(sorted(reversed(ordered), key=publisher.version_key), ordered)
        self.assertEqual(publisher.version_key("1.0.0+build.1"),
                         publisher.version_key("1.0.0+build.999"))
        self.assertEqual(publisher.release_version("1.0.0-rc.1+build.01"),
                         "1.0.0-rc.1+build.01")

    def test_invalid_versions_rejected_before_any_write_or_command(self):
        original = self.source_manifest.read_bytes()
        for version in ("../latest", "latest", "v1.0.0", "01.0.0", "1.0",
                        "1.0.0-01", "1.0.0-rc..1", "1.0.0+", "1.0.0/x"):
            with self.subTest(version=version), self.assertRaises(SystemExit):
                self.main("--skip-build", version=version)
        self.assertEqual(self.source_manifest.read_bytes(), original)
        self.assertEqual(self.commands, [])

    def test_no_latest_preserves_pointer_and_first_release_date(self):
        self.main("--skip-build", "--no-upload", "--no-latest")
        self.assertEqual(self.catalog()["latest"], "0.38.3")
        self.assertEqual(self.catalog()["versions"][0]["version"], "0.39.0")
        self.main("--skip-build", "--no-upload", "--no-latest", version="0.38.3")
        self.assertEqual(self.catalog()["versions"][1]["released"], "2026-01-01")
        self.assertEqual(self.commands, [])
        self.assertFalse(self.output.exists())

    def test_first_version_without_promotion_retains_null_latest(self):
        self.source_manifest.unlink()
        self.main("--skip-build", "--no-upload", "--no-latest")
        self.assertIsNone(self.catalog()["latest"])

    def test_full_no_upload_build_validates_and_stamps_without_aws(self):
        with patch.object(publisher, "build_docs", self.build):
            self.assertEqual(self.main("--no-upload"), 0)
        publisher.validate_build_record("0.39.0")
        self.assertEqual(self.commands, [])
        self.assertEqual(self.catalog()["latest"], "0.39.0")

    def test_build_uses_explicit_output_and_forwards_skip_examples(self):
        publisher.build_docs(self.workspace, True)
        self.assertEqual(self.commands[0][-1], "--skip-examples")
        self.assertEqual(self.commands[1][-2:], ["--dest-dir", str(self.output)])
        self.assertEqual(self.commands[2], ["node", str(self.workspace / "pax-docs/scripts/check_publication_output.mjs"), str(self.output)])

    def test_skip_build_refreshes_catalog_without_changing_content(self):
        self.create_output()
        publisher.write_build_record("0.39.0")
        self.main("--skip-build", "--no-latest")
        uploaded = json.loads(self.objects["versions.json"][0])
        self.assertEqual(uploaded, self.catalog())
        self.assertEqual(uploaded["latest"], "0.38.3")
        self.assertEqual(uploaded["versions"][0]["version"], "0.39.0")
        self.assertEqual(self.objects["0.39.0/index.html"][0], b"new docs")
        self.assertNotIn("index.html", self.objects)
        invalidation = next(c for c in self.commands if c[1:3] == ["cloudfront", "create-invalidation"])
        self.assertEqual(invalidation[invalidation.index("--paths") + 1:-2],
                         ["/0.39.0/*", "/versions.json"])
        self.assertEqual(self.commands[-1][1:4], ["cloudfront", "wait", "invalidation-completed"])

    def test_promotion_and_ghost_patch_refresh_headers_preserve_other_versions(self):
        self.objects["0.39.0/index.html"] = (b"new docs", "immutable")
        self.objects["0.39.0/removed.html"] = (b"stale", "immutable")
        with patch.object(publisher, "build_docs", self.build):
            self.main()
            self.main()  # Same-semver publication is intentionally allowed.
        self.assertEqual(self.objects["0.39.0/index.html"], (b"new docs", "no-cache"))
        self.assertEqual(self.objects["index.html"], (b"new docs", "no-cache"))
        self.assertEqual(self.objects["0.38.3/old.html"], (b"historical", "immutable"))
        self.assertNotIn("0.39.0/removed.html", self.objects)
        self.assertEqual(json.loads(self.objects["versions.json"][0])["latest"], "0.39.0")
        invalidation = next(c for c in self.commands if c[1:3] == ["cloudfront", "create-invalidation"])
        self.assertEqual(invalidation[invalidation.index("--paths") + 1:-2], ["/*"])
        for command in self.commands:
            if command[1:3] == ["s3", "cp"] and "--recursive" in command:
                self.assertIn("--no-follow-symlinks", command)
            if command[1] == "s3" and command[4] == "s3://test-bucket/":
                self.assertNotIn("--delete", command)
                self.assertEqual(command[-2:], ["--exclude", "versions.json"])

    def test_failed_version_upload_does_not_promote_root_or_catalog(self):
        self.fail_destination = "s3://test-bucket/0.39.0/"
        with patch.object(publisher, "build_docs", self.build):
            with self.assertRaises(subprocess.CalledProcessError):
                self.main()
        self.assertNotIn("versions.json", self.objects)
        self.assertNotIn("index.html", self.objects)
        self.assertEqual(len(self.commands), 1)

    def test_failed_root_upload_does_not_promote_catalog(self):
        self.fail_destination = "s3://test-bucket/"
        with patch.object(publisher, "build_docs", self.build):
            with self.assertRaises(subprocess.CalledProcessError):
                self.main()
        self.assertNotIn("versions.json", self.objects)
        self.assertFalse(any(command[1] == "cloudfront" for command in self.commands))

    def test_skip_build_refuses_missing_or_wrong_version_record_before_manifest_write(self):
        self.create_output()
        original = self.source_manifest.read_bytes()
        with self.assertRaisesRegex(SystemExit, "build record"):
            self.main("--skip-build")
        publisher.write_build_record("0.38.3")
        with self.assertRaisesRegex(SystemExit, "not recorded for 0.39.0"):
            self.main("--skip-build")
        self.assertEqual(self.commands, [])
        self.assertEqual(self.source_manifest.read_bytes(), original)

    def test_skip_build_refuses_modified_output(self):
        self.create_output()
        publisher.write_build_record("0.39.0")
        (self.output / "theme/pax-version.js").write_text("different")
        with self.assertRaisesRegex(SystemExit, "changed since"):
            self.main("--skip-build")
        self.assertEqual(self.commands, [])

    def test_skip_build_refuses_deleted_asset(self):
        self.create_output()
        asset = self.output / "example.wasm"
        asset.write_bytes(b"wasm")
        publisher.write_build_record("0.39.0")
        asset.unlink()
        with self.assertRaisesRegex(SystemExit, "changed since"):
            self.main("--skip-build")
        self.assertEqual(self.commands, [])

    def test_incomplete_build_and_symlinks_never_upload(self):
        with patch.object(publisher, "build_docs", lambda *_: None):
            with self.assertRaisesRegex(SystemExit, "Incomplete docs output"):
                self.main()
        self.create_output()
        (self.output / "outside").symlink_to(self.source_manifest)
        with self.assertRaisesRegex(SystemExit, "symlinks"):
            publisher.write_build_record("0.39.0")
        self.assertEqual(self.commands, [])

    def test_failed_stale_or_incomplete_embeds_cannot_pass_publication_validation(self):
        self.create_output()
        (self.output / "index.html").write_text('<pax-example path="demo"></pax-example>')
        directory = self.output / "_pax_examples/demo"
        (directory / "app").mkdir(parents=True)
        (directory / "app/index.html").write_text("example")
        (directory / "app/demo.wasm").write_bytes(b"wasm")
        manifest = {"source_fingerprint": "new", "built_fingerprint": "new",
                    "app": {"available": True, "index": "app/index.html"},
                    "files": [{"path": "src/lib.pax", "contents": "<Rectangle/>"}]}
        file = directory / "manifest.json"
        with self.assertRaisesRegex(SystemExit, "example manifest"):
            publisher.validate_output()
        for change, message in [
            ({"app": {"available": False, "build_error": "compile failed"}}, "no successful"),
            ({"built_fingerprint": "old"}, "stale compiled source"),
            ({"files": []}, "Missing embedded example source"),
        ]:
            file.write_text(json.dumps({**manifest, **change}))
            with self.subTest(change=change), self.assertRaisesRegex(SystemExit, message):
                publisher.validate_output()
        file.write_text(json.dumps(manifest))
        publisher.validate_output()
        (directory / "app/demo.wasm").unlink()
        with self.assertRaisesRegex(SystemExit, "Missing compiled Wasm"):
            publisher.validate_output()
        self.assertEqual(self.commands, [])

    def test_invalid_embed_paths_cannot_escape_output_tree(self):
        self.create_output()
        for name in ("../escape", "/absolute", "demo/../escape", "demo\\escape", ""):
            (self.output / "index.html").write_text(f'<pax-example path="{name}"></pax-example>')
            with self.subTest(name=name), self.assertRaisesRegex(SystemExit, "Invalid embedded example path"):
                publisher.validate_output()

    def test_no_distribution_id_skips_invalidation(self):
        self.create_output()
        publisher.write_build_record("0.39.0")
        publisher.publish_docs(argparse.Namespace(version="0.39.0", workspace=self.workspace,
                                                 bucket="test-bucket", no_latest=True,
                                                 distribution_id=None))
        self.assertFalse(any(command[1] == "cloudfront" for command in self.commands))


    def test_invalidation_failure_keeps_receipt_pending_and_retry_completes(self):
        self.create_output()
        publisher.write_build_record("0.39.0")
        def fail_wait(command, cwd, **kwargs):
            if command[1:3] == ["cloudfront", "wait"]:
                raise subprocess.CalledProcessError(1, command)
            return self.run_command(command, cwd, **kwargs)
        with patch.object(publisher, "run", fail_wait):
            with self.assertRaises(subprocess.CalledProcessError):
                self.main("--skip-build")
        receipt = self.workspace / "target/docs-publication/0.39.0.json"
        self.assertEqual(json.loads(receipt.read_text())["status"], "invalidation pending")
        self.main("--skip-build")
        self.assertEqual(json.loads(receipt.read_text())["status"], "invalidation completed")

    def test_live_check_failure_is_not_reported_as_success(self):
        self.create_output()
        publisher.write_build_record("0.39.0")
        with patch.object(publisher, "preflight_publication"), patch.object(
            publisher, "verify_live_output", side_effect=SystemExit("stale live content")
        ):
            with self.assertRaisesRegex(SystemExit, "stale live content"):
                self.main("--skip-build", "--verify-live")
        receipt = json.loads((self.workspace / "target/docs-publication/0.39.0.json").read_text())
        self.assertEqual(receipt["status"], "invalidation completed")

    def test_failed_preflight_uploads_nothing(self):
        self.create_output()
        publisher.write_build_record("0.39.0")
        with patch.object(publisher, "preflight_publication", side_effect=SystemExit("bad credentials")):
            with self.assertRaisesRegex(SystemExit, "bad credentials"):
                self.main("--skip-build", "--verify-live")
        self.assertEqual(self.commands, [])


if __name__ == "__main__":
    unittest.main()
