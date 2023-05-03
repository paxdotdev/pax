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

use templating::{TemplateArgsMacroPaxPrimitive, TemplateArgsMacroPax, TemplateArgsMacroPaxType, StaticPropertyDefinition};

use sailfish::TemplateOnce;

use syn::{parse_macro_input, Data, DeriveInput, Type, Field, Fields, PathArguments, GenericArgument, Attribute, Meta, NestedMeta, parse2, MetaList, Lit};

#[proc_macro_attribute]
pub fn pax_primitive(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let original_tokens = input.to_string();

    let input_parsed = parse_macro_input!(input as DeriveInput);
    let pascal_identifier = input_parsed.ident.to_string();

    let static_property_definitions = get_static_property_definitions_from_tokens(input_parsed.data);

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
        static_property_definitions,
        primitive_instance_import_path,
    }.render_once().unwrap().to_string();

    TokenStream::from_str(&output).unwrap().into()
}

// Helper function to extract the values inside the `custom` attribute.
// fn extract_custom_attr(attr: &Attribute) -> Option<Vec<String>> {
//     if attr.path.is_ident("custom") {
//         let meta = attr.parse_meta().ok()?;
//         let meta_list = match meta {
//             Meta::List(list) => list,
//             _ => return None,
//         };
//
//         let attr_values = meta_list
//             .nested
//             .into_iter()
//             .filter_map(|meta| match meta {
//                 NestedMeta::Meta(Meta::Path(path)) => {
//                     path.get_ident().map(|ident| ident.to_string())
//                 }
//                 _ => None,
//             })
//             .collect();
//
//         Some(attr_values)
//     } else {
//         None
//     }
// }

// Helper function to extract the values inside the `custom` attribute.
// fn extract_custom_attr(attr: &Meta) -> Option<Vec<String>> {
//     let meta_list = match attr {
//         Meta::List(list) => list,
//         _ => return None,
//     };
//
//     let custom_attr = meta_list
//         .nested
//         .iter()
//         .find(|nested_meta| match nested_meta {
//             NestedMeta::Meta(Meta::NameValue(name_value)) => {
//                 name_value.path.is_ident("custom")
//             }
//             _ => false,
//         })?;
//
//     let custom_values = match custom_attr {
//         NestedMeta::Meta(Meta::NameValue(name_value)) => match &name_value.lit {
//             syn::Lit::Str(lit_str) => {
//                 lit_str.value().split(',').map(|s| s.trim().to_string()).collect()
//             }
//             _ => return None,
//         },
//         _ => return None,
//     };
//
//     Some(custom_values)
// }

// Helper function to extract the values inside the `custom` attribute.
fn extract_custom_attr(attr: &MetaList) -> Option<Vec<String>> {
    let custom_attr = attr
        .nested
        .iter()
        .find(|nested_meta| match nested_meta {
            NestedMeta::Meta(Meta::NameValue(name_value)) => {
                name_value.path.is_ident("custom")
            }
            _ => false,
        })?;

    let custom_values = match custom_attr {
        NestedMeta::Meta(Meta::NameValue(name_value)) => match &name_value.lit {
            syn::Lit::Str(lit_str) => {
                lit_str.value().split(',').map(|s| s.trim().to_string()).collect()
            }
            _ => return None,
        },
        _ => return None,
    };

    Some(custom_values)
}


#[proc_macro_attribute]
pub fn pax_type(attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let original_tokens = input.to_string();

    // // Parse pax_type attribute TokenStream as a Meta.
    // let pax_type_attr: Meta = syn::parse(attr).expect("Failed to parse pax_type attribute");
    //
    // // Initialize an empty Vec to store custom attribute values.
    // let mut custom_attrs = Vec::new();
    //
    // // Extract the values inside the `custom` attribute if provided.
    // if let Some(attr_values) = extract_custom_attr(&pax_type_attr) {
    //     custom_attrs.extend(attr_values);
    // }


    // Parse pax_type attribute TokenStream as a MetaList.
    let pax_type_attr: MetaList = syn::parse(attr).expect("Failed to parse pax_type attribute");

    // Initialize an empty Vec to store custom attribute values.
    let mut custom_attrs = Vec::new();

    // Extract the values inside the `custom` attribute if provided.
    if let Some(attr_values) = extract_custom_attr(&pax_type_attr) {
        custom_attrs.extend(attr_values);
    }






    let input = parse_macro_input!(input as DeriveInput);

    let pascal_identifier = input.ident.to_string();

    let static_property_definitions = get_static_property_definitions_from_tokens(input.data);

    let type_dependencies= static_property_definitions.iter().map(|spd|{
        spd.original_type.clone()
    }).collect();

    let output = templating::TemplateArgsMacroPaxType{
        pascal_identifier: pascal_identifier.clone(),
        original_tokens,
        static_property_definitions,
        type_dependencies,
        should_derive_default: !custom_attrs.contains(&"Default".to_string()),
        should_derive_clone: !custom_attrs.contains(&"Clone".to_string()),
    }.render_once().unwrap().to_string();
    
    // fs::write(format!("/Users/zack/debug/out-{}.txt", &pascal_identifier), &output);

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

fn get_static_property_definitions_from_tokens(data: Data) -> Vec<StaticPropertyDefinition> {
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
                                    StaticPropertyDefinition {
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
        Data::Enum(ref data) => {
            let mut ret = vec![];
            data.variants.iter().for_each(|variant| {
                let variant_name = &variant.ident;

                variant.fields.iter().for_each(|f| {
                    if let Some(ty) = get_property_wrapped_field(f) {
                        let original_type = quote!(#ty).to_string().replace(" ", "");
                        let scoped_resolvable_types = get_scoped_resolvable_types(&ty);
                        ret.push(
                            StaticPropertyDefinition {
                                original_type,
                                field_name: quote!(#variant_name).to_string(),
                                scoped_resolvable_types,
                            }
                        )
                    }
                })


            });
            ret
        }
        _ => {unreachable!("Pax may only be attached to `struct`s")}
    }
}


fn pax_internal(args: proc_macro::TokenStream, input: proc_macro::TokenStream, is_root: bool, include_fix : Option<TokenStream>) -> proc_macro::TokenStream {
    let original_tokens = input.to_string();

    let input_parsed = parse_macro_input!(input as DeriveInput);
    let pascal_identifier = input_parsed.ident.to_string();

    let static_property_definitions = get_static_property_definitions_from_tokens(input_parsed.data);
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
        static_property_definitions,
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

#[proc_macro_derive(Pax, attributes(root, file, inline, custom))]
pub fn pax_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let attrs = &input.attrs;

    let mut is_root = false;
    let mut file_path: Option<String> = None;
    let mut inline_contents: Option<String> = None;
    let mut custom_values: Option<Vec<String>> = None;

    // Handle `derive macro helper attributes` — https://doc.rust-lang.org/reference/procedural-macros.html#derive-macro-helper-attributes
    for attr in attrs {
        match attr.parse_meta() {
            Ok(Meta::Path(path)) => {
                if path.is_ident("root") {
                    is_root = true;
                }
            }
            Ok(Meta::NameValue(name_value)) => {
                if name_value.path.is_ident("file") {
                    if let Lit::Str(file_str) = name_value.lit {
                        file_path = Some(file_str.value());
                    }
                } else if name_value.path.is_ident("inline") {
                    if let Lit::Str(inline_str) = name_value.lit {
                        inline_contents = Some(inline_str.value());
                    }
                }
            }
            Ok(Meta::List(meta_list)) => {
                if meta_list.path.is_ident("custom") {
                    let values: Vec<String> = meta_list
                        .nested
                        .into_iter()
                        .filter_map(|nested_meta| {
                            if let syn::NestedMeta::Meta(Meta::Path(path)) = nested_meta {
                                path.get_ident().map(|ident| ident.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    custom_values = Some(values);
                }
            }
            _ => {}
        }
    }

    //Validation
    if let (Some(_), Some(_)) = (file_path.as_ref(), inline_contents.as_ref()) {
        return syn::Error::new_spanned(input.ident, "`#[file(...)]` and `#[inline(...)]` attributes cannot be used together")
            .to_compile_error()
            .into();
    }
    if let (None, None) = (file_path.as_ref(), inline_contents.as_ref()) {
        if is_root {
            return syn::Error::new_spanned(input.ident, "Root components must specify either a Pax file or inline Pax content, e.g. #[file=\"some-file.pax\"] or #[inline=(<SomePax />)]")
                .to_compile_error()
                .into();
        }
    }

    // Implement Clone
    let clone_impl = match &input.data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields_named) => {
                    let field_clones = fields_named.named.iter().map(|f| {
                        let name = &f.ident;
                        quote! { #name: self.#name.clone() }
                    });

                    quote! {
                    impl #impl_generics Clone for #name #ty_generics #where_clause {
                        fn clone(&self) -> Self {
                            Self {
                                #(#field_clones,)*
                            }
                        }
                    }
                }
                }
                Fields::Unnamed(_) | Fields::Unit => {
                    quote! {
                    impl #impl_generics Clone for #name #ty_generics #where_clause {
                        fn clone(&self) -> Self {
                            Self::default()
                        }
                    }
                }
                }
            }
        }
        Data::Enum(_) | Data::Union(_) => {
            quote! {
                compile_error!("Pax derive can only be used with structs");
            }
        }
    };

    // Implement Default
    let mut default_impl = match &input.data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields_named) => {
                    let field_defaults = fields_named.named.iter().map(|f| {
                        let name = &f.ident;
                        quote! { #name: Default::default() }
                    });

                    quote! {
                        impl #impl_generics Default for #name #ty_generics #where_clause {
                            fn default() -> Self {
                                Self {
                                    #(#field_defaults,)*
                                }
                            }
                        }
                    }
                }
                Fields::Unnamed(_) | Fields::Unit => {
                    quote! {
                        impl #impl_generics Default for #name #ty_generics #where_clause {
                            fn default() -> Self {
                                Self::default()
                            }
                        }
                    }
                }
            }
        }
        Data::Enum(_) | Data::Union(_) => {
            quote! {
                compile_error!("Pax derive can only be used with structs");
            }
        }
    };

    //wipe out our derives if `#[custom(...)]` attrs are set
    if let Some(custom) = custom_values {
        if custom.contains(&"Default".to_string()) {
            default_impl = quote! {};
        }
        if custom.contains(&"Clone".to_string()) {
            clone_impl = quote! {};
        }
    }

    let is_pax_type = matches!(file_path, None) && matches!(inline_contents, None);
    let is_pax_inline = matches!(inline_contents, Some(_));

    let appended_tokens = if is_pax_type {
        // an empty struct is treated as a pax_type, giving it special
        // handling for certain codegen concerns, but not treating
        // the struct as a full-blown component definition
        handle_type()
    } else if is_pax_type {
        // pax_inline

    } else {
        // pax_file
    };


    let output = quote! {
        #appended_tokens
        #clone_impl
        #default_impl
    };

    output.into()
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

