# Versioned documentation publication

`publish_versioned_docs.py` builds the source checkout, updates the explicit
version catalog, and optionally uploads to S3/CloudFront. PAX-906 owns the coherent
crate/CLI release; PAX-987 owns the production docs deployment. Both need Zack's
approval before publication. This script does not release crates.

## Local preflight

From the repository root, with the release's crate versions and bundled sources
already aligned:

```sh
python3 -B -m unittest discover -s pax-docs/tests -p 'test_publish*.py' -v
source ~/.zshrc
nvm use
node --test pax-docs/tests/*.test.mjs
cargo test -p pax-docs --bin gen_api_docs --bin gen_example_docs --test source-bundle
python3 scripts/sync-cli-examples.py --check
python3 scripts/sync-docs-examples.py --check
python3 pax-docs/scripts/publish_versioned_docs.py 0.39.0 --no-upload
node pax-docs/scripts/check_publication_output.mjs
```

The tests intercept every external publication command; they require no AWS
credentials. The publisher command runs the actual docs generators and mdBook,
including runnable examples, with **no upload or CDN invalidation**. It updates
`pax-docs/book/src/versions.json` locally and writes the reviewed site to
`pax-docs/book/book/`. A candidate version here is a local preview, not evidence
that matching crates have been published. Inspect the catalog diff before
checkpointing it. `--skip-examples` reuses existing example builds and must not
substitute for release-example verification. The publisher checks each embedded
example for a successful build, matching source/build fingerprints, an entry
document, Wasm, and embedded source. A failed example build cannot silently
become a source-only placeholder in a published site.
The publisher also runs the final link check; the standalone command is useful
when inspecting an existing build. It validates article/API links and anchors
at both the bare root and a semver prefix. Generated API links remain inside
the selected version.

The publisher records the version and a SHA-256 digest of the generated tree in
`_pax_docs_build.json`. A later `--skip-build` upload requires that version and
the same content. Catalog-only changes are permitted and the output catalog is
refreshed from source immediately before upload. Missing, modified, or differently
versioned output requires rebuilding; there is no bypass flag. This checks build
reuse integrity, not compatibility with the public crate registry.

`--skip-build --no-upload` has a separate, manifest-only meaning used by legacy
integrations. That combination performs no build validation and is not a
substitute for the full preflight above. `scripts/release.py prepare` runs the
full build instead. The release wrapper forwards `--docs-no-latest` during both
local preparation and publication.

## Publication contract

After release and deployment approval, the same command without `--no-upload`
publishes; `--skip-build` can reuse the exact preflight output. Supply `--bucket`
and `--distribution-id`, or `PAX_DOCS_S3_BUCKET` and
`PAX_DOCS_CLOUDFRONT_DISTRIBUTION_ID`. The bucket defaults to `docs.pax.dev`.
The integrated release requires the actual distribution ID. It waits for
invalidation completion and uses `--verify-live` to preflight AWS identity,
bucket routing, hostname and cache policy, then compare every file in the
prepared tree with its deployed copy at both the root and version prefix.
This includes all articles, JS bootstrap/dynamic imports, CSS, search assets,
fonts, images, example source manifests, and Wasm. Every file must have matching
bytes and `no-cache`; HTML, JavaScript, CSS, JSON, and Wasm must also have an
appropriate MIME type. JavaScript accepts `text/javascript` or
`application/javascript`. Under `--no-latest`, only the version tree and root
catalog are checked. Missing or incorrectly served dependencies fail publication
even when the example's HTML and Wasm succeed. This verifies delivery; interactive
browser acceptance is still required. A publication receipt in
`target/docs-publication/<version>.json` records progress and verified URLs.
Standalone publication without a distribution ID still has no invalidation; it
does not satisfy the integrated release completion gate.

AWS CLI calls inherit the environment. Use `AWS_PROFILE=pax` on this workstation
and `--distribution-id E2EYK7TMGOPCYY` for the docs distribution. Its
`PaxDocsCachePolicy` has minimum TTL zero so origin `no-cache` is honored.

- `/<version>/` is the release-specific tree. Same-semver ghost patches are
  allowed. Strict SemVer is required, without a `v` prefix; stable versions sort
  after prereleases in ascending precedence, and build metadata does not affect
  precedence.
- The bare site root is mutable latest. There is no separate `/latest/` tree.
- `--no-latest` updates the selected version and catalog while preserving the
  existing latest pointer and root articles. Even a first publication with no
  previous latest leaves that pointer null; the picker does not guess a version.
- Content uploads finish before the root `/versions.json` is replaced. A failed
  upload stops the process, so it does not proceed to catalog promotion or CDN
  invalidation. A copied static site is not an atomic deployment: a partial
  upload can still leave mixed files. Rerun the approved build to completion
  before announcing it; retain the previous complete build for recovery.
- Each target tree is copied in full with `Cache-Control: no-cache`, including
  unchanged files. The selected version prefix is then synced with `--delete`
  to remove its stale files. Root is never synced with `--delete`, because it
  also contains historical versions. Removed root-only URLs may consequently
  remain; any cleanup needs a separately reviewed, explicitly scoped object list.
- A version-only upload invalidates `/<version>/*` and `/versions.json`. A latest
  promotion invalidates `/*`, which includes its versioned copy. Other version
  objects are not overwritten or deleted.

`no-cache` permits cached storage but requires revalidation. Content-hashed files
could use long-lived immutable caching later; today's stable filenames cannot
do so while supporting ghost patches. A full copy ensures origin metadata is
refreshed even when bytes are unchanged, which `s3 sync` alone does not guarantee.
See [AWS sync behavior](https://docs.aws.amazon.com/cli/latest/reference/s3/sync.html)
and [HTTP cache directives](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control).

CloudFront invalidation cannot evict files already cached in a reader's browser
under the former one-year policy. Such previously served URLs can remain stale
until reload/expiry; a new URL is needed to guarantee an immediate fresh fetch.
Also verify the deployed CloudFront cache policy (including minimum TTL) honors
the origin's intended revalidation policy. These are production checks, not
properties established by the local mocks.

## PAX-906 / PAX-987 handoff checklist

1. Checkpoint and integrate the docs/runtime follow-ups before taking the release
   commit. Do not use a stale worktree build as the release artifact.
2. PAX-906 aligns **all** crate versions/dependencies for 0.39.0, regenerates the
   CLI starter archive and docs source snapshot after version rewriting, and
   checks package inclusion. `scripts/release.py` already invokes both snapshot
   generators; this publication work does not certify its entire release graph.
3. Verify packaged `pax-compiler` contains its prebuilt web JS/CSS and bundled
   starter, and `pax-docs` contains its example source snapshot and docs assets.
   Validate the candidate package set outside the monorepo. Source patches back
   to this checkout are not publish-channel proof.
4. After the approved crate release, install the actual published CLI, create
   Living Quilt outside the monorepo, and run the documented web build/run path.
   Check CLI docs and example source visibility. Record the macOS/Linux/Windows
   first-touch results and hand off to PAX-997 as specified by PAX-906.
5. PAX-987 reruns the full no-upload docs build from that exact release state,
   reviews the catalog/latest selection and complete example payloads, then
   obtains production upload approval. The release script always includes docs
   publication and returns a distinct failure if that stage fails. See
   [the release procedure](../../scripts/RELEASING.md)
   for the exact approval, retry, and recovery commands.
6. After the approved upload, wait for invalidation completion and verify HTTPS,
   cache headers, Wasm MIME type, version switching, direct article links,
   source tabs, and runnable examples at both root and the version prefix.
   For a ghost patch, include a warm-cache check. Record the release commit,
   version, deployment time, test results, and recovery artifact in the issues.
