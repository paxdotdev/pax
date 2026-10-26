# Paxflix whole-component loading fixture (PAX-1034)

This isolated web experiment packages Paxflix's existing templates, structured
property schema, catalog, and Rust handlers into a deferred WASM module. The host
owns the existing Pax renderer, scene, reactive graph, animations, and standard
component implementations. Image files remain ordinary external assets and are
requested by the existing image loader when the component mounts.

This is a trusted, same-build prototype, not a supported public cartridge ABI.
The generated guest copies `examples/paxflix` without rewriting application
methods; it replaces only the runnable entry and supplies property/context
bindings. No production compiler or runtime source is changed.

## Run

Build outputs and generated source crates belong outside the repository. These
commands run from the repository root. Supply a built web interface from the
same source revision (the directory containing `pax-interface-web.js` and CSS).
The matching wasm-bindgen CLI is required; set `PAX_PROBE_BINDGEN` to override
the default workstation cache path for version 0.2.129.

```sh
python3 tests/src/paxflix-cartridge-probe/build.py \
  --output /private/tmp/pax-1034-paxflix \
  --interface /path/to/matching/web-interface
python3 tests/src/paxflix-cartridge-probe/serve.py \
  /private/tmp/pax-1034-paxflix --port 18753
```

Open `/split/` and click **Load Paxflix**. `/baseline/` has the same initial shell
but statically links the original Paxflix. `/baseline/?eager` mounts Paxflix
at startup. `/shell/` retains the standard runtime capabilities with an empty
outlet. `/decoder/` adds only the existing binary decoder to that shell. These
last two controls isolate the decoder and broader adapter footprint.

For browser validation and measurement, make Playwright available through
`NODE_PATH`; set `CHROME` to override the default local Chrome executable path.
Initialize this repository's Node version first (`source ~/.zshrc && nvm use`).

```sh
node tests/src/paxflix-cartridge-probe/check.cjs http://127.0.0.1:18753 /private/tmp/pax-1034-paxflix
node tests/src/paxflix-cartridge-probe/bench.cjs http://127.0.0.1:18753 /private/tmp/pax-1034-paxflix
node tests/src/paxflix-cartridge-probe/decoder.cjs
python3 tests/src/paxflix-cartridge-probe/summarize.py /private/tmp/pax-1034-paxflix
```

`--phase` supports rebuilding individual artifacts. The output snapshot records
the repository revision and source hashes. Reuse one snapshot for comparisons;
use a fresh output directory when testing another application revision.

## Contract exercised

- Separate memories and allocators. A single raw WASM import carries operations;
  a host function-table entry calls the guest. JS handles fetching and staging,
  with no JS callback in property reads/writes or event dispatch.
- Structured values cross as serialized `PaxValue` data. Scalars retain typed
  host properties so bindings to standard components and interpolation preserve
  their normal behavior. The current transport copies four bytes per bridge
  call; this is measurable prototype overhead, not an optimized wire format.
- Guest closures reference generation-tagged host property handles. The actual
  computations and scoped subscriptions live in the host graph and invoke the
  retained guest closures. Dropping host closures releases guest captures.
- The normal descriptor traversal constructs nested components, repeats,
  conditions, imported settings, and timelines. Event payloads include keyboard,
  pointer, wheel, touch-start, and viewport-proximity data used by Paxflix;
  cancellation returns to the host event.
- Browser checks cover dialogs, feedback, Escape, theme bindings, animated row
  scrolling, wheel scrolling, mobile layout, and complete handle/callback
  release on unmount followed by remount.

The adapter supports this application's API surface. It does not provide a
sandbox for arbitrary modules, runtime-feature loading, hot replacement,
multiple packages, native chassis support, arbitrary application services, or
custom easing callbacks. Descriptor banks are bounded to 16 component types
and 64 handlers. The primitive registry is obtained from a dormant capability
union in the host template; a production compiler would need an explicit
registry and dependency negotiation. Both the binary manifest and generated
schema are shipped inside the child, including nested type definitions.

## Measurement boundaries

Release builds use `opt-level=z`, LTO, one codegen unit, and wasm-bindgen, without
wasm-opt. The local server precompresses payloads at gzip level 9 and serves
images unchanged. Transfer totals exclude HTTP headers. Media totals distinguish
observed first-view requests from the complete image inventory; `prompts.json`
is authoring metadata and is not requested by the UI.

The benchmark alternates baseline/split order in fresh Chrome processes, with
seven local pairs and five pairs using CDP's 10 Mbps / 80 ms network emulation
by default; `TRIALS` overrides both counts and `PROFILES` selects the profiles.
The interface performs exactly one ordinary base WASM fetch. The harness
separates compile and instantiate timing by splitting that ordinary byte-buffer
instantiation into the equivalent two WebAssembly calls in both variants.
`pax_scroll_perf` records the first runtime tick and frame. Mount-to-DOM and
subsequent frame callbacks are separate observations, not claims of a physical
GPU presentation timestamp. Resource completion does not measure image decode
or GPU upload completion. These desktop results do not establish mobile or
production-network performance.

Detailed results and design discussion belong in PAX-1034. The external output
contains the exact WASM files, source snapshot, locks, logs, screenshots, raw
measurements, and machine-readable summary needed to audit a run.
