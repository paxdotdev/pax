# Website whole-component loading experiment

An isolated continuation of the Paxflix fixture for PAX-1034. It snapshots an
active website without editing that checkout and runs it against this checkout's
engine. Calculator, shaders, source drawers and ordinary assets remain in the
website. Only Paxflix's complete component package is split out. This is not
a production module loader or a partitioner for every website component.

The child is the existing Paxflix fixture's binary: original Rust application
logic, component definitions and metadata, with host-owned reactive properties
and one shared renderer. A website wrapper forwards `embedded` and
`disable_keyboard`, retaining the same dimensions while waiting. Its readiness
property activates the normal component tree after registration. Standard
descriptors are captured from the website tree and a dormant capability union.
The prototype remains trusted, same-build, one-package-only.

## Reproduction

First build the adjacent `paxflix-cartridge-probe`. Supply its external output,
the website source to snapshot, and a compatible compiled web interface:

```sh
source ~/.zshrc
nvm use
python3 tests/src/website-cartridge-probe/build.py \
  --website /absolute/path/to/pax-website \
  --paxflix-output /private/tmp/pax-1034-paxflix \
  --interface /absolute/path/to/web-interface-output \
  --output /private/tmp/pax-1034-website
```

Builds use `opt-level=z`, LTO, one codegen unit, and no wasm-opt. Generated
crates, locks, manifests and binaries stay in the external output directory.
Use `--phase pruned` and `--phase mount-only` to build the two attribution controls.
`pruned` removes only the unused Paxflix standalone entrypoint from a private
dependency copy. `mount-only` adds the same readiness wrapper to that monolith,
delaying scene construction without a child WASM. The source snapshot is created only once. Source drawers retain their original
text in both variants, including source files containing substituted imports.
The entry export is selected by Cargo package/library identity because runnable
dependencies can export additional entrypoints.

Create a temporary localhost certificate in that output directory, then start
`node tests/src/website-cartridge-probe/serve.cjs /private/tmp/pax-1034-website`.
The server expects `localhost-key.pem` and `localhost-cert.pem`. It serves gzip-9
code and unchanged media. Code uses no-store; assets allow reuse within a page.
Every sample starts with a fresh browser profile. HTTP preview ports are 18754
(baseline) and 18755 (split); HTTPS benchmark ports are 18756 and 18757, using
HTTP/2. The benchmark accepts this local certificate without installing it.
The pruned control uses HTTP 18758 / HTTPS 18759; the mount-only control uses
HTTP 18760 / HTTPS 18761. Include `pruned,mount-only` in `VARIANTS` to measure them.

`bench.cjs NEW_OUTPUT_DIRECTORY` uses Playwright and Chrome, with a fresh browser
process per sample. Set `NODE_PATH` to the installed Playwright package location.
It refuses to overwrite an existing run. `TRIALS`, `PROFILES=local,network`, and
`VARIANTS=baseline,parallel,sequential,deferred` select the run. Network emulation
is 10 Mbps and 80 ms via CDP; requests share that profile. Alternate variant order
rotates each trial. `summarize.py BUILD_DIRECTORY RUN_DIRECTORY` aggregates the
saved measurements and hashes/sizes the actual binaries.
`CACHE_DISABLED=1` reproduces the separate stress control, which can redownload
the same font for different font faces. The primary cold-navigation run keeps
normal within-page asset reuse and excludes cache hits from wire-byte totals.

## Policies and measurements

- Baseline: ordinary monolithic website with yielded startup.
- Parallel: fetch/compile the child alongside the base, register before mounting.
- Sequential: fetch the child after base initialization, register before mounting.
- Deferred: present the website's first canvas submission, allow a paint
  opportunity, then fetch/register/activate Paxflix.
- Manual: keep the child absent until `probe.activate()` is called; useful for
  checking initial payload and the mobile layout independently.
- Pruned and mount-only: isolate unused-entrypoint retention and scene scheduling
  from actual component splitting. These retain the complete typed Paxflix code.

Text reveal (`firstViewportPresentation`) is the existing coordinated native
viewport checkpoint. `firstCanvasSubmission` is the first nonempty result of
the renderer's surface submission call, not a physical screen timestamp.
`startupSettled` is the web host's mount promise; deferred child mounting can
occur after that promise. Resource completion requires a fresh quiet interval
after both settlement and package activation, tracking new requests rather than
reusing a previously reached browser `networkidle` state. The load-complete
summary includes these milestones and requested resource response completion;
it does not imply every image has decoded or every offscreen gallery card has
mounted. Interactive/animated pages have no universal “entire load” timestamp.
An additional animation-frame checkpoint waits for activated scene work. All
transfer totals include the navigation document and exclude protocol headers.
Gzip sizes use the server's Node/zlib implementation; other gzip implementations
can produce different bytes even at level 9.

The harness uses the ordinary single base-WASM fetch. Both variants separate
byte-buffer compilation and instantiation for timing; neither gains streaming
compilation from this instrumentation. The loader hook runs after wasm-bindgen
initializes the base, before the existing generated Pax entrypoint. Only loading
and registration use JavaScript; component event/property traffic uses the
existing direct WASM bridge. Assets remain separate requests.

Compare eventual bytes as well as initial bytes. Parallel streams still share
connection bandwidth; a shorter critical path is not a reduction in total
transfer. Preserve raw samples and report CPU contention and external resources.
Detailed results and design conclusions belong in PAX-1034.
