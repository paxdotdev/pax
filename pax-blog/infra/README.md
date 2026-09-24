# Shared website hosting — deployment pending

Updated 24 September 2026. The selected plan publishes the contents of
`pax-blog/public/` to **`s3://www.pax.dev/blog/`**, reusing the existing S3 website
origin and CloudFront distribution. Terraform adoption is deferred. No production
configuration, upload, or invalidation has been performed by PAX-984.

## Existing publisher and ownership

The existing publisher was found in the canonical Pax checkout's sibling repo:
`../www.pax.dev/deploy.prod.sh` (on the inspected workstation,
`/Users/zack/code/www.pax.dev/deploy.prod.sh`). No deployment script was found in
this checkout's `examples/src/pax-website/`.

The script builds with npm, then runs:

```sh
aws --profile=pax s3 sync --exclude ".git/*" --acl=public-read ./public/ s3://www.pax.dev/
aws --profile=pax cloudfront create-invalidation --distribution-id=EYKZZ3KH242XU --paths "/*"
```

**It currently does not use `--delete`.** Existing blog objects therefore are not
removed merely because the website build lacks them. However, website output
with colliding blog paths can overwrite them, and future deletion needs an
explicit boundary.

The website-side changes belong to
[PAX-869](https://linear.app/paxdev/issue/PAX-869/website-phase-1-launchable#comment-06ff7c60-e3ff-4142-ae65-1e72ad90a1fe).
The handoff was also sent directly to its active Codex task. The sibling repo and
active website worktree were not modified here.

Reserve these publication scopes:

| Publisher | Destination | Boundary |
| --- | --- | --- |
| Website | `s3://www.pax.dev/` | Exclude `blog/*` from every root upload/sync, including deletion |
| Blog | `s3://www.pax.dev/blog/` | All writes and any reviewed cleanup stay under this prefix |

For the current root sync, add `--exclude "blog/*"`. If an exact `blog` object is
possible, exclude that too. Place these final exclusions after any broad include
filters; do not let later includes reselect the blog. AWS sync also excludes
filtered objects from deletion. Do not replace this with a recursive bucket-root
removal. Test both a destination-only blog object and a colliding local preview
file so preservation covers deletion and overwrites.

The proposed root-level `pax-website/` move belongs to PAX-869 and remains a
recommendation for Zack's decision. Keep `pax-blog/` as its sibling; site-wide
hosting configuration and combined publication orchestration belong to the
website. On a move, update Cargo workspace membership/exclusion, relative crate
and favicon paths, launcher paths, docs/example source discovery, and `blog.py`'s
local stage/unstage location. The current `examples` workspace exclusion does not
automatically cover a root-level website. The site should be non-publishable.

## Existing hosting snapshot

Read-only AWS inspection with profile `pax` on 22 September 2026 found:

| Setting | Value |
| --- | --- |
| Distribution | `EYKZZ3KH242XU` (`d4bphiy9da05g.cloudfront.net`) |
| Alias | `www.pax.dev` |
| Origin | `www.pax.dev.s3-website-us-west-2.amazonaws.com` |
| Origin ID | `pax-designer.webflow.io` (historical name) |
| Origin protocol | HTTP to the S3 website endpoint |
| Viewer policy | Redirect HTTP to HTTPS; existing ACM certificate, TLS 1.2 minimum |
| Default cache policy | CachingOptimized, `658327ea-f89d-4fab-a63d-7e88639e58f6` |
| Behaviors / functions / custom errors | None |
| Bucket website configuration | `index.html` index suffix; no error document or redirect rules |

Re-read the configuration before changing it. The apex `pax.dev` and the
docs/newsletter use separate hosting and are outside this change.

S3 website hosting already resolves directory indexes. The uploaded object keys
retain the blog prefix:

| Public URL | Bucket object key |
| --- | --- |
| `/blog/` | `blog/index.html` |
| `/blog/pax-0-39-0/` | `blog/pax-0-39-0/index.html` |
| `/blog/main.css?h=…` | `blog/main.css` |
| `/blog/atom.xml` | `blog/atom.xml` |

Use canonical trailing-slash article URLs and check the S3 website's slashless
redirect through CloudFront. No new origin, origin path, prefix stripping, or
edge directory-index function is required for these paths.

The current default behavior can route these requests to the existing origin.
Add `/blog` and `/blog/*` behaviors only as needed to select an appropriate blog
cache policy: minimum TTL zero, with origin revalidation/short cache directives
honored. Tabi uses `h` stylesheet query fingerprints; include `h` in the cache key
if relying on query-version caching. Other query parameters need not fragment the
cache. A distribution change should start from fresh configuration, preserve
unrelated fields, show a reviewable diff, and use the returned ETag with
`--if-match`. Do not add a global 403/404-to-Pax response fallback; blog misses
must retain error status. Add `https://www.pax.dev/blog/sitemap.xml` to the root
website's `robots.txt`.

The website must also publish the PAX-984 client change with `/blog` in
`server_owned_prefixes`. Cargo metadata controls client/dev-server ownership;
it does not configure CloudFront. Preserve the removal of the old generated
blog Router placeholder when integrating the two workstreams.

## Publication contract to implement

The local build helper currently contains no publishing commands. This contract
is the handoff for the small blog publisher and PAX-869's website integration:

1. Use AWS CLI `--profile pax` (or the existing publisher convention
   `AWS_PROFILE=pax`). Let AWS CLI resolve credentials on the authorized machine;
   scripts must not read or embed secrets. Confirm the expected account, bucket,
   distribution and current object-access/cache configuration before uploading.
2. Finalize the approved announcement's copy, date and metadata before removing
   `draft = true`. Run `python3 pax-blog/blog.py build` for production output.
   Reject draft/loopback preview content. Remove any website-local draft stage
   with `blog.py unstage` before its release build. A combined website/blog
   publication builds and validates both production outputs from the same
   checkout before the first upload. "Latest blog" means that checkout's approved
   production content, not an implicit fetch of unreviewed remote content.
3. Upload assets before documents/feed/sitemap into `s3://www.pax.dev/blog/`.
   Preserve the existing public-read access model after checking it; private
   default ACLs may make objects unreadable through the website endpoint.
   Set correct MIME types and deliberate cache headers, such as `no-cache` for
   mutable files with a CloudFront minimum TTL of zero. Query-hashed stylesheet
   names are still mutable object keys. A plain sync can skip unchanged bytes
   whose metadata needs correction; use a full copy/metadata refresh when needed.
   Any blog deletion must be deliberately scoped to the blog prefix and account
   for historical post URLs. Never clean the shared bucket root from the blog.
4. The website's root upload always excludes the blog, including in a combined
   publication. Invoke the blog publisher separately for its owned prefix. Keep
   blog-only publishing and a website-only mode that preserves existing blog
   objects; do not require a Wasm build just to publish an article.
5. After successful uploads, blog-only publication invalidates `"/blog*"` on
   `EYKZZ3KH242XU`. A combined publication can invalidate `"/*"` once after both
   uploads. Wait for invalidation completion, then verify live HTML and assets,
   canonical URLs, feed/sitemap, MIME/cache headers, deep links, slash handling,
   missing-page status, and unchanged homepage/AI paths. Article documents must
   not bootstrap Pax. Retain the previous complete outputs for recovery; multiple
   S3 uploads do not make an atomic release.

Before accepting the publisher changes, exercise preservation with sentinel
blog objects and a local colliding preview, with and without `--delete`; verify
that a failed blog build prevents all uploads in a combined release. Test AWS
command construction without production writes, then perform production
acceptance only during an authorized deployment.

Final copy, archive migration, old-host redirects, embedded demos and production
acceptance remain separate work. Existing Substack delivery stays in place.

## Superseded Terraform draft

`main.tf`, its provider lock, and `blog-paths.js`/test are retained as the historical
22 September private-origin proposal. **Do not apply that draft:** it creates a
separate bucket and strips `/blog`, neither of which belongs to the selected
shared-bucket plan. Its previous schema/test checks are historical evidence only.
If Terraform is adopted later, migrate ownership of the complete website
configuration into the website's infrastructure directory with shared state;
do not introduce a second independent owner of this distribution under the blog.

## Primary references

- [S3 website directory indexes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/IndexDocumentSupport.html)
- [AWS sync filters, deletion and metadata](https://docs.aws.amazon.com/cli/latest/reference/s3/sync.html)
- [CloudFront update semantics and ETags](https://docs.aws.amazon.com/cli/latest/reference/cloudfront/update-distribution.html)
- [CloudFront invalidation paths](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/invalidation-specifying-objects.html)
- [AWS CLI named profiles](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html)
