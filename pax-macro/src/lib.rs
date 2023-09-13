extern crate proc_macro;
extern crate proc_macro2;

mod templating;
mod parsing;
use std::io::Read;
use std::fs;
use std::str::FromStr;

use std::fs::File;

use std::path::Path;


use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};

use templating::{ArgsPrimitive, ArgsFullComponent, ArgsStructOnlyComponent, TemplateArgsDerivePax, StaticPropertyDefinition};

use sailfish::TemplateOnce;

use syn::{parse_macro_input, Data, DeriveInput, Type, Field, Fields, PathArguments, GenericArgument, Meta, Lit};

fn pax_primitive(input_parsed: DeriveInput, primitive_instance_import_path: String, include_imports: bool, is_custom_interpolatable: bool,) -> proc_macro2::TokenStream {
    let _original_tokens = quote! { #input_parsed }.to_string();
    let pascal_identifier = input_parsed.ident.to_string();

    let static_property_definitions = get_static_property_definitions_from_tokens(input_parsed.data);

    let output = TemplateArgsDerivePax {
        args_primitive: Some(ArgsPrimitive {
            primitive_instance_import_path,
        }),
        args_struct_only_component: None,
        args_full_component: None,
        static_property_definitions,
        pascal_identifier,
        include_imports,
        is_custom_interpolatable,
    }.render_once().unwrap().to_string();

    TokenStream::from_str(&output).unwrap().into()
}

fn pax_struct_only_component(input_parsed: DeriveInput, include_imports: bool, is_custom_interpolatable: bool) -> proc_macro2::TokenStream {


    let pascal_identifier = input_parsed.ident.to_string();

    let static_property_definitions = get_static_property_definitions_from_tokens(input_parsed.data);

    let output = templating::TemplateArgsDerivePax{
        args_full_component: None,
        args_primitive: None,
        args_struct_only_component: Some(ArgsStructOnlyComponent {}),

        pascal_identifier: pascal_identifier.clone(),
        static_property_definitions,
        include_imports,
        is_custom_interpolatable,
    }.render_once().unwrap().to_string();

    TokenStream::from_str(&output).unwrap().into()
}

/// Returns the type associated with a field, as well as a flag describing whether the property
/// type is wrapped in Property<T>
fn get_field_type(f: &Field) -> Option<(Type, bool)> {
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
                                                ret = Some((gat.to_owned(), true));
                                            },
                                            _ => {/* lifetimes and more */}
                                        };
                                    })
                                },
                                _ => {}
                            }
                        }
                    });

                    if ret.is_none() {
                        //ret is still None, so we will assume this is a simple type and pass it forward
                        ret = Some((f.ty.to_owned(), false));
                    }
                },
                _ => {},
            };
        },
        _ => {},
    };
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
    let ret = match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let mut ret = vec![];
                    fields.named.iter().for_each(|f| {
                        let field_name = f.ident.as_ref().unwrap();
                        let _field_type = match get_field_type(f) {
                            None => { /* noop */ },
                            Some(ty) => {
                                let type_name = quote!(#(ty.0)).to_string().replace(" ", "");

                                let (scoped_resolvable_types, root_scoped_resolvable_type) = get_scoped_resolvable_types(&ty.0);

                                let pascal_identifier = type_name.split("::").last().unwrap().to_string();
                                ret.push(
                                    StaticPropertyDefinition {
                                        original_type: type_name,
                                        field_name: quote!(#field_name).to_string(),
                                        scoped_resolvable_types,
                                        root_scoped_resolvable_type,
                                        pascal_identifier,
                                        is_property_wrapped: ty.1
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
                    if let Some(ty) = get_field_type(f) {
                        let original_type = quote!(#(ty.0)).to_string().replace(" ", "");
                        let (scoped_resolvable_types, root_scoped_resolvable_type) = get_scoped_resolvable_types(&ty.0);
                        let pascal_identifier = original_type.split("::").last().unwrap().to_string();
                        ret.push(
                            StaticPropertyDefinition {
                                original_type,
                                field_name: quote!(#variant_name).to_string(),
                                scoped_resolvable_types,
                                root_scoped_resolvable_type,
                                pascal_identifier,
                                is_property_wrapped: ty.1,
                            }
                        )
                    }
                })


            });

            ret
        }

        _ => {unreachable!("Pax may only be attached to `struct`s")}
    };

    ret
}

fn pax_full_component(raw_pax: String, input_parsed: DeriveInput, is_main_component: bool, include_fix : Option<TokenStream>, include_imports: bool, is_custom_interpolatable: bool) -> proc_macro2::TokenStream {

    let pascal_identifier = input_parsed.ident.to_string();

    let static_property_definitions = get_static_property_definitions_from_tokens(input_parsed.data);
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

    let output = TemplateArgsDerivePax {
        args_primitive: None,
        args_struct_only_component: None,
        args_full_component: Some(ArgsFullComponent {
            is_main_component,
            raw_pax,
            template_dependencies,
            reexports_snippet,
        }),
        pascal_identifier,
        include_imports,
        static_property_definitions,
        is_custom_interpolatable,
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

#[proc_macro_derive(Pax, attributes(main, file, inlined, primitive, custom, default))]
pub fn pax_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let attrs = &input.attrs;

    let mut is_main_component = false;
    let mut file_path: Option<String> = None;
    let mut inlined_contents: Option<String> = None;
    let mut custom_values: Option<Vec<String>> = None;
    let mut primitive_instance_import_path: Option<String> = None;
    let mut is_primitive = false;

    // iterate through `derive macro helper attributes` to gather config & args
    for attr in attrs {
        if attr.path.is_ident("file") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                if let Some(nested_meta) = meta_list.nested.first() {
                    if let syn::NestedMeta::Lit(Lit::Str(file_str)) = nested_meta {
                        file_path = Some(file_str.value());
                    }
                }
            }
        } else if attr.path.is_ident("primitive") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                if let Some(nested_meta) = meta_list.nested.first() {
                    if let syn::NestedMeta::Lit(Lit::Str(file_str)) = nested_meta {
                        primitive_instance_import_path = Some(file_str.value());
                        is_primitive = true;
                    }
                }
            }
        } else if attr.path.is_ident("inlined") {
            let tokens = attr.tokens.clone();
            let mut content = proc_macro2::TokenStream::new();

            for token in tokens {
                match token {
                    proc_macro2::TokenTree::Group(group) => {
                        if group.delimiter() == proc_macro2::Delimiter::Parenthesis {
                            content.extend(group.stream());
                        }
                    }
                    _ => {}
                }
            }

            if !content.is_empty() {
                inlined_contents = Some(content.to_string());
            }
        } else {
            match attr.parse_meta() {
                Ok(Meta::Path(path)) => {
                    if path.is_ident("main") {
                        is_main_component = true;
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
    }

    //Validation
    if let (Some(_), Some(_)) = (file_path.as_ref(), inlined_contents.as_ref()) {
        return syn::Error::new_spanned(input.ident, "`#[file(...)]` and `#[inlined(...)]` attributes cannot be used together")
            .to_compile_error()
            .into();
    }
    if let (None, None) = (file_path.as_ref(), inlined_contents.as_ref()) {
        // &&
        if is_main_component {
            return syn::Error::new_spanned(input.ident, "Main (application-root) components must specify either a Pax file or inlined Pax content, e.g. #[file(\"some-file.pax\")] or #[inlined(<SomePax />)]")
                .to_compile_error()
                .into();
        }
    }
    if is_primitive {
        const ERR : &str = "Primitives cannot have attached templates.  Instead, specify a fully qualified Rust import path pointing to the `impl RenderNode` struct for this primitive.";

        if let Some(_) = file_path.as_ref() {
            return syn::Error::new_spanned(input.ident, ERR)
                .to_compile_error()
                .into();
        }
        if let Some(_) = inlined_contents.as_ref() {
            return syn::Error::new_spanned(input.ident, ERR)
                .to_compile_error()
                .into();
        }

    }

    // Implement Clone
    let mut clone_impl = match &input.data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields) => {
                    let field_names = fields.named.iter().map(|f| &f.ident);
                    quote! {
                        impl #impl_generics ::core::clone::Clone for #name #ty_generics #where_clause {
                            fn clone(&self) -> Self {
                                #name {
                                    #( #field_names : ::core::clone::Clone::clone(&self.#field_names), )*
                                }
                            }
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let indices = (0..fields.unnamed.len()).map(syn::Index::from);
                    quote! {
                        impl #impl_generics ::core::clone::Clone for #name #ty_generics #where_clause {
                            fn clone(&self) -> Self {
                                #name (
                                    #( ::core::clone::Clone::clone(&self.#indices), )*
                                )
                            }
                        }
                    }
                }
                Fields::Unit => {
                    quote! {
                        impl #impl_generics ::core::clone::Clone for #name #ty_generics #where_clause {
                            fn clone(&self) -> Self {
                                #name
                            }
                        }
                    }
                }
            }
        }
        Data::Enum(data_enum) => {
            let variants = data_enum.variants.iter().map(|v| {
                let variant_ident = &v.ident;
                match &v.fields {
                    Fields::Named(fields) => {
                        let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
                        quote! {
                            #name::#variant_ident { #(ref #field_names, )* } => {
                                #name::#variant_ident {
                                    #( #field_names : #field_names.clone(), )*
                                }
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let indices: Vec<_> = (0..fields.unnamed.len()).map(|i| {
                            let name = format!("_{}", i);
                            Ident::new(&name, proc_macro2::Span::call_site())
                        }).collect();
                        quote! {
                            #name::#variant_ident ( #(ref #indices, )* ) => {
                                #name::#variant_ident (
                                    #( #indices.clone(), )*
                                )
                            }
                        }
                    }
                    Fields::Unit => {
                        quote! {
                            #name::#variant_ident => #name::#variant_ident,
                        }
                    }
                }
            });

            quote! {
                impl #impl_generics ::core::clone::Clone for #name #ty_generics #where_clause {
                    fn clone(&self) -> Self {
                        match self {
                            #( #variants )*
                        }
                    }
                }
            }
        }
        Data::Union(_) => {
            panic!("`Pax` derive macro does not support unions.");
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
        Data::Enum(data_enum) => {
            let default_variant = data_enum.variants.iter().find(|variant| {
                variant.attrs.iter().any(|attr| attr.path.is_ident("default"))
            });

            match default_variant {
                Some(variant) => {
                    let variant_ident = &variant.ident;
                    quote! {
                        impl #impl_generics Default for #name #ty_generics #where_clause {
                            fn default() -> Self {
                                Self::#variant_ident
                            }
                        }
                    }
                }
                None => {
                    quote! {
                        compile_error!("#[default] attribute required on one of the enum variants");
                    }
                }
            }
        }
        Data::Union(_) => {
            quote! {
                compile_error!("Pax derive does not currently support Unions");
            }
        }
    };

    let mut include_imports = true;
    let mut is_custom_interpolatable = false;

    //wipe out the above derives if `#[custom(...)]` attrs are set
    if let Some(custom) = custom_values {
        if custom.contains(&"Default".to_string()) {
            //wipe out the above derive
            default_impl = quote! {};
        }
        if custom.contains(&"Clone".to_string()) {
            //wipe out the above derive
            clone_impl = quote! {};
        }
        if custom.contains(&"Imports".to_string()) {
            include_imports = false;
        }
        if custom.contains(&"Interpolatable".to_string()) {
            is_custom_interpolatable = true;
        }
    }

    let is_pax_file = matches!(&file_path, Some(_f));
    let is_pax_inlined = matches!(&inlined_contents, Some(_i));
    // let debug = format!("//is_pax_file:  {}, is_pax_inlined: {}\n//inlined_contents: /*{:?}*/", &is_pax_file, &is_pax_inlined, &inlined_contents);


    let appended_tokens = if is_pax_file {
        let filename = if let Some(p) = file_path.as_ref() {p} else {unreachable!()};
        let current_dir = std::env::current_dir().expect("Unable to get current directory");
        let path = current_dir.join(Path::new("src").join(Path::new(&filename)));

        // generate_include to watch for changes in specified file, ensuring macro is re-evaluated when file changes
        let name = Ident::new("PaxFile", Span::call_site());
        let include_fix = generate_include(&name,path.clone().to_str().unwrap());

        let file = File::open(path);
        let mut content = String::new();
        let _ = file.unwrap().read_to_string(&mut content);
        let stream: proc_macro::TokenStream = content.parse().unwrap();
        pax_full_component(stream.to_string(), input.clone(), is_main_component, Some(include_fix),include_imports, is_custom_interpolatable)

    } else if is_pax_inlined {
        let contents = if let Some(p) = inlined_contents {p} else {unreachable!()};

        pax_full_component(contents, input.clone(), is_main_component, None,include_imports, is_custom_interpolatable)
    } else if is_primitive {
        pax_primitive(input.clone(), primitive_instance_import_path.unwrap(), include_imports, is_custom_interpolatable)
    } else {
        pax_struct_only_component(input.clone(), include_imports, is_custom_interpolatable)
    };

    let output = quote! {
        #appended_tokens
        #clone_impl
        #default_impl
    };

    output.into()
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

