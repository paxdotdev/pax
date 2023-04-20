extern crate proc_macro;
extern crate proc_macro2;

mod templating;
mod parsing;
use std::io::Read;
use std::fs;
use std::str::FromStr;
use std::collections::HashSet;
use std::env::current_dir;
use std::fs::File;
use std::convert::TryFrom;
use std::path::Path;
use litrs::StringLit;

use proc_macro2::{Ident, Span, TokenStream};
use quote::__private::ext::RepToTokensExt;
use quote::{quote, ToTokens};

use templating::{TemplateArgsMacroPaxPrimitive, TemplateArgsMacroPax, TemplateArgsMacroPaxType, CompileTimePropertyDefinition};

use sailfish::TemplateOnce;

use syn::{parse_macro_input, Data, DeriveInput, Type, Field, Fields, PathArguments, GenericArgument};

#[proc_macro_attribute]
pub fn pax_primitive(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let original_tokens = input.to_string();

    let input_parsed = parse_macro_input!(input as DeriveInput);
    let pascal_identifier = input_parsed.ident.to_string();

    let compile_time_property_definitions = get_compile_time_property_definitions_from_tokens(input_parsed.data);

    // for a macro invocation like the following:
    // #[pax_primitive("./pax-std-primitives",  "pax_std_primitives::GroupInstance")]
    //                                           ^
    //                                           |
    // the second argument, above, is our `primitive_instance_import_path`.
    // Note that these tokens are already parsed by rustc, thus the symbols come with spaces injected in between tokens
    let primitive_instance_import_path= args.to_string().split(",").last().unwrap().to_string().replace(" ", "");

    let output = TemplateArgsMacroPaxPrimitive{
        pascal_identifier,
        original_tokens,
        compile_time_property_definitions,
        primitive_instance_import_path,
    }.render_once().unwrap().to_string();

    TokenStream::from_str(&output).unwrap().into()
}

#[proc_macro_attribute]
pub fn pax_type(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let original_tokens = input.to_string();

    let input = parse_macro_input!(input as DeriveInput);

    let pascal_identifier = input.ident.to_string();

    let output = templating::TemplateArgsMacroPaxType{
        pascal_identifier: pascal_identifier.clone(),
        original_tokens,
    }.render_once().unwrap().to_string();
    
    fs::write(format!("/Users/zack/debug/out-{}.txt", &pascal_identifier), &output);

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
                                            _ => {/* lifetimes and more */}
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


/// Break apart a raw Property inner type (`T<K>` for `Property<T<K>>`):
/// into a list of `rustc` resolvable identifiers, possible namespace-nested,
/// which may be appended with `::get_fully_qualified_path(...)` for dynamic analysis.
/// For example: `K` and `T::<K>`, which become `K::get_fully_qualified_path(...)` and `T::<K>::get_fully_qualified_path(...)`.
/// This is used to bridge from static to dynamic analysis, parse-time "reflection,"
/// so that the Pax compiler can resolve fully qualified paths.
fn get_scoped_resolvable_types(t: &Type) -> HashSet<String> {
    let mut accum: HashSet<String> = HashSet::new();
    recurse_get_scoped_resolvable_types(t, &mut accum);
    accum
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
                                        //FUTURE: _might_ need to extract and deal with lifetimes, most notably where the "full string type" is used.
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

                                ret.push(
                                    CompileTimePropertyDefinition {
                                        original_type: name,
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


fn pax_internal(args: proc_macro::TokenStream, input: proc_macro::TokenStream, is_root: bool, include_fix : Option<TokenStream>) -> proc_macro::TokenStream {
    let original_tokens = input.to_string();

    let input_parsed = parse_macro_input!(input as DeriveInput);
    let pascal_identifier = input_parsed.ident.to_string();

    let compile_time_property_definitions = get_compile_time_property_definitions_from_tokens(input_parsed.data);
    let raw_pax = args.to_string();
    let template_dependencies = parsing::parse_pascal_identifiers_from_component_definition_string(&raw_pax);

    // std::time::SystemTime::now().elapsed().unwrap().subsec_nanos()

    // Load reexports.partial.rs if PAX_DIR is set
    let pax_dir: Option<&'static str> = option_env!("PAX_DIR");
    let reexports_snippet = if let Some(pax_dir) = pax_dir {
        let reexports_path = std::path::Path::new(pax_dir).join("reexports.partial.rs");
        fs::read_to_string(&reexports_path).unwrap()
    } else {
        "".to_string()
    };

    let output = TemplateArgsMacroPax {
        raw_pax,
        pascal_identifier,
        original_tokens,
        is_root,
        template_dependencies,
        compile_time_property_definitions,
        reexports_snippet
    }.render_once().unwrap().to_string();

    let ret = TokenStream::from_str(&output).unwrap().into();
    if !include_fix.is_none(){
         quote!{
            #include_fix
            #ret
        }
    }else {
        ret
    }.into()
}

#[proc_macro_attribute]
pub fn pax(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    pax_internal(args, input, false, None)
}

#[proc_macro_attribute]
pub fn pax_app(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    pax_internal(args, input, true, None)
}

#[proc_macro_attribute]
pub fn pax_file(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path_arg = args.clone().into_iter().collect::<Vec<_>>();
    if path_arg.len() != 1 {
        let msg = format!("expected single string, got {}", path_arg.len());
        return quote! { compile_error!(#msg) }.into();
    }
    let path_string = match StringLit::try_from(&path_arg[0]) {
        // Error if the token is not a string literal
        Err(e) => return e.to_compile_error(),
        Ok(lit) => lit,
    };
    let filename = path_string.value();
    let current_dir = std::env::current_dir().expect("Unable to get current directory");
    let path = current_dir.join(Path::new("src").join(Path::new(filename)));

    // generate_include to watch for changes in specified file, ensuring macro is re-evaluated when file changes
    let name = Ident::new("PaxFile", Span::call_site());
    let include_fix = generate_include(&name,path.clone().to_str().unwrap());

    let mut file = File::open(path);
    let mut content = String::new();
    let _ = file.unwrap().read_to_string(&mut content);
    let stream: proc_macro::TokenStream = content.parse().unwrap();
    pax_internal(stream, input, false, Some(include_fix))
}

#[proc_macro_attribute]
pub fn pax_on(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let _ = args;
    let _ = input;

    // unimplemented!("pax_on not yet supported");
    //TODO: register event handler (e.g. PreRender)
    //Handle incremental compilation

    input
}


#[proc_macro_attribute]
pub fn pax_const(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let _ = args;
    let _ = input;

    // unimplemented!("pax_const not yet supported");
    //TODO: expose reference to assoc. constant through reexports; support scope resolution for consts for expressions
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

