# Pax-Compiler

This document describes the high-level architecture of pax-compiler.

For more information refer to our [docs](https://docs.pax.dev/reference-compilation-model.html).

## Bird's Eye View

Roughly it can be broken down into three steps: 
1. Analyze the user Pax project (Pax Template + Rust) and generate a Pax Manifest (data structure summarizing the Pax project)
2. Code-gen a Pax Manifest into a Pax Cartridge (Rust target agnostic library).
3. Build a target platform executable (chassis) with this rust cartridge included.

The main entry-point for all of this is `perform_build` found in `lib.rs`.

### Step 1: Pax Project -> Pax Manifest 

Pax projects decorate their associated Rust with a special macro `#[derive(Pax)]`. The compiler now builds the Pax Manifest through static analysis of the project source, using `static_analysis.rs` plus the Pax parser grammar (`pax.pest`) for template contents. `perform_build` calls this static-analysis path directly before generating the cartridge, so project manifests no longer require a generated `parser` binary target or macro-generated `Reflectable` manifest code.

### Step 2: Pax Manifest -> Pax Cartridge

Next we generate the Pax Cartridge. This work is roughly two steps: compiling [expressions](https://docs.pax.dev/start-key-concepts-expressions.html) and generating the cartridge code. The former involves parsing Paxel (Pax's expression language) and generating the equivalent rust code. This work lives in `expressions.rs`. Once expressions are compiled, the second step is generating the cartridge code. This lives in the `code_generation` module. We utilize [Tera](https://keats.github.io/tera/) templates for the code-gen and the bulk of this work is translating a Pax Manifest into a Tera context. The main entry point is `generate_and_overwrite_cartridge` in `code_generation/mod.rs`.


### Step 3: Building a Chassis with our Pax Cartridge

The last step of this process involves building our target platform (e.g. Web/MacOS/..) scaffolding (see [chassis](https://docs.pax.dev/reference-compilation-model.html#3-chassis-compilation)) with our cartridge included. This work lives in the `building` module. This mainly involves building the generated cartridge, loading it into our specific chassis and then building that chassis. Currently we support 3 targets (Web/MacOS/iOS). We load our cartridge as [WASM](https://webassembly.org/) for Web and as `.dylib`s for our Apple targets.

## Consumers

The main consumers of `pax-compiler` are `pax-cli` and `pax-macro`. `pax-cli` is the CLI that Pax users invoke to build Pax projects. `pax-macro` defines the `#[derive(Pax)]` macro used by authored Pax projects.
