"""Packaging boundaries; real Flatpak/GPU admission is tested on the Linux guest."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("packager", Path(__file__).resolve().parents[1] / "package-linux-flatpak.py")
packager = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(packager)


class FlatpakPackageTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.source = self.root / "input package"
        self.source.mkdir()
        header = b"\x7fELF\x02\x01" + bytes(12) + b"\x3e\x00"
        for name in ("example-app", "libapp.so", "libpax_host.so"):
            (self.source / name).write_bytes(header)
            (self.source / name).chmod(0o755)
        (self.source / "assets/0").mkdir(parents=True)
        (self.source / "assets/0/font.ttf").write_bytes(b"test asset")
        (self.source / "pax-app.json").write_text(json.dumps({"cartridge": "libapp.so", "assets": ["assets/0"]}))
        self.args = argparse.Namespace(package=str(self.source), output=str(self.root / "output package"),
                                       launcher="example-app", app_id="org.example.App", runtime_commit="a" * 64,
                                       timestamp=1792141200)

    def capture(self, *args):
        if "--show-commit" in args:
            return self.args.runtime_commit
        if "--show-origin" in args:
            return "flathub"
        return "Flatpak fixture"

    def flatpak_run(self, *args):
        if args[0] == "build-export":
            ref = Path(args[2]) / "refs/heads/app/org.example.App/x86_64/pilot"
            ref.parent.mkdir(parents=True)
            ref.write_text("c" * 64 + "\n")
        elif args[0] == "build-bundle":
            Path(args[3]).write_bytes(b"fixture bundle")

    def package(self):
        with patch.object(packager, "capture", side_effect=self.capture), patch.object(packager, "run", side_effect=self.flatpak_run) as run:
            result = packager.package_flatpak(self.args)
            return result, [call.args for call in run.call_args_list]

    def test_package_keeps_assets_and_source_and_has_only_narrow_permissions(self):
        before = packager.inventory(self.source)
        result, commands = self.package()
        output = Path(self.args.output)
        payload = output / "build/files/lib/pax"
        self.assertEqual(packager.inventory(payload), before)
        self.assertEqual(packager.inventory(self.source), before)
        metadata = (output / "build/metadata").read_text()
        self.assertIn("sockets=wayland;fallback-x11;", metadata)
        self.assertIn("devices=dri;", metadata)
        for forbidden in ("filesystem", "network", "session-bus", "system-bus"):
            self.assertNotIn(forbidden, metadata)
        self.assertIn('exec ./example-app "$@"', (output / "build/files/bin/pax-launch").read_text())
        self.assertEqual([command[0] for command in commands], ["build-finish", "build-export", "build-bundle"])
        self.assertIn("--no-inherit-permissions", commands[0])
        self.assertTrue(any(arg.startswith("--timestamp=") for arg in commands[1]))
        self.assertEqual(result["packagingRuntimeCommit"], "a" * 64)
        self.assertEqual(result["appCommit"], "c" * 64)
        provenance = json.loads((output / "package-provenance.json").read_text())
        self.assertEqual(provenance["inputs"], before)
        self.assertNotIn(str(self.root), json.dumps(provenance))

    def test_existing_or_overlapping_output_is_never_changed(self):
        existing = self.root / "existing"
        existing.mkdir()
        (existing / "keep").write_text("preserve")
        for output in (existing, self.source / "nested", self.source):
            with self.subTest(output=output), patch.object(packager, "run") as run:
                self.args.output = str(output)
                with self.assertRaises(packager.PackageError):
                    packager.package_flatpak(self.args)
                run.assert_not_called()
        self.assertEqual((existing / "keep").read_text(), "preserve")

    def test_runtime_mismatch_stops_before_writes_or_flatpak_mutations(self):
        with patch.object(packager, "capture", return_value="b" * 64), patch.object(packager, "run") as run:
            with self.assertRaisesRegex(packager.PackageError, "runtime differs"):
                packager.package_flatpak(self.args)
            run.assert_not_called()
        self.assertFalse(Path(self.args.output).exists())

    def test_missing_assets_wrong_architecture_and_escape_are_rejected(self):
        recipe = self.source / "pax-app.json"
        for asset in ("missing", "../outside", "/etc", "assets/../assets/0"):
            with self.subTest(asset=asset):
                recipe.write_text(json.dumps({"cartridge": "libapp.so", "assets": [asset]}))
                with self.assertRaises((packager.PackageError, FileNotFoundError)):
                    packager.validate_package(self.source, self.args.launcher)
        recipe.write_text(json.dumps({"cartridge": "libapp.so", "assets": []}))
        (self.source / "libapp.so").write_bytes(b"wrong architecture")
        with self.assertRaisesRegex(packager.PackageError, "x86_64 Linux ELF"):
            packager.validate_package(self.source, self.args.launcher)

    def test_relative_internal_symlinks_survive_but_external_links_are_rejected(self):
        link = self.source / "assets/0/alias.ttf"
        link.symlink_to("font.ttf")
        self.package()
        copied = Path(self.args.output) / "build/files/lib/pax/assets/0/alias.ttf"
        self.assertTrue(copied.is_symlink())
        self.assertEqual(copied.read_bytes(), b"test asset")
        link.unlink()
        link.symlink_to(self.root)
        with self.assertRaisesRegex(packager.PackageError, "Symlink escapes"):
            packager.inventory(self.source)

    def test_filename_and_metadata_injection_are_rejected_before_flatpak(self):
        for field, value in (("launcher", "app; touch bad"), ("app_id", "org.example.App\n[Context]"),
                             ("runtime_commit", "main"), ("timestamp", -1)):
            with self.subTest(field=field), patch.object(packager, "capture") as capture:
                original = getattr(self.args, field)
                setattr(self.args, field, value)
                with self.assertRaises(packager.PackageError):
                    packager.package_flatpak(self.args)
                capture.assert_not_called()
                setattr(self.args, field, original)


if __name__ == "__main__":
    unittest.main()
