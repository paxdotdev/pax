# Pax-Compiler

This document describes the high-level archiecture of pax-compiler.

For more information refer to our [docs](https://docs.pax.dev/reference-compilation-model.html).

## Bird's Eye View

Roughly it can be broken down into three steps: 
1) Analyze the user Pax project (Pax Template + Rust) and generate a Pax Manifest (data structure summarizing the Pax project)
2) Code-gen a Pax Manifest into a Pax Cartridge (Rust target agnostic library).
3) Build a target platform executable (chassis) with this rust cartridge included. 

The main entry-point for all of this is `perform_build` found in `lib.rs`.

### Step 1: Pax Project -> Pax Manifest 

Pax projects decorate their associated Rust with a special macro `#[derive(Pax)]`. These macros generate code to dynamically analyze their tagged structs. They each add a `parse_to_manifest` function for every `#[derive(Pax)]` tagged struct. This `parse_to_manifest` function (template found [here](https://github.com/pax-lang/pax/blob/master/pax-macro/templates/derive_pax.stpl)) stores its associated structs information in a ParsingContext object and calls `parse_to_manifest` on its Pax Template dependencies. It utilizes logic in `parsing.rs` and relies on our pest grammar (`pax.pest`) to understand the template dependencies. For the root struct (tagged `#[main]` e.g. [here](https://github.com/pax-lang/pax/blob/aabc8978085a65a5369b7b5a61c00d620d5b5c81/examples/src/camera/src/lib.rs#L7)), we generate a binary target as well that starts the process and writes the accumulated information into a Pax Manifest and serializes it to stdout. This binary (named `parser`) is kicked off (`run_parser_binary`) as our first step of the compilation process to generate the Manifest in the `lib.rs/perform_build` function. 

### Step 2: Pax Manifest -> Pax Cartridge

Next we generate the Pax Cartridge. This work is roughly two steps: compiling [expressions](https://docs.pax.dev/start-key-concepts-expressions.html) and generating the cartridge code. The former involves parsing Paxel (Pax's expression language) and generating the equivalent rust code. This work lives in `expressions.rs`. Once expressions are compiled, the second step is generating the cartridge code. This lives in the `code_generation` module. We utilize [Tera](https://keats.github.io/tera/) templates for the code-gen and the bulk of this work is translating a Pax Manifest into a Tera context. The main entry point is `generate_and_overwrite_cartridge` in `code_generation/mod.rs`.


### Step 3: Building a Chassis with our Pax Cartridge

The last step of this process involves building our target platform (e.g. Web/MacOS/..) scaffolding (see [chassis](https://docs.pax.dev/reference-compilation-model.html#3-chassis-compilation)) with our cartridge included. This work lives in the `building` module. This mainly involves building the generated cartridge, loading it into our specific chassis and then building that chassis. Currently we support 3 targets (Web/MacOS/iOS). We load our cartridge as [WASM](https://webassembly.org/) for Web and as `.dylib`s for our Apple targets.

## Consumers

The main consumers of `pax-compiler` are `pax-cli` and `pax-macro`. `pax-cli` is the CLI that Pax user's invoke to build Pax projects. `pax-macro` is the crate the defines the `#[derive(Pax)]` macro that we use for dynamic analysis of the Pax Project (see Step 1). 