extern crate proc_macro;
extern crate proc_macro2;
use std::str::FromStr;

use proc_macro2::{Ident, Span, TokenStream};
use quote::__private::ext::RepToTokensExt;
use quote::quote;
use pax_compiler_api::{TemplateArgsMacroPaxPrimitive, TemplateArgsMacroPax};

use syn::{parse_macro_input, Data, DeriveInput};


#[proc_macro_attribute]
pub fn pax_primitive(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let original_tokens = input.to_string();

    let input = parse_macro_input!(input as DeriveInput);

    let pascal_identifier = input.ident.to_string();

    let output = pax_compiler_api::press_template_macro_pax_primitive(TemplateArgsMacroPaxPrimitive{
        pascal_identifier,
        original_tokens: original_tokens
    });


    TokenStream::from_str(&output).unwrap().into()
}

#[proc_macro_attribute]
pub fn pax_type(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    //similar to pax_primitive: registers annotated type with PropertiesCoproduct
    //and generates instantiation code from a PropertiesCoproduct —
    //unlike pax_primitive, does not expect implementation of RenderNode
    //(maybe this isn't unlike pax_primitive after all?
    input
}

#[proc_macro_attribute]
pub fn pax(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let original_tokens = input.to_string();

    let input_parsed = parse_macro_input!(input as DeriveInput);
    let pascal_identifier = input_parsed.ident.to_string();

    let raw_pax = args.to_string();
    let dependencies = pax_compiler_api::parse_pascal_identifiers_from_component_definition_string(&raw_pax);

    let output = pax_compiler_api::press_template_macro_pax_root(TemplateArgsMacroPax {
        raw_pax,
        pascal_identifier,
        original_tokens: original_tokens,
        is_root: false,
        dependencies,
    });

    // println!("Macro output: {}", &output);

    TokenStream::from_str(&output).unwrap().into()
}

// Exactly like `#[pax()]`, except specifies that the attached component is intended to be mounted at
// the root of an app-contained cartridge
#[proc_macro_attribute]
pub fn pax_root(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let original_tokens = input.to_string();

    let input_parsed = parse_macro_input!(input as DeriveInput);
    let pascal_identifier = input_parsed.ident.to_string();

    let raw_pax = args.to_string();
    let dependencies = pax_compiler_api::parse_pascal_identifiers_from_component_definition_string(&raw_pax);

    let output = pax_compiler_api::press_template_macro_pax_root(TemplateArgsMacroPax {
        raw_pax,
        pascal_identifier,
        original_tokens: original_tokens,
        is_root: true,
        dependencies,
    });

    // println!("Macro output: {}", &output);

    TokenStream::from_str(&output).unwrap().into()
}


#[proc_macro_attribute]
pub fn pax_file(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    //TODO: use generate_include to watch for changes in specified file, ensuring macro is re-evaluated when file changes
    //let include = generate_include(...);

    //TODO: load specified file contents, hack into `args: proc_macro::TokenStream`, and call `pax(args, input)`
    let _ = args;
    let _ = input;

    input
}


#[proc_macro_attribute]
pub fn pax_on(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let _ = args;
    let _ = input;

    //TODO: register event handler (e.g. PreRender)
    //Handle incremental compilation

    input
}

// Needed because Cargo wouldn't otherwise watch for changes in pax files.
// By include_str!ing the file contents,
// (Trick borrowed from Pest: github.com/pest-parser/pest)
fn generate_include(name: &Ident, path: &str) -> TokenStream {
    let const_name = Ident::new(&format!("_PAX_FILE_{}", name), Span::call_site());
    quote! {
        #[allow(non_upper_case_globals)]
        const #const_name: &'static str = include_str!(#path);
    }
}


//zb lab journal:
//   This file including trick does NOT force the compiler to visit every macro.
//   Instead, it (arguably more elegantly) forces Cargo to dirty-detect changes
//   in the linked pax file by inlining that pax file into the .rs file.
//   This is subtly different vs. our needs.

//   One way to handle this is with an idempotent file registry — the challenge is
//   how do entries get removed from the registry?

//   Another possibility is static analysis

//   Another possibility is a static registry — instead of phoning home over TCP,
//   each macro generates a snippet that registers the pax parsing task via code.
//   When the cartridge is run (with the `designtime` feature), that registry
//   is exposed to the compiler, which is then able to determine which files to parse.
//   This works in tandem with the dirty-watching trick borrowed from Pest —
//   the "static registry" assignment will exist IFF the macro is live.
//
//   Note: possibly problematically, this "dynamic evaluation" of the manifest requires
//         happening BEFORE the second cargo build, meaning the binary is run once (with blanks),
//         evaluated to pass its macro manifest, then patched+recompiled before ACTUALLY running
//   Perhaps this deserves a separate feature vs. `designtime`
//   Alternatively:  is there a way to fundamentally clean this up?

//   Another possibility: keep a manifest manually, e.g. in JSON or XML or YAML

// v0? ----
// Keep a .manifest.json alongside each pax file — the #[pax] macro can write to the companion file
// for each pax file that it finds, and it can encode the relevant information for that file (e.g. component name)
// The compiler can just naively look for all .pax.manifest files in the src/ dir
//
//Along with the "force dirty-watch" trick borrowed from Pest, this technique ensures that .manifest.json can
//stay current with any change.
//
//Sanity check that we can accomplish our goals here
// 1. generate PropertiesCoproduct for subsequent compilation,
// - codegen a PropertiesCoproduct lib.rs+crate that imports the target crate's exported members
// - codegen a "patched" Cargo.toml,
// 2. choose which .pax files to parse, and which ComponentDefinitions to associate a given .pax file with
// - refer to manifests for precise fs paths to .pax files
//
// Limitation: only one Pax component can be registered per file
// Refinement: can store a duplicate structure of .pax.manifest files inside the local .pax directory
//             that is, instead of alongside the source files in userland
// Finally: this could be evolved into an automated, "double pass" compilation, where `pax-compiler` orchestrates
//          fancy metaprogramming and message-passing (thinking: a special feature flag for the first pass, which
//          aggregates data and hands off to the second pass which operates under the `designtime` feature.)
//
// To recap: - during initial & standard compilation, generate .pax/manifests/{mimic structure of src/}file.manifest
//           - before designtime compilation: generate pax-properties-coproduct, Cargo.toml
//           - parse pax files and prepare data for dump
//              (Advantages of waiting until cartridge is running:
//                [will fail parsing more gracefully; will better transition to compiler-side RIL generation])
//           - perform second compilation; load resulting lib into demo chassis
//           - dump parsed data to demo chassis; start running
//             [Refinement: in the future when RIL is generated, this initial dump could be avoided]
//
// twist! might not be able to reliably get FS path from inside macros (see proc_macro_span unstable feature, since 2018)
//    Spitballing a possible approach using multi-stage compilation:
//     - macro generates functions behind a special feature "stage-0" that perform a TCP phone-home:
//          - call file!() at runtime, allowing reliable resolution
//          - pass file path for .pax file
//          - pass struct path (& module? TBD but probably std::module_path) for properties coproduct generation

// 1. compile cartridge with `parser` feature
//  - each #[pax] macro
//  ? how do files get into the tree?  Can we rely on the root file & its imports?
//    note that resolving our deps requires traversing Component templates — this probably means
//    we need to parse the templates *at this phase* so that each macro can 'phone home' for `parser`
//    i.e. unroll dependencies from Pax into Rust so that the compiler can visit all the necessary files
//  - THEN - either by evaling logic as a side-effect of importing (is this possible?) or by
//    conventionally importing a certain-named entity that abides by a certain impl (and calling a known method)
//    then each macro communicates its relevant data: file path, module path, name (and later: methods, properties)

// Then too... if we're going to be parsing the pax tree in order to determine which modules to resolve, maybe we don't need
// a separate manifest-generating step after all?  (Except we still need to generate the PropertiesCoproduct).


