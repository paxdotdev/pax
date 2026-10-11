"""Keep source discovery, scaffolding, and cleanup aligned with both example levels."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("sync_docs", ROOT / "scripts/sync-docs-examples.py")
sync_docs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync_docs)


class ExampleLayoutTests(unittest.TestCase):
    def test_repository_helpers_resolve_the_monorepo_cli(self):
        for parent in [ROOT / "examples", ROOT / "examples/fixtures"]:
            for helper in parent.glob("*/pax"):
                script = helper.read_text()
                for line in script.splitlines():
                    if line.startswith("pushd "):
                        self.assertEqual((helper.parent / line.split()[1]).resolve(), ROOT / "pax-cli")
                    if line.startswith('monorepo_cli="'):
                        relative = line.split('$project_dir/', 1)[1].rstrip('"')
                        self.assertEqual((helper.parent / relative).resolve(), ROOT / "pax-cli/Cargo.toml")

    def test_snapshot_discovers_both_levels_and_rejects_duplicate_names(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("first", "fixtures/probe"):
                app = root / "examples" / name
                (app / "src").mkdir(parents=True)
                (app / "Cargo.toml").write_text("[package]\n")
                (app / "src/lib.rs").write_text("// example\n")
                (app / ".pax").mkdir()
                (app / ".pax/generated.rs").write_text("// exclude\n")
            files = json.loads(sync_docs.snapshot(root))["files"]
            self.assertEqual(len(files), 4)
            self.assertIn("examples/fixtures/probe/src/lib.rs", files)
            (root / "examples/probe").mkdir()
            (root / "examples/probe/Cargo.toml").write_text("")
            with self.assertRaisesRegex(ValueError, "Duplicate example name"):
                sync_docs.snapshot(root)

    def test_scaffold_registers_fixture_and_explicit_showcase(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "examples/fixtures").mkdir(parents=True)
            (root / "Cargo.toml").write_text('[workspace]\nmembers = [\n    "examples/fixtures/*",\n]\n[profile.release]\nopt-level = "z"\n[dev-dependencies]\n')
            helper = root / "examples/create-example.sh"
            shutil.copy2(ROOT / "examples/create-example.sh", helper)
            (root / "bin").mkdir()
            cargo = root / "bin/cargo"
            cargo.write_text('''#!/usr/bin/env python3
from pathlib import Path
import sys
root = Path(sys.argv[sys.argv.index("create") + 1])
root.mkdir()
(root / "Cargo.toml").write_text("[package]\\nname = 'demo'\\n[profile.dev]\\nopt-level = 1\\n")
''')
            cargo.chmod(0o755)
            env = dict(os.environ, PATH=str(root / "bin") + os.pathsep + os.environ["PATH"])
            for args in (["probe"], ["--showcase", "showcase"]):
                subprocess.run(["bash", str(helper), *args], env=env, check=True, capture_output=True)
            manifest = (root / "Cargo.toml").read_text()
            for name in ("fixtures/probe", "showcase"):
                self.assertIn(f'path = "examples/{name}"', manifest)
                self.assertIn(f'path = "examples/{name}.rs"', manifest)
                self.assertTrue((root / "examples" / (name + ".rs")).is_file())
                self.assertNotIn("[profile.dev]", (root / "examples" / name / "Cargo.toml").read_text())
            self.assertIn('    "examples/showcase",', manifest)
            self.assertNotEqual(subprocess.run(["bash", str(helper), "--showcase", "probe"], env=env, capture_output=True).returncode, 0)

    def test_cleanup_reaches_fixtures_and_preserves_tracked_or_unignored_data(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            (root / "Cargo.toml").touch()
            (root / "pax-cli").mkdir()
            (root / ".gitignore").write_bytes((ROOT / ".gitignore").read_bytes())
            removed = ["target", "examples/first/.pax", "examples/fixtures/probe/.pax", "examples/fixtures/probe/target"]
            retained = ["examples/fixtures/tracked/.pax", "custom/.pax"]
            for name in removed + retained:
                (root / name).mkdir(parents=True)
                (root / name / "sentinel").write_text("preserve until cleanup\n")
            subprocess.run(["git", "-C", str(root), "add", "-f", "examples/fixtures/tracked/.pax/sentinel"], check=True)
            command = ["bash", str(ROOT / "scripts/clean-temp-files.sh")]
            dry = subprocess.run(command + ["--dry-run"], cwd=root, text=True, check=True, capture_output=True)
            self.assertIn("examples/fixtures/probe/.pax", dry.stdout)
            self.assertTrue(all((root / name).exists() for name in removed + retained))
            subprocess.run(command, cwd=root, check=True, capture_output=True)
            self.assertTrue(all(not (root / name).exists() for name in removed))
            self.assertTrue(all((root / name / "sentinel").is_file() for name in retained))


if __name__ == "__main__":
    unittest.main()
