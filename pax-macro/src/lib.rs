extern crate proc_macro;
extern crate proc_macro2;

use std::fs;
use std::str::FromStr;
use std::collections::HashSet;

use proc_macro2::{Ident, Span, TokenStream};
use quote::__private::ext::RepToTokensExt;
use quote::{quote, ToTokens};
use pax_compiler_api::{TemplateArgsMacroPaxPrimitive, TemplateArgsMacroPax, TemplateArgsMacroPaxType, CompileTimePropertyDefinition};

use syn::{parse_macro_input, Data, DeriveInput, Type, Field, Fields, PathArguments, GenericArgument};


#[proc_macro_attribute]
pub fn pax_primitive(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {


    let original_tokens = input.to_string();

    let input_parsed = parse_macro_input!(input as DeriveInput);
    let pascal_identifier = input_parsed.ident.to_string();

    let compile_time_property_definitions = get_compile_time_property_definitions_from_tokens(input_parsed.data);

    let output = pax_compiler_api::press_template_macro_pax_primitive(TemplateArgsMacroPaxPrimitive{
        pascal_identifier,
        original_tokens,
        compile_time_property_definitions,
    });

    TokenStream::from_str(&output).unwrap().into()
}


#[proc_macro_attribute]
pub fn pax_type(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let original_tokens = input.to_string();

    let input = parse_macro_input!(input as DeriveInput);

    let pascal_identifier = input.ident.to_string();

    let output = pax_compiler_api::press_template_macro_pax_type(TemplateArgsMacroPaxType{
        pascal_identifier,
        original_tokens,
    });

    TokenStream::from_str(&output).unwrap().into()

}



/// Determines whether a field is wrapped in Property<...>, returning None if not,
/// and returning the encapsulated type if so.  This heuristic is used to determine
/// whether a declared field should be treated as a Pax Property
fn get_property_wrapped_field(f: &Field) -> Option<Type> {
    let mut ret = None;
    match &f.ty {
        Type::Path(tp) => {
            match tp.qself {
                None => {
                    tp.path.segments.iter().for_each(|ps| {
                        //Only generate parsing logic for types wrapped in `Property<>`
                        if ps.ident.to_string().ends_with("Property") {
                            match &ps.arguments {
                                PathArguments::AngleBracketed(abga) => {
                                    abga.args.iter().for_each(|abgaa| {
                                        match abgaa {
                                            GenericArgument::Type(gat) => {
                                                ret = Some(gat.to_owned());
                                            },
                                            _ => {}
                                        };
                                    })
                                },
                                _ => {}
                            }
                        }
                    });
                },
                _ => {},
            };
        },
        _ => {}
    };
    ret
}

fn recurse_get_scoped_resolvable_types(t: &Type, accum: &mut HashSet<String>) {
    match t {
        Type::Path(tp) => {
            match tp.qself {
                None => {
                    let mut accumulated_scoped_resolvable_type = "".to_string();
                    tp.path.segments.iter().for_each(|ps| {
                        match &ps.arguments {
                            PathArguments::AngleBracketed(abga) => {

                                if accumulated_scoped_resolvable_type.ne("") {
                                    accumulated_scoped_resolvable_type = accumulated_scoped_resolvable_type.clone() + "::"
                                }
                                let ident = ps.ident.to_token_stream().to_string();
                                let turbofish_contents = ps.to_token_stream()
                                    .to_string()
                                    .replacen(&ident, "", 1)
                                    .replace(" ", "");
                                accumulated_scoped_resolvable_type =
                                    accumulated_scoped_resolvable_type.clone() +
                                    &ident +
                                    "::" +
                                    &turbofish_contents;

                                abga.args.iter().for_each(|abgaa| {
                                    match abgaa {
                                        GenericArgument::Type(gat) => {

                                            //break apart, for example, `Vec` from `Vec<(usize, Size)` >
                                            recurse_get_scoped_resolvable_types(gat, accum);
                                        },
                                        //TODO: _might_ need to extract and deal with lifetimes, most notably where the "full string type" is used.
                                        //      May be a non-issue, but this is where that data would need to be extracted.
                                        //      Finally: might want to choose whether to require that any lifetimes used in Pax `Property<...>` are compatible with `'static`

                                        _ => { }
                                    };
                                })
                            },
                            PathArguments::Parenthesized(_) => {unimplemented!("Parenthesized path arguments (for example, Fn types) not yet supported inside Pax `Property<...>`")},
                            PathArguments::None => {
                                //PathSegments without Args are vanilla segments, like
                                //`std` or `collections`.  While visiting path segments, assemble our
                                //accumulated_scoped_resolvable_type
                                if accumulated_scoped_resolvable_type.ne("") {
                                    accumulated_scoped_resolvable_type = accumulated_scoped_resolvable_type.clone() + "::"
                                }
                                accumulated_scoped_resolvable_type = accumulated_scoped_resolvable_type.clone() + &ps.to_token_stream().to_string();
                            }
                            _ => {}
                        }
                    });

                    accum.insert(accumulated_scoped_resolvable_type);

                },
                _ => { unimplemented!("Self-types not yet supported with Pax `Property<...>`")}
            }
        },
        //For example, the contained tuple: `Property<(usize, Vec<String>)>`
        Type::Tuple(t) => {
            t.elems.iter().for_each(|tuple_elem| {
                recurse_get_scoped_resolvable_types(tuple_elem, accum);
            });
        },
        _ => {
            unimplemented!("Unsupported Type::Path {}", t.to_token_stream().to_string());
        }
    }
}

/// From a raw Property inner type (`T<K>` for `Property<T<K>>`):
/// Retrieve a list of "resolvable types"; this is, a sequence of identifers that `rustc` can resolve
/// to a namespace for a `::get_fully_qualified_path` call.
/// For example: `K` and `T::<K>`.  This is used to bridge from static to dynamic analysis, parse-time "reflection,"
/// so that the Pax compiler can resolve fully qualified paths.
fn get_scoped_resolvable_types(t: &Type) -> HashSet<String> {
    let mut accum: HashSet<String> = HashSet::new();
    recurse_get_scoped_resolvable_types(t, &mut accum);
    accum
}

fn get_compile_time_property_definitions_from_tokens(data: Data) -> Vec<CompileTimePropertyDefinition> {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let mut ret = vec![];
                    fields.named.iter().for_each(|f| {
                        let field_name = f.ident.as_ref().unwrap();
                        let field_type = match get_property_wrapped_field(f) {
                            None => { /* noop */ },
                            Some(ty) => {
                                let name = quote!(#ty).to_string().replace(" ", "");

                                let scoped_resolvable_types = get_scoped_resolvable_types(&ty);
                                // let (scoped_resolvable_types_minus_prelude, scoped_resolvable_types_intersect_prelude) =  {
                                //     let mut filt_minus= HashSet::new();
                                //     let mut filt_intersect= HashSet::new();
                                //     scoped_resolvable_types.iter().for_each(|a| {
                                //         if !pax_compiler_api::is_prelude_type(a) {
                                //             filt_minus.insert(a.clone());
                                //         } else {
                                //             filt_intersect.insert(a.clone());
                                //         }
                                //     });
                                //     (filt_minus, filt_intersect)
                                // };
                                ret.push(
                                    CompileTimePropertyDefinition {
                                        full_type_name: name,
                                        field_name: quote!(#field_name).to_string(),
                                        scoped_resolvable_types,
                                    }
                                )
                            }
                        };

                    });
                    ret
                },
                _ => {
                    unimplemented!("Pax may only be attached to `struct`s with named fields");
                }
            }
        },
        _ => {unreachable!("Pax may only be attached to `struct`s")}
    }
}



///TODO: extend `Type`-matching logic to be recursive, and return a flat list of atomic Types
/// (as well as a full string of the type, esp. for Lifetime concerns.


fn pax_internal(args: proc_macro::TokenStream, input: proc_macro::TokenStream, is_root: bool) -> proc_macro::TokenStream {
    let original_tokens = input.to_string();

    let pub_mod_types = "".to_string();

    let input_parsed = parse_macro_input!(input as DeriveInput);
    let pascal_identifier = input_parsed.ident.to_string();

    let compile_time_property_definitions = get_compile_time_property_definitions_from_tokens(input_parsed.data);

    let raw_pax = args.to_string();
    let template_dependencies = pax_compiler_api::parse_pascal_identifiers_from_component_definition_string(&raw_pax);

    let pub_mod_types = "".into(); //TODO: load codegenned types.fragment.rs file.  Might feature-gate an include_str! behind a `cartridge-attached` feature.

    let output = pax_compiler_api::press_template_macro_pax_root(TemplateArgsMacroPax {
        raw_pax,
        pascal_identifier,
        original_tokens,
        is_root,
        template_dependencies,
        compile_time_property_definitions,
        pub_mod_types,
    });

    TokenStream::from_str(&output).unwrap().into()
}


#[proc_macro_attribute]
pub fn pax(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    pax_internal(args, input, false)
}

// Exactly like `#[pax()]`, except specifies that the attached component is intended to be mounted at
// the root of an app-contained cartridge
#[proc_macro_attribute]
pub fn pax_root(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    pax_internal(args, input, true)
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


