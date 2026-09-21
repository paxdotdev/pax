"""Read-only production preflight and live verification, using local fixtures."""
import argparse
from email.message import Message
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
import urllib.error
import urllib.parse
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('publisher_preflight', ROOT / 'pax-docs/scripts/publish_versioned_docs.py')
publisher = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(publisher)


class PreflightTests(unittest.TestCase):
    def setUp(self):
        self.args = argparse.Namespace(workspace=ROOT, bucket='docs.pax.dev', distribution_id='CDN',
                                       site_url='https://docs.pax.dev', version='0.39.0', no_latest=False)
        self.config = {'Enabled': True, 'Aliases': {'Items': ['docs.pax.dev']},
                       'Origins': {'Items': [{'Id': 'docs', 'DomainName': 'docs.pax.dev.s3.amazonaws.com'}]},
                       'DefaultCacheBehavior': {'TargetOriginId': 'docs', 'CachePolicyId': 'CACHE'}}
        self.ttl = 0
        self.acl = {'Grants': []}
        self.commands = []
        self.enterContext(patch.object(publisher, 'run', self.run_command))

    def run_command(self, command, cwd, **kwargs):
        self.commands.append(command)
        if command[1:3] == ['cloudfront', 'get-distribution']:
            return json.dumps({'Distribution': {'Status': 'Deployed', 'DistributionConfig': self.config}})
        if command[1:3] == ['cloudfront', 'get-cache-policy']:
            return json.dumps({'CachePolicy': {'CachePolicyConfig': {'MinTTL': self.ttl}}})
        if command[1:3] == ['s3api', 'list-objects-v2']:
            return '{}'
        if command[1:3] == ['s3api', 'get-object-acl']:
            return json.dumps(self.acl)
        return ''

    def test_accepts_matching_origin_and_only_reads_aws(self):
        self.assertFalse(publisher.preflight_publication(self.args))
        self.assertEqual([c[1:3] for c in self.commands], [
            ['sts', 'get-caller-identity'], ['s3api', 'head-bucket'],
            ['cloudfront', 'get-distribution'], ['cloudfront', 'get-cache-policy'],
            ['s3api', 'list-objects-v2']])

    def test_website_preserves_public_read_without_changing_bucket_permissions(self):
        self.config['Origins']['Items'][0]['DomainName'] = 'docs.pax.dev.s3-website-us-west-2.amazonaws.com'
        self.acl = {'Grants': [{'Permission': 'READ', 'Grantee': {
            'URI': 'http://acs.amazonaws.com/groups/global/AllUsers'}}]}
        self.assertTrue(publisher.preflight_publication(self.args))
        self.assertIn(['aws', 's3api', 'get-object-acl', '--bucket', 'docs.pax.dev',
                       '--key', 'index.html', '--output', 'json'], self.commands)
        self.assertFalse(any('put-' in str(command) for command in self.commands))

    def test_private_or_policy_based_origins_do_not_implicitly_grant_public_acls(self):
        self.config['Origins']['Items'][0]['DomainName'] = 'docs.pax.dev.s3-website-us-west-2.amazonaws.com'
        self.assertFalse(publisher.preflight_publication(self.args))
        self.acl = {'Grants': [{'Permission': 'READ_ACP', 'Grantee': {
            'URI': 'http://acs.amazonaws.com/groups/global/AllUsers'}}]}
        self.assertFalse(publisher.preflight_publication(self.args))
        self.acl['Grants'][0]['Permission'] = 'READ'
        self.config['Origins']['Items'][0]['DomainName'] = 'docs.pax.dev.s3.amazonaws.com'
        self.commands.clear()
        self.assertFalse(publisher.preflight_publication(self.args))
        self.assertFalse(any(c[1:3] == ['s3api', 'get-object-acl'] for c in self.commands))

    def test_explicit_public_read_does_not_require_an_existing_homepage(self):
        self.config['Origins']['Items'][0]['DomainName'] = 'docs.pax.dev.s3-website-us-west-2.amazonaws.com'
        self.args.public_read = True
        self.assertTrue(publisher.preflight_publication(self.args))
        self.assertFalse(any(c[1:3] == ['s3api', 'get-object-acl'] for c in self.commands))

    def test_catalog_cannot_drop_history_or_move_latest_with_no_latest(self):
        remote = {'latest': '0.38.3', 'versions': [{'version': '0.38.3', 'path': '/0.38.3/'}]}
        local = {'latest': '0.39.0', 'versions': [{'version': '0.39.0', 'path': '/0.39.0/'}]}
        with self.assertRaisesRegex(SystemExit, 'omits or relocates'):
            publisher.validate_catalog_history(local, remote, no_latest=False)
        local['versions'] += remote['versions']
        publisher.validate_catalog_history(local, remote, no_latest=False)
        with self.assertRaisesRegex(SystemExit, 'latest catalog pointer'):
            publisher.validate_catalog_history(local, remote, no_latest=True)

    def test_rejects_staging_distribution_wrong_bucket_or_forced_caching(self):
        self.config['Aliases']['Items'] = ['staging.pax.dev']
        with self.assertRaisesRegex(SystemExit, 'alias'):
            publisher.preflight_publication(self.args)
        self.config['Aliases']['Items'] = ['docs.pax.dev']
        self.config['Origins']['Items'][0]['DomainName'] = 'other.s3.amazonaws.com'
        with self.assertRaisesRegex(SystemExit, 'bucket root'):
            publisher.preflight_publication(self.args)
        self.config['Origins']['Items'][0]['DomainName'] = 'docs.pax.dev.s3.amazonaws.com'
        self.ttl = 60
        with self.assertRaisesRegex(SystemExit, 'minimum TTL'):
            publisher.preflight_publication(self.args)

    def test_missing_distribution_cannot_begin_verified_upload(self):
        self.args.distribution_id = None
        with self.assertRaisesRegex(SystemExit, 'distribution ID'):
            publisher.preflight_publication(self.args)
        self.assertEqual(self.commands, [])


class LiveVerificationTests(unittest.TestCase):
    def setUp(self):
        self.output = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.args = argparse.Namespace(version='0.39.0', site_url='https://docs.pax.dev', no_latest=False)
        # Include dynamically loaded assets with no HTML reference: checking
        # only tags, entry HTML, or Wasm must not pass this fixture.
        self.files = {'index.html': '<pax-example path="demo"></pax-example>',
                      'getting-started.html': 'article', 'theme/pax-version.js': 'picker',
                      'theme/pax-example.js': 'embed host', 'css/general.css': 'styles',
                      'searchindex.js': 'search', 'searchindex.json': '{}',
                      'versions.json': '{}', publisher.BUILD_RECORD: '{}',
                      '_pax_examples/demo/manifest.json': '{}',
                      '_pax_examples/demo/app/index.html': 'app',
                      '_pax_examples/demo/app/pax-cartridge.js': 'bootstrap',
                      '_pax_examples/demo/app/snippets/inline0.js': 'dynamic import',
                      '_pax_examples/demo/app/demo.wasm': 'wasm',
                      '_pax_examples/demo/app/assets/a b.png': 'image',
                      'fonts/book.woff2': 'font'}
        for name, content in self.files.items():
            path = self.output / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        self.missing = self.stale = self.bad_mime = None
        self.cache_control = 'no-cache'
        self.javascript_mime = 'application/javascript'
        self.enterContext(patch.object(publisher, 'BOOK_OUTPUT', self.output))
        self.enterContext(patch.object(publisher.urllib.request, 'urlopen', self.fetch))

    def fetch(self, request, timeout):
        relative = urllib.parse.unquote(request.full_url.removeprefix(self.args.site_url + '/').removeprefix('0.39.0/'))
        if not relative or relative.endswith('/'):
            relative += 'index.html'
        if relative == self.missing:
            raise urllib.error.HTTPError(request.full_url, 404, 'missing asset', {}, None)
        response = io.BytesIO(b'stale' if relative == self.stale else self.files[relative].encode())
        response.headers = Message()
        response.headers['Cache-Control'] = self.cache_control
        types = {'.html': 'text/html', '.js': self.javascript_mime, '.css': 'text/css',
                 '.json': 'application/json', '.wasm': 'application/wasm'}
        response.headers['Content-Type'] = ('application/octet-stream' if relative == self.bad_mime
                                           else types.get(Path(relative).suffix, 'application/octet-stream'))
        response.geturl = lambda: request.full_url
        return response

    def test_checks_entire_tree_at_both_roots_and_accepts_javascript_mime_variants(self):
        expected = {self.args.site_url + '/' + prefix + urllib.parse.quote(path, safe='/')
                    for prefix in ('', '0.39.0/') for path in self.files}
        expected.update([self.args.site_url + '/', self.args.site_url + '/0.39.0/'])
        for mime in ('text/javascript', 'application/javascript'):
            self.javascript_mime = mime
            self.assertEqual(set(publisher.verify_live_output(self.args)), expected)

    def test_no_latest_checks_version_tree_and_root_catalog_only(self):
        self.args.no_latest = True
        urls = publisher.verify_live_output(self.args)
        self.assertEqual(len(urls), len(self.files) + 2)
        self.assertEqual([url for url in urls if '/0.39.0/' not in url], [self.args.site_url + '/versions.json'])

    def test_external_directory_entry_points_are_verified(self):
        for name in ('getting-started', 'template-language', 'targets-build-deploy'):
            relative = f'{name}/index.html'
            self.files[relative] = 'redirect page'
            target = publisher.BOOK_OUTPUT / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(self.files[relative])
        urls = publisher.verify_live_output(self.args)
        for prefix in ('', '0.39.0/'):
            for name in ('getting-started', 'template-language', 'targets-build-deploy'):
                self.assertIn(self.args.site_url + '/' + prefix + name + '/', urls)

    def test_missing_stale_or_wrong_type_assets_fail(self):
        for path in ('_pax_examples/demo/app/pax-cartridge.js', '_pax_examples/demo/app/snippets/inline0.js',
                     'theme/pax-example.js', 'css/general.css', 'searchindex.js', 'searchindex.json',
                     '_pax_examples/demo/app/demo.wasm', 'getting-started.html'):
            with self.subTest(path=path):
                self.missing = path
                with self.assertRaisesRegex(SystemExit, 'Unable to fetch live docs https://docs.pax.dev/'):
                    publisher.verify_live_output(self.args)
                self.missing = None
                self.stale = path
                with self.assertRaisesRegex(SystemExit, 'differ'):
                    publisher.verify_live_output(self.args)
                self.stale = None
                self.bad_mime = path
                with self.assertRaisesRegex(SystemExit, 'MIME'):
                    publisher.verify_live_output(self.args)
                self.bad_mime = None

    def test_requires_revalidation_and_complete_local_inputs(self):
        self.cache_control = 'max-age=86400'
        with self.assertRaisesRegex(SystemExit, 'do not revalidate'):
            publisher.verify_live_output(self.args)
        (self.output / 'versions.json').unlink()
        with self.assertRaisesRegex(SystemExit, 'Missing live verification inputs'):
            publisher.verify_live_output(self.args)


if __name__ == '__main__':
    unittest.main()
