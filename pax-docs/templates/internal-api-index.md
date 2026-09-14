<a id="internal-api-docs"></a>

# Maintainer Reference
<!-- summary: Internal engine APIs and historical design context for primitive authors and Pax maintainers. -->
<!-- tags: api, internal, maintainer, architecture -->
<!-- Generated into book/src/api/internal/index.md by gen_api_docs; edit this template. -->

This reference is for work on Pax itself and for extensions that need its
lower-level interfaces. It covers the language, compiler representation,
runtime, platform messages, and renderer. Application authors can usually
stay with the [guide chapters](../../components-composition.md) and
[public API reference](../index.md).

For a first look inside the engine, read [How Pax Runs](../../how-pax-runs.md).
If you are implementing an element through runtime hooks, begin with
[Primitives](../../primitives.md), then use the runtime reference and source
to follow the relevant lifecycle and rendering contract.

## Source contracts and design notes

The generated pages below describe Rust declarations and their documentation
comments at this source revision. Internal interfaces evolve with the engine;
inspect the implementation and tests for the Pax version you are extending.
In particular, check debug and release paths when changing information that
crosses the compiler/runtime boundary.

The [Runtime and Cartridge Notes](../../architecture-runtime-cartridge.md)
are a historical design appendix. They include proposed directions and older
packaging models; their status is different from a tested current API.
Internal design records elsewhere in the repository likewise need to be read
in the context of their implementation status. For current build artifacts,
use [Targets, Build, and Deployment](../../targets-build-deploy.md).

## Crates

The crate names follow the engine boundary: `pax-language` handles Pax syntax
and expressions, `pax-manifest` holds program representations, `pax-runtime`
executes the scene, `pax-message` defines platform messages, and `pax-gpu`
implements the GPU renderer.
