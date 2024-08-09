extern crate proc_macro;
extern crate proc_macro2;
mod parsing;
mod templating;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::{fs, path::PathBuf};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};

use syn::punctuated::Punctuated;
use templating::{
    ArgsFullComponent, ArgsPrimitive, ArgsStructOnlyComponent, EnumVariantDefinition,
    InternalDefinitions, StaticPropertyDefinition, TemplateArgsDerivePax,
};

use sailfish::TemplateOnce;

use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, FnArg, GenericArgument, ImplItem,
    ImplItemMethod, ItemFn, ItemImpl, Lit, Meta, PatType, PathArguments, Signature, Token, Type,
};

fn pax_primitive(
    input_parsed: &DeriveInput,
    primitive_instance_import_path: String,
    is_custom_interpolatable: bool,
) -> proc_macro2::TokenStream {
    let _original_tokens = quote! { #input_parsed }.to_string();
    let pascal_identifier = input_parsed.ident.to_string();
    let is_enum = match &input_parsed.data {
        Data::Enum(_) => true,
        _ => false,
    };

    let internal_definitions = get_internal_definitions_from_tokens(&input_parsed.data);

    let output = TemplateArgsDerivePax {
        args_primitive: Some(ArgsPrimitive {
            primitive_instance_import_path,
        }),
        args_struct_only_component: None,
        args_full_component: None,
        internal_definitions,
        pascal_identifier,
        is_custom_interpolatable,
        is_enum,
    }
    .render_once()
    .unwrap()
    .to_string();

    TokenStream::from_str(&output).unwrap().into()
}

fn pax_struct_only_component(
    input_parsed: &DeriveInput,
    is_custom_interpolatable: bool,
) -> proc_macro2::TokenStream {
    let pascal_identifier = input_parsed.ident.to_string();
    let is_enum = match &input_parsed.data {
        Data::Enum(_) => true,
        _ => false,
    };

    let internal_definitions = get_internal_definitions_from_tokens(&input_parsed.data);

    let output = templating::TemplateArgsDerivePax {
        args_full_component: None,
        args_primitive: None,
        args_struct_only_component: Some(ArgsStructOnlyComponent {}),

        pascal_identifier: pascal_identifier.clone(),
        internal_definitions,
        is_custom_interpolatable,
        is_enum,
    }
    .render_once()
    .unwrap()
    .to_string();

    TokenStream::from_str(&output).unwrap().into()
}

/// Returns the type associated with a field, as well as a flag describing whether the property
/// type is wrapped in Property<T>
fn get_field_type(f: &Field) -> Option<(Type, bool)> {
    let mut ret = None;
    if let Type::Path(tp) = &f.ty {
        match tp.qself {
            None => {
                tp.path.segments.iter().for_each(|ps| {
                    //Only generate parsing logic for types wrapped in `Property<>`
                    if ps.ident.to_string().ends_with("Property") {
                        if let PathArguments::AngleBracketed(abga) = &ps.arguments {
                            abga.args.iter().for_each(|abgaa| {
                                if let GenericArgument::Type(gat) = abgaa {
                                    ret = Some((gat.to_owned(), true));
                                }
                            })
                        }
                    }
                });
                if ret.is_none() {
                    //ret is still None, so we will assume this is a simple type and pass it forward
                    ret = Some((f.ty.to_owned(), false));
                }
            }
            _ => {}
        };
    }
    ret
}

/// Break apart a raw Property inner type (`T<K>` for `Property<T<K>>`):
/// into a list of `rustc` resolvable identifiers, possible namespace-nested,
/// which may be appended with `::get_type_id(...)` for dynamic analysis.
/// For example: `K` and `T::<K>`, which become `K::get_type_id(...)` and `T::<K>::get_type_id(...)`.
/// This is used to bridge from static to dynamic analysis, parse-time "reflection,"
/// so that the Pax compiler can resolve fully qualified paths.
fn get_scoped_resolvable_types(t: &Type) -> (Vec<String>, String) {
    let mut accum: Vec<String> = vec![];
    recurse_get_scoped_resolvable_types(t, &mut accum);

    //the recursion above was post-order, so we will assume
    //the final element is root
    let root_scoped_resolvable_type = accum.get(accum.len() - 1).unwrap().clone();

    (accum, root_scoped_resolvable_type)
}

fn recurse_get_scoped_resolvable_types(t: &Type, accum: &mut Vec<String>) {
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
                        }
                    });

                    accum.push(accumulated_scoped_resolvable_type);
                }
                _ => {
                    unimplemented!("Self-types not yet supported with Pax `Property<...>`")
                }
            }
        }
        //For example, the contained tuple: `Property<(usize, Vec<String>)>`
        Type::Tuple(t) => {
            t.elems.iter().for_each(|tuple_elem| {
                recurse_get_scoped_resolvable_types(tuple_elem, accum);
            });
        }
        _ => {
            unimplemented!("Unsupported Type::Path {}", t.to_token_stream().to_string());
        }
    }
}

fn index_to_ascii_lowercase(index: usize) -> char {
    (b'a' + (index as u8)) as char
}

fn get_internal_definitions_from_tokens(data: &Data) -> InternalDefinitions {
    let ret = match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let mut spds = vec![];
                    fields.named.iter().for_each(|f| {
                        let field_name = f.ident.as_ref().unwrap();
                        let _field_type = match get_field_type(f) {
                            None => { /* noop */ }
                            Some(ty) => {
                                let type_name = quote!(#(ty.0)).to_string().replace(" ", "");

                                let (scoped_resolvable_types, root_scoped_resolvable_type) =
                                    get_scoped_resolvable_types(&ty.0);
                                let pascal_identifier =
                                    type_name.split("::").last().unwrap().to_string();
                                spds.push(StaticPropertyDefinition {
                                    original_type: type_name,
                                    field_name: quote!(#field_name).to_string(),
                                    scoped_resolvable_types,
                                    root_scoped_resolvable_type,
                                    pascal_identifier,
                                    is_property_wrapped: ty.1,
                                    is_enum: false,
                                })
                            }
                        };
                    });
                    InternalDefinitions::Struct(spds)
                }
                _ => {
                    unimplemented!("Pax may only be attached to `struct`s with named fields");
                }
            }
        }
        Data::Enum(ref data) => {
            let mut evds = vec![];
            data.variants.iter().for_each(|variant| {
                let variant_name = variant.ident.to_string();
                let mut variant_fields = vec![];
                for (i, f) in variant.fields.iter().enumerate() {
                    if let Some(ty) = get_field_type(f) {
                        let original_type = quote!(#(ty.0)).to_string().replace(" ", "");
                        let (scoped_resolvable_types, root_scoped_resolvable_type) =
                            get_scoped_resolvable_types(&ty.0);
                        let pascal_identifier =
                            original_type.split("::").last().unwrap().to_string();
                        variant_fields.push(StaticPropertyDefinition {
                            original_type,
                            field_name: index_to_ascii_lowercase(i).to_string(),
                            scoped_resolvable_types,
                            root_scoped_resolvable_type,
                            pascal_identifier,
                            is_property_wrapped: ty.1,
                            is_enum: true,
                        })
                    }
                }
                evds.push(EnumVariantDefinition {
                    variant_name,
                    variant_fields,
                });
            });

            InternalDefinitions::Enum(evds)
        }

        _ => {
            unreachable!("Pax may only be attached to `struct`s")
        }
    };

    ret
}

fn pax_full_component(
    raw_pax: String,
    input_parsed: &DeriveInput,
    is_main_component: bool,
    include_fix: Option<TokenStream>,
    is_custom_interpolatable: bool,
    associated_pax_file_path: Option<PathBuf>,
) -> proc_macro2::TokenStream {
    let pascal_identifier = input_parsed.ident.to_string();
    let is_enum = match &input_parsed.data {
        Data::Enum(_) => true,
        _ => false,
    };

    let internal_definitions = get_internal_definitions_from_tokens(&input_parsed.data);

    let mut template_dependencies = vec![];
    let mut error_message: Option<String> = None;

    match parsing::parse_pascal_identifiers_from_component_definition_string(&raw_pax) {
        Ok(deps) => {
            template_dependencies = deps;
        }
        Err(err) => {
            error_message = Some(err);
        }
    }

    // Add BlankComponent to template_dependencies so it's guaranteed to be included in the PaxManifest
    if is_main_component {
        template_dependencies.push("BlankComponent".to_string());
    }

    let pax_dir: Option<&'static str> = option_env!("PAX_DIR");
    let cartridge_snippet = if let Some(pax_dir) = pax_dir {
        let cartridge_path = std::path::Path::new(pax_dir).join("cartridge.partial.rs");
        fs::read_to_string(&cartridge_path).unwrap()
    } else {
        "".to_string()
    };
    let output = TemplateArgsDerivePax {
        args_primitive: None,
        args_struct_only_component: None,
        args_full_component: Some(ArgsFullComponent {
            is_main_component,
            raw_pax,
            template_dependencies,
            cartridge_snippet,
            associated_pax_file_path,
            error_message,
        }),
        pascal_identifier,
        internal_definitions,
        is_custom_interpolatable,
        is_enum,
    }
    .render_once()
    .unwrap()
    .to_string();

    let ret = TokenStream::from_str(&output).unwrap().into();
    if !include_fix.is_none() {
        quote! {
            #include_fix
            #ret
        }
    } else {
        ret
    }
    .into()
}

struct Config {
    is_main_component: bool,
    file_path: Option<String>,
    inlined_contents: Option<String>,
    custom_values: Option<Vec<String>>,
    primitive_instance_import_path: Option<String>,
    is_primitive: bool,
    has_helpers: bool,
}

fn parse_config(attrs: &mut Vec<syn::Attribute>) -> Config {
    let mut config = Config {
        is_main_component: false,
        file_path: None,
        inlined_contents: None,
        custom_values: None,
        primitive_instance_import_path: None,
        is_primitive: false,
        has_helpers: false,
    };

    // iterate through `derive macro helper attributes` to gather config & args
    // remove the ones we use, don't remove the ones we don't
    attrs.retain(|attr| {
        match attr.path.get_ident() {
            Some(s) if s == "file" => {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    if let Some(nested_meta) = meta_list.nested.first() {
                        if let syn::NestedMeta::Lit(Lit::Str(file_str)) = nested_meta {
                            config.file_path = Some(file_str.value());
                            return false;
                        }
                    }
                }
            }
            Some(s) if s == "primitive" => {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    if let Some(nested_meta) = meta_list.nested.first() {
                        if let syn::NestedMeta::Lit(Lit::Str(file_str)) = nested_meta {
                            config.primitive_instance_import_path = Some(file_str.value());
                            config.is_primitive = true;
                            return false;
                        }
                    }
                }
            }
            Some(s) if s == "inlined" => {
                let tokens = attr.tokens.clone();
                let mut content = proc_macro2::TokenStream::new();

                for token in tokens {
                    if let proc_macro2::TokenTree::Group(group) = token {
                        if group.delimiter() == proc_macro2::Delimiter::Parenthesis {
                            content.extend(group.stream());
                        }
                    }
                }

                if !content.is_empty() {
                    config.inlined_contents = Some(content.to_string());
                    return false;
                }
            }
            Some(s) if s == "has_helpers" => {
                config.has_helpers = true;
                return false;
            }
            _ => {
                if let Ok(Meta::Path(path)) = attr.parse_meta() {
                    if path.is_ident("main") {
                        config.is_main_component = true;
                        return false;
                    }
                } else if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
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
                        config.custom_values = Some(values);
                        return false;
                    }
                }
            }
        }
        true
    });

    config
}

fn validate_config(
    input: &syn::DeriveInput,
    config: &Config,
) -> Result<(), proc_macro::TokenStream> {
    if config.file_path.is_some() && config.inlined_contents.is_some() {
        return Err(syn::Error::new_spanned(
            input.ident.clone(),
            "`#[file(...)]` and `#[inlined(...)]` attributes cannot be used together",
        )
        .to_compile_error()
        .into());
    }
    if config.file_path.is_none() && config.inlined_contents.is_none() && config.is_main_component {
        return Err(syn::Error::new_spanned(
            input.ident.clone(),
            "Main (application-root) components must specify either a Pax file or inlined Pax content, e.g. #[file(\"some-file.pax\")] or #[inlined(<SomePax />)]",
        )
        .to_compile_error()
        .into());
    }
    if config.is_primitive && (config.file_path.is_some() || config.inlined_contents.is_some()) {
        const ERR: &str = "Primitives cannot have attached templates. Instead, specify a fully qualified Rust import path pointing to the `impl RenderNode` struct for this primitive.";
        return Err(syn::Error::new_spanned(input.ident.clone(), ERR)
            .to_compile_error()
            .into());
    }
    Ok(())
}

#[proc_macro_attribute]
pub fn pax(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);

    let pascal_identifier = input.ident.to_string();
    let config = parse_config(&mut input.attrs);
    validate_config(&input, &config).unwrap();

    let mut trait_impls = vec!["Clone", "Default", "Serialize", "Deserialize", "Debug"];

    let mut is_custom_interpolatable = false;

    //wipe out the above derives if `#[custom(...)]` attrs are set
    if let Some(custom) = config.custom_values {
        let custom_str: Vec<&str> = custom.iter().map(String::as_str).collect();
        trait_impls.retain(|v| !custom_str.contains(v));

        if custom.contains(&"Interpolatable".to_string()) {
            is_custom_interpolatable = true;
        }
    }

    let is_pax_file = config.file_path.is_some();
    let is_pax_inlined = config.inlined_contents.is_some();

    let appended_tokens = if is_pax_file {
        let filename = config.file_path.unwrap();
        let current_dir = std::env::current_dir().expect("Unable to get current directory");
        let path = current_dir.join("src").join(&filename);
        // generate_include to watch for changes in specified file, ensuring macro is re-evaluated when file changes
        let name = Ident::new("PaxFile", Span::call_site());

        let name = Ident::new(&pascal_identifier, Span::call_site());
        let include_fix = generate_include(&name, &path);
        let associated_pax_file = Some(path.clone());
        let file = File::open(path);
        let mut content = String::new();
        let _ = file.unwrap().read_to_string(&mut content);
        pax_full_component(
            content,
            &input,
            config.is_main_component,
            Some(include_fix),
            is_custom_interpolatable,
            associated_pax_file,
        )
    } else if is_pax_inlined {
        let contents = config.inlined_contents.unwrap();

        pax_full_component(
            contents.to_owned(),
            &input,
            config.is_main_component,
            None,
            is_custom_interpolatable,
            None,
        )
    } else if config.is_primitive {
        pax_primitive(
            &input,
            config.primitive_instance_import_path.unwrap(),
            is_custom_interpolatable,
        )
    } else {
        pax_struct_only_component(&input, is_custom_interpolatable)
    };

    let derives: proc_macro2::TokenStream = trait_impls
        .into_iter()
        .flat_map(|ident| {
            let syn_ident = syn::Ident::new(ident, Span::call_site());
            if ["Serialize", "Deserialize"].contains(&ident) {
                // fully qualify serde dependencies
                quote! {pax_engine::serde::#syn_ident,}
            } else {
                quote! {#syn_ident,}
            }
        })
        .collect();

    let ident = &input.ident;
    let helper_functions_impl = if !config.has_helpers {
        quote! {
            impl pax_engine::api::HelperFunctions for #ident {
                fn register_all_functions() {
                    // Do nothing
                }
            }
        }
    } else {
        quote! {}
    };

    let output = quote! {
        // TODO make this value represented in PaxValue instead (map of properties), and impl to/from that value
        impl pax_engine::api::ImplToFromPaxAny for #ident {}

        #[derive(#derives)]
        #[serde(crate = "pax_engine::serde")]
        #input
        #appended_tokens

        #helper_functions_impl
    };
    output.into()
}

// Needed because Cargo wouldn't otherwise watch for changes in pax files.
// By include_str!ing the file contents,
// (Trick borrowed from Pest: github.com/pest-parser/pest)
fn generate_include(name: &Ident, path: &PathBuf) -> TokenStream {
    let const_name = Ident::new(&format!("_PAX_FILE_{}", name), Span::call_site());
    let path_str = path.to_str().expect("expected non-unicode path");
    quote! {
        #[allow(non_upper_case_globals)]
        const #const_name: &'static str = include_str!(#path_str);
    }
}
#[proc_macro_attribute]
pub fn helpers(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let struct_name = &input.self_ty;

    let mut register_functions = vec![];

    for item in input.items.iter() {
        if let ImplItem::Method(method) = item {
            let func_name = &method.sig.ident;

            // Make sure it's associated function (doesn't use `self`)
            if method
                .sig
                .inputs
                .iter()
                .any(|arg| matches!(arg, FnArg::Receiver(_)))
            {
                return syn::Error::new_spanned(
                    method.sig.clone(),
                    "Helpers macro can only be used on associated functions (methods that don't take self)",
                )
                .to_compile_error()
                .into();
            }

            let arg_count = method.sig.inputs.len();
            let param_checks = generate_param_checks(&method.sig.inputs);
            let func_call = generate_function_call(&method.sig, struct_name);

            register_functions.push(quote! {
                register_function(
                    stringify!(#struct_name).to_string(),
                    stringify!(#func_name).to_string(),
                    Arc::new(move |args: Vec<PaxValue>| -> Result<PaxValue, String> {
                        if args.len() != #arg_count {
                            return Err(format!("Expected {} arguments for function {}", #arg_count, stringify!(#func_name)));
                        }
                        #param_checks
                        #func_call
                    })
                );
            });
        }
    }

    let expanded = quote! {
        #input

        impl pax_engine::api::HelperFunctions for #struct_name {
            fn register_all_functions() {
                use std::sync::Arc;
                use pax_engine::api::{PaxValue, register_function};
                #(#register_functions)*
            }
        }
    };

    expanded.into()
}

fn generate_param_checks(inputs: &Punctuated<FnArg, Token![,]>) -> proc_macro2::TokenStream {
    let checks = inputs.iter().enumerate().filter_map(|(i, arg)| {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            let ty_string = quote!(#ty).to_string();
            let arg_name = format_ident!("arg_{}", i);
            Some(quote! {
                let #arg_name = <#ty as pax_engine::api::CoercionRules>::try_coerce(args[#i].clone())
                    .map_err(|_| format!("Failed to coerce argument {} to {}", #i, #ty_string))?;
            })
        } else {
            None
        }
    });

    quote! { #(#checks)* }
}

fn generate_function_call(sig: &Signature, struct_name: &Box<Type>) -> proc_macro2::TokenStream {
    let func_name = &sig.ident;
    let args = sig.inputs.iter().enumerate().filter_map(|(i, arg)| {
        if let FnArg::Typed(_) = arg {
            let arg_name = format_ident!("arg_{}", i);
            Some(quote! { #arg_name })
        } else {
            None
        }
    });

    quote! {
        let result = #struct_name::#func_name(#(#args),*);
        Ok(<_ as pax_engine::api::ToPaxValue>::to_pax_value(result))
    }
}
