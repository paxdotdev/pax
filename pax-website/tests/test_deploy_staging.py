import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import deploy_staging as deploy
from prepare_production import assemble


class StagingRelease(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.base = Path(temp.name)
        self.web = self.base / "web"
        self.html = ('<head></head><script src="pax-interface-web.js"></script>'
                     '<script>const url = new URL("pax-cartridge", document.baseURI);</script>')
        for key in deploy.RUNTIME:
            self.write(key, key)
        self.write("pax-cartridge.js", "import {f} from './snippets/nested/init.js';")
        self.write("snippets/nested/init.js", "export const f = 1")
        self.write("route-metadata.json", json.dumps({"routes": [
            {"concrete_path": path} for path in ("/", "/future/demo", None)]}))
        self.write("index.html", self.html)
        self.write("future/demo/index.html", self.html)
        self.write("ai/index.html", "<html><head></head><body>Static AI guide</body></html>")
        self.write("blog/index.html", "Unpublished local draft")

    def write(self, key, text):
        path = self.web / key
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def prepare(self):
        work = Path(tempfile.mkdtemp(dir=self.base))
        return work, *deploy.prepare(self.web, work)

    def test_cache_key_and_all_route_entries(self):
        work, prefix, entries, public = self.prepare()
        self.assertEqual(set(entries), {"index.html", "future/demo/index.html"})
        for name in entries:
            html = (work / "entries" / name).read_text()
            for key in ("pax-interface-web.css", "pax-interface-web.js", "pax-cartridge"):
                self.assertIn(f'"{prefix}{key}"', html)
            self.assertEqual((self.web / name).read_text(), self.html)
        for key in ("entries/index.html", "entries/future/demo/index.html", "public/ai/index.html"):
            html = (work / key).read_text()
            self.assertEqual(html.count('src="/site-analytics/consent.js"'), 1)
            self.assertNotIn("googletagmanager.com", html)
        for name in ("consent.js", "consent.css"):
            self.assertTrue((work / "public/site-analytics" / name).is_file())
        self.assertTrue((work / "runtime/snippets/nested/init.js").is_file())
        self.assertFalse((work / "public/pax-interface-web.css").exists())
        self.assertEqual(set(public), {"ai/index.html", "route-metadata.json",
                                       "site-analytics/consent.js", "site-analytics/consent.css"})
        self.write("index.html", self.html + "<p>New copy</p>")
        self.assertEqual(prefix, self.prepare()[1])
        for key in ("pax-interface-web.css", "split_application.wasm", "snippets/nested/init.js"):
            self.write(key, "new bytes")
            updated = self.prepare()[1]
            self.assertNotEqual(prefix, updated)
            prefix = updated

    def test_incomplete_artifacts_and_unknown_bootstrap_fail_closed(self):
        self.write("future/demo/index.html", self.html.replace("</head>", '<link href="pax-interface-web.css"></head>'))
        with self.assertRaisesRegex(ValueError, "already references the runtime stylesheet"):
            self.prepare()
        self.write("future/demo/index.html", self.html.replace("pax-interface-web.js", "other.js"))
        with self.assertRaisesRegex(ValueError, "future/demo/index.html"):
            self.prepare()
        self.write("future/demo/index.html", self.html)
        (self.web / "snippets/nested/init.js").unlink()
        with self.assertRaisesRegex(ValueError, "missing runtime import"):
            self.prepare()

    def test_production_adds_only_the_first_party_gate_once_per_document(self):
        self.write("ai/index.html", "<html><head></head><body>AI guide</body></html>")
        output = self.base / "production"
        assemble(self.web, output)
        for name in ("index.html", "future/demo/index.html", "ai/index.html"):
            html = (output / name).read_text()
            self.assertEqual(html.count('src="/site-analytics/consent.js"'), 1)
            self.assertNotIn("googletagmanager.com", html)
            self.assertNotIn("consent.js", (self.web / name).read_text())
        self.assertTrue((output / "site-analytics/consent.js").is_file())
        self.assertTrue((output / "site-analytics/consent.css").is_file())
        self.assertFalse((output / "blog").exists())
        with self.assertRaisesRegex(ValueError, "new output"):
            assemble(self.web, output)

    def publisher(self, fail_on=None, origin_path=""):
        calls = []

        def run(args, **kwargs):
            calls.append(args)
            if fail_on and fail_on in args:
                raise subprocess.CalledProcessError(1, args)
            response = {"DistributionConfig": {"Aliases": {"Items": [deploy.BUCKET]}, "Origins": {"Items": [{
                "DomainName": f"{deploy.BUCKET}.s3-website-us-east-1.amazonaws.com", "OriginPath": origin_path}]}}}
            return subprocess.CompletedProcess(args, 0, stdout=json.dumps(response) if "get-distribution-config" in args else "id")
        return calls, patch.object(deploy.subprocess, "run", side_effect=run)

    def test_promote_last_keep_old_releases_and_invalidate_directory_aliases(self):
        prepared = self.prepare()
        calls, transport = self.publisher()
        with transport:
            deploy.publish(*prepared)
        uploads = [args[args.index("cp") + 2:] for args in calls if "cp" in args]
        runtime = [u for u in uploads if deploy.IMMUTABLE in u]
        self.assertTrue(all(prepared[1] in u[0] for u in runtime))
        self.assertEqual(uploads[:len(runtime)], runtime)
        self.assertIn("application/wasm", next(u for u in runtime if u[0].endswith("split_application.wasm")))
        self.assertEqual(uploads[-1][0], f"s3://{deploy.BUCKET}/index.html")
        self.assertIn(deploy.MUTABLE, uploads[-1])
        self.assertFalse(any("--delete" in args for args in calls))
        invalidation = next(args for args in calls if "create-invalidation" in args)
        self.assertEqual(set(invalidation[invalidation.index("--paths") + 1:invalidation.index("--query")]),
                         {"/", "/index.html", "/future/demo", "/future/demo/", "/future/demo/index.html",
                          "/ai", "/ai/", "/ai/index.html", "/route-metadata.json",
                          "/site-analytics/consent.js", "/site-analytics/consent.css"})
        self.assertIn("invalidation-completed", calls[-1])

    def test_no_promotion_after_origin_mismatch_or_upload_failure(self):
        prepared = self.prepare()
        for options, error in (({"origin_path": "/www"}, ValueError),
                               ({"fail_on": "application/wasm"}, subprocess.CalledProcessError)):
            calls, transport = self.publisher(**options)
            with transport, self.assertRaises(error):
                deploy.publish(*prepared)
            self.assertFalse(any(deploy.MUTABLE in args or "create-invalidation" in args for args in calls))


if __name__ == "__main__":
    unittest.main()
