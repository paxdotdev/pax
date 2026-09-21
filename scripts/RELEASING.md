# Pax release procedure

PAX-906 owns 0.39.0 preparation and publication; PAX-987 is its docs completion
gate. 0.39.x can repair pre-launch findings. Public launch is 0.40.0.
Zack owns commits, merges, tags, pushes, and final publication approval.
`scripts/release.py` never performs git writes.

## Prepare a candidate

Use Python 3.11+ with `tomlkit`, Cargo 1.93+, the workstation prerequisites in
[Getting Started](../pax-docs/book/src/getting-started.md), mdBook, and Node from
`.nvmrc`. On this workstation Python 3.12 has `tomlkit` installed. In another
environment, install it in a virtual environment and use that Python executable.

```sh
source ~/.zshrc
nvm use
python3.12 scripts/release.py plan 0.39.0
python3.12 -B -m unittest discover -s scripts/tests -v
python3.12 -B -m unittest discover -s pax-docs/tests -p 'test_publish*.py' -v
node --test pax-docs/tests/*.test.mjs
python3.12 scripts/release.py prepare 0.39.0
python3.12 scripts/release.py verify 0.39.0
```

`plan` only reads Cargo metadata. `prepare` changes local source and builds local
artifacts; it does not publish crates, upload docs, invalidate caches, or write
git state. The old `release.py VERSION` invocation and skip/turbo switches were
removed so an old command cannot accidentally publish. Preparation:

1. Discovers every publishable workspace crate using Cargo metadata, checks local
   dependency closure, and rejects publication cycles. `publish = false` keeps
   the repository's root `pax` harness out. There is no hand-maintained crate list.
   `[package.metadata.pax.release].after` declares non-linking release edges:
   `pax-cli` waits for `pax-kit` because its generated starter resolves that crate.
2. Aligns package versions in all source manifests and coordinated dependencies,
   including renamed dependencies, strings, tables, target/build/dev dependencies,
   workspace inheritance, and local dependencies between examples/fixtures.
   Path-only dev dependencies remain path-only: Cargo omits them from the package.
   This avoids the test-only `pax-chassis-web -> pax-std -> pax-engine` cycle.
3. Builds the web interface JS/CSS and regenerates/checks both canonical source
   bundles after version rewriting.
4. Runs the full no-upload docs publisher, including release-web examples,
   generators, mdBook, catalog update, and root/version link/anchor validation.
5. Runs Cargo workspace packaging for the discovered publishable crates, with normal archive build
   verification enabled. Cargo's temporary registry resolves unpublished sibling
   packages. Checks normalized dependency versions, required bundled files, and
   absence of transient build files in every `.crate` archive. Retains copies in
   the candidate's `archives/` directory, outside Cargo's packaging output.
6. Unpacks the actual archives into a temporary directory outside the checkout,
   builds the CLI there, reads embedded example sources, creates Living Quilt,
   and checks web build/run while blocking Node/npm. Registry patches point only
   to unpacked candidate archives; metadata verifies every resolved Pax source.
   This check uses a shared compilation cache but no monorepo source dependency.
7. Records commit, dirty state, source/lockfile/bundle hashes, package order,
   archive checksums/file lists, docs digest, and exact Cargo version in
   `target/release-candidate/0.39.0/`. Cargo's own `package --list` supplies an
   additional inventory of every packaging input, including ignored generated
   files, their contents, executable permissions, and symlink targets. Inputs
   must resolve inside the checkout. File additions and removals are checked too.
8. Repackages the complete set with `--registry crates-io --locked` in an isolated
   temporary target and compares every archive byte-for-byte by SHA-256. This
   assembly check uses `--no-verify` because normal archive compilation already
   ran in step 5. `verify` repeats the inventory, Cargo-version, and repackaging
   checks. A failed prepare removes the ready record. Older candidate records
   without this inventory must be prepared again.

Use `CARGO_BUILD_JOBS=4` to bound resource use. `CARGO_NET_OFFLINE=true` is useful
with pre-populated caches; record that choice because it restricts dependency
resolution to cached versions. Do not confuse archive verification or local
candidate smoke results with proof from crates.io. Inspect the ignored archive
and site outputs before deleting them: they are the review/recovery artifacts.

The docs version catalog can contain the local candidate before publication.
That entry alone does not establish that matching crates or docs are live.

## Read-only production preflight

```sh
AWS_PROFILE=pax python3.12 scripts/release.py preflight 0.39.0 \
  --docs-bucket docs.pax.dev \
  --docs-distribution-id E2EYK7TMGOPCYY
```

AWS commands inherit `AWS_PROFILE`; neither Python script hardcodes a profile.
The docs distribution `E2EYK7TMGOPCYY` uses `PaxDocsCachePolicy` with minimum TTL
zero. The distribution can also be supplied through
`PAX_DOCS_CLOUDFRONT_DISTRIBUTION_ID`.
Supply the actual docs distribution, not the staging-site distribution in CI.
Preflight checks AWS identity, bucket access, the enabled/deployed distribution,
HTTPS hostname/aliases, bucket-root routing, and zero minimum TTL across cache
behaviors/policies. It reads the deployed S3 catalog and rejects local history
omissions or moved version paths (and a changed latest pointer with `--no-latest`).
It also checks whether a crates.io token is configured,
without printing it. Token presence does not prove publish permission or new
crate-name ownership. AWS read permissions do not prove write permissions.

Failure here blocks publication, not local preparation. Refresh invalid AWS
credentials through the normal operator login process. Never place secrets in
issue comments, evidence files, or command arguments.

## Approval and publication

Review the changes, inventory, local docs, candidate-package checks, unresolved
limits, and this procedure with Zack. Zack then checkpoints the intended release
state. Rerun `prepare` and `verify` from that **clean exact commit** so archive VCS
metadata and the candidate record identify it. The script refuses a dirty
candidate, changed source/lockfile/bundles, modified archives/site, or a commit
other than `--approved-commit`. A merge/rebase requires fresh preparation.

Only after explicit publication approval:

```sh
AWS_PROFILE=pax python3.12 scripts/release.py publish 0.39.0 \
  --approved-commit FULL_APPROVED_COMMIT_SHA \
  --docs-bucket docs.pax.dev \
  --docs-distribution-id E2EYK7TMGOPCYY
```

The command reruns read-only preflight and full-set repackaging parity before
the first upload. Immediately before each missing crate's upload, it also checks
single-crate packaging against the live registry, now that its prerequisites
are published. Changed inputs, Cargo version, or archive bytes stop before that
upload. It publishes each crate in dependency order using `cargo publish --locked`
with build verification enabled. Cargo reassembles archives when publishing;
keep the checkout, generated inputs, toolchain, and Cargo configuration untouched
throughout the operation. These checks do not lock out concurrent writers.
The default
interval is 60 seconds. It verifies registry checksums and journals progress in
`target/release-candidate/0.39.0/publication.json` before advancing. It then invokes
the checked-in docs publisher with `--skip-build --verify-live`, reusing the exact
prepared site. Versioned content uploads first, root content next, and the root
catalog last. CloudFront invalidation must finish before live HTTPS verification.
Every prepared file is checked at root and `/<version>/`, including app bootstrap
scripts and dynamic imports, styles, search indexes, fonts, images, source
manifests, and Wasm. All bytes and `no-cache` headers must match; HTML, JavaScript,
CSS, JSON, and Wasm also require their serving MIME types. A version-only release
checks the version tree and the root catalog. Browser interaction checks remain
part of published-channel acceptance. See the
[docs publisher runbook](../pax-docs/scripts/README.md) for ghost-patch/cache policy.

Docs failure returns exit code **3**, independently of successful crate uploads;
other failures return nonzero. Never report the release complete on a partial
success. The docs receipt under `target/docs-publication/` records the invalidation
ID, digest, completion state, timestamp, and verified URLs.

## Partial failure and recovery

- Keep the approved checkout, Cargo.lock, `.crate` files, complete site tree,
  candidate record, journals, and previous complete docs build. Do not clean the
  worktree's targets until the release handoff is finished.
- If Cargo fails or times out after an upload, stop and inspect its output and the
  journal. Rerun the exact approved publish command. Each already-present version
  is skipped only when its registry checksum matches the reviewed archive and it
  is not yanked. An absent version is retried; any mismatch stops immediately.
  Never silently skip an arbitrary existing version. Crate versions cannot be
  overwritten: a defective published crate requires a coordinated 0.39.x repair;
  yanking requires a separate operator decision.
- If docs upload fails, crates may already be complete. The same publish command
  safely rechecks/skips matching crates and recopies the full prepared docs tree.
  The catalog is promoted only after content upload. A copied S3 tree is not an
  atomic deployment; a partial copy can leave mixed files until the retry finishes.
- If invalidation times out, inspect the recorded ID with
  `aws cloudfront get-invalidation --distribution-id ID --id INVALIDATION_ID`.
  Retry waits/uploads from the same approved artifact; a second invalidation is
  harmless. Do not announce completion until live checks pass.
- For bad docs content after deployment, restore a retained complete approved tree
  through the docs publisher, or prepare/review a same-current-semver ghost patch.
  Same-version patches are allowed; preserve other historical versions. Root is
  latest; there is no `/latest/` alias and no bucket-root `--delete` operation.
  Rollback/root promotion is another externally mutating action requiring approval.

## Published-channel acceptance

After publication, use the available macOS/Linux/Windows first-touch baselines:

```sh
cargo install pax-cli --version 0.39.0 --locked
pax-cli create my-first-project
cd my-first-project
pax-cli build --target=web
pax-cli run --target=web
```

Use a fresh directory outside the monorepo, no source patches, and no Node/npm
requirement for the generated app. Inspect Living Quilt, CLI docs/example sources,
and telemetry according to [CLI Telemetry](../pax-docs/book/src/cli-telemetry.md).
Record installation/build/run results and exact package versions. The
[first-touch harness](first-touch/README.md) supplies baseline details; its
`*-source` scripts deliberately patch local sources and are not this acceptance.
Hand the exact release to PAX-924 for unpatched fresh-project iOS validation and
to PAX-997 for its isolated AI NUX smoke. Complete live browser checks for docs
navigation, search, version switching, source tabs, runnable examples, responsive
layout, and warm-cache ghost patches. Keep PAX-906/PAX-987 open until their actual
publication and acceptance criteria are satisfied; Zack controls closure.
