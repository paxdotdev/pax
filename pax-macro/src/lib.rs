extern crate proc_macro;
extern crate proc_macro2;
mod templating;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::{env, path::PathBuf}; // Necessary for `writeln!` macro to work

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};

use syn::punctuated::Punctuated;
use templating::{
    ArgsFullComponent, EnumVariantDefinition, InternalDefinitions, StaticPropertyDefinition,
    TemplateArgsDerivePax, TemplateBuildConfig,
};

use sailfish::TemplateOnce;

const CRATES_WITHOUT_ROOT_CARTRIDGE_SNIPPET: &[&str] = &["pax-designer", "pax-std", "pax-runtime"];

fn is_root_crate() -> bool {
    let is_not_blacklisted = !CRATES_WITHOUT_ROOT_CARTRIDGE_SNIPPET
        .contains(&std::env::var("CARGO_PKG_NAME").unwrap_or_default().as_str());
    is_not_blacklisted
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            let normalized = value.to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn env_flag_explicitly_disabled(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            let normalized = value.to_ascii_lowercase();
            matches!(normalized.as_str(), "0" | "false" | "no" | "off")
        })
        .unwrap_or(false)
}

fn cargo_feature_enabled(feature: &str) -> bool {
    env::var_os(format!(
        "CARGO_FEATURE_{}",
        feature.replace('-', "_").to_ascii_uppercase()
    ))
    .is_some()
}

fn template_build_config() -> TemplateBuildConfig {
    let pax_build_target = env::var("PAX_BUILD_TARGET").unwrap_or_default();
    let designer = env_flag("PAX_BUILD_DESIGNER") || cargo_feature_enabled("designer");
    let designtime =
        designer || env_flag("PAX_BUILD_DESIGNTIME") || cargo_feature_enabled("designtime");

    TemplateBuildConfig {
        web: pax_build_target == "web" || cargo_feature_enabled("web"),
        macos: pax_build_target == "macos" || cargo_feature_enabled("macos"),
        ios: pax_build_target == "ios" || cargo_feature_enabled("ios"),
        ipados: pax_build_target == "ipados",
        designtime,
        designer,
    }
}

use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, FnArg, GenericArgument, ImplItem,
    ItemImpl, Lit, Meta, PatType, PathArguments, Signature, Token, Type,
};

fn pax_primitive(
    input_parsed: &DeriveInput,
    _primitive_instance_import_path: String,
    is_custom_interpolatable: bool,
    engine_import_path: String,
) -> proc_macro2::TokenStream {
    let _original_tokens = quote! { #input_parsed }.to_string();
    let pascal_identifier = input_parsed.ident.to_string();
    let is_enum = match &input_parsed.data {
        Data::Enum(_) => true,
        _ => false,
    };

    let internal_definitions = get_internal_definitions_from_tokens(&input_parsed.data);

    let output = TemplateArgsDerivePax {
        args_full_component: None,
        internal_definitions,
        pascal_identifier,
        is_custom_interpolatable,
        is_root_crate: is_root_crate(),
        _is_enum: is_enum,
        build_config: template_build_config(),
        engine_import_path,
    }
    .render_once()
    .unwrap()
    .to_string();

    TokenStream::from_str(&output).unwrap().into()
}

fn pax_struct_only_component(
    input_parsed: &DeriveInput,
    is_custom_interpolatable: bool,
    engine_import_path: String,
) -> proc_macro2::TokenStream {
    let pascal_identifier = input_parsed.ident.to_string();
    let is_enum = match &input_parsed.data {
        Data::Enum(_) => true,
        _ => false,
    };

    let internal_definitions = get_internal_definitions_from_tokens(&input_parsed.data);

    let output = TemplateArgsDerivePax {
        args_full_component: None,

        pascal_identifier: pascal_identifier.clone(),
        internal_definitions,
        is_root_crate: is_root_crate(),
        is_custom_interpolatable,
        _is_enum: is_enum,
        build_config: template_build_config(),
        engine_import_path,
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
/// into a list of `rustc` resolvable identifiers, possibly namespace-nested.
/// The macro template uses these paths when generating value coercion code.
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

/* Context:
[ ] Issue: we are including cartridge.partial.rs across every #[main], which e.g. causes build of pax-designer to fail
        when running Fireworks.

        Drafted solution:
            [ ] detect whether we are in the root crate of this build.
                [ ] might be able to store a static mutable Option<root_crate_pkg_name>, a write-once-read-many (WORM) signal to the rest of the build.
            [ ] in the stpl template, check this signal and only include the partial if we are in the root crate.
                [-] This might be fragile if somehow different versions of pax-macro are included in a build (is that possible or does cargo prevent it?) Answer: cargo prevents it.
 */

// Task at hand: [ ] detect whether we are in the root crate of this build.
//                 [ ] might be able to store a static mutable Option<root_crate_pkg_name>, a write-once-read-many (WORM) signal to the rest of the build.

//I should set this in the pax-macro crate, and then check it in the stpl template.
//How can I access that env value, correctly reflecting the package being built (instead of pax-macro, this package) ?
// [ ] I could set it in the build script, but that would require the user to add a build script to their project.
// [ ] I could set it in the pax-macro crate, but that would require the user to include pax-macro in their project.
//     This is okay -- pax-macro is available in the workspace, so it's not a big deal.
// To verify: what snippet of code will read the env value and set the static mutable variable?
// ```
// let root_crate_pkg_name = std::env::var("CARGO_PKG_NAME").unwrap_or_default();
// if let None = unsafe { ROOT_CRATE_PKG_NAME } {
//      unsafe { ROOT_CRATE_PKG_NAME = Some(root_crate_pkg_name); }
// }
// ```
// And to doubly verify: this first time this is run, CARGO_PKG_NAME should be the root crate being built?
// [ ] I should add a println! to the build script to verify this.

fn pax_full_component(
    _raw_pax: String,
    input_parsed: &DeriveInput,
    is_main_component: bool,
    include_fix: Option<TokenStream>,
    is_custom_interpolatable: bool,
    _associated_pax_file_path: Option<PathBuf>,
    engine_import_path: String,
) -> proc_macro2::TokenStream {
    let pascal_identifier = input_parsed.ident.to_string();
    let is_enum = match &input_parsed.data {
        Data::Enum(_) => true,
        _ => false,
    };

    let internal_definitions = get_internal_definitions_from_tokens(&input_parsed.data);

    // `PAX_DIR` is injected by `pax-cli` per project build. Read it at macro-expansion time
    // instead of `option_env!`, because proc-macro crates are compiled once and then reused.
    let pax_dir: Option<PathBuf> = env::var("PAX_DIR")
        .ok()
        // The \\?\ prefix in Windows paths is the Win32 file namespace prefix.
        // Needs to be removed to properly check if start matches below.
        .map(|v| v.trim_start_matches("\\\\?\\").to_string())
        .map(PathBuf::from);
    let current_manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| ".".into());
    let build_config = template_build_config();
    let needs_runtime_cartridge = is_main_component && is_root_crate();
    let needs_runtime_target_cartridge = needs_runtime_cartridge
        && (build_config.web || build_config.macos || build_config.ios || build_config.ipados);
    let missing_cartridge_snippet = |reason: String| {
        if needs_runtime_target_cartridge {
            format!("compile_error!({reason:?});")
        } else {
            "".to_string()
        }
    };
    // pax-engine's designtime feature propagates to this proc macro. Checking
    // the macro's compiled feature (rather than CARGO_FEATURE_* at expansion
    // time) provides a final backstop even when a wrapper crate activated it.
    let release_designtime_feature_conflict = cfg!(feature = "designtime")
        && env_flag_explicitly_disabled("PAX_BUILD_DESIGNTIME")
        && needs_runtime_target_cartridge;
    let cartridge_snippet = if release_designtime_feature_conflict {
        "compile_error!(\"Pax release builds cannot include the `pax-engine/designtime` feature. Remove the transitive designtime feature activation or build this app in debug mode.\");".to_string()
    } else if let Some(pax_dir) = pax_dir {
        if pax_dir.starts_with(&current_manifest_dir) {
            let cartridge_path = pax_dir.join("cartridge.partial.rs");
            let cartridge_path = cartridge_path.to_str().unwrap_or_else(|| {
                panic!("non-UTF-8 Pax cartridge path: {}", cartridge_path.display())
            });
            format!(
                "#[allow(dead_code, non_snake_case, non_upper_case_globals, unused_mut, unused_variables, mismatched_lifetime_syntaxes)]\ninclude!({cartridge_path:?});"
            )
        } else if needs_runtime_cartridge {
            missing_cartridge_snippet(format!(
                "PAX_DIR ({}) does not point at the active Pax project root ({}). Build Pax apps through pax-cli so the generated cartridge can be injected into #[pax].",
                pax_dir.display(),
                current_manifest_dir.display()
            ))
        } else {
            "".to_string()
        }
    } else if needs_runtime_cartridge {
        missing_cartridge_snippet(format!(
            "PAX_DIR was not set while expanding #[pax] for {}. Build Pax apps through pax-cli so the generated cartridge can be injected into #[pax].",
            current_manifest_dir.display()
        ))
    } else {
        "".to_string()
    };
    let output = TemplateArgsDerivePax {
        args_full_component: Some(ArgsFullComponent {
            is_main_component,
            cartridge_snippet,
        }),
        pascal_identifier,
        internal_definitions,
        is_root_crate: is_root_crate(),
        is_custom_interpolatable,
        _is_enum: is_enum,
        build_config,
        engine_import_path,
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
    svg_path: Option<String>,
    inlined_contents: Option<String>,
    custom_values: Option<Vec<String>>,
    engine_import_path: Option<String>,
    primitive_instance_import_path: Option<String>,
    is_primitive: bool,
    has_helpers: bool,
}

fn parse_config(attrs: &mut Vec<syn::Attribute>) -> Config {
    let mut config = Config {
        is_main_component: false,
        file_path: None,
        svg_path: None,
        inlined_contents: None,
        custom_values: None,
        primitive_instance_import_path: None,
        engine_import_path: None,
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
            Some(s) if s == "svg" => {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    if let Some(nested_meta) = meta_list.nested.first() {
                        if let syn::NestedMeta::Lit(Lit::Str(file_str)) = nested_meta {
                            config.svg_path = Some(file_str.value());
                            return false;
                        }
                    }
                }
            }
            Some(s) if s == "engine_import_path" => {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    if let Some(nested_meta) = meta_list.nested.first() {
                        if let syn::NestedMeta::Lit(Lit::Str(engine_import_path)) = nested_meta {
                            config.engine_import_path = Some(engine_import_path.value());
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
            Some(s) if s == "route_branch" => {
                return false;
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
    let template_source_count = [
        config.file_path.is_some(),
        config.svg_path.is_some(),
        config.inlined_contents.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if template_source_count > 1 {
        return Err(syn::Error::new_spanned(
            input.ident.clone(),
            "`#[file(...)]`, `#[svg(...)]`, and `#[inlined(...)]` attributes cannot be used together",
        )
        .to_compile_error()
        .into());
    }
    if template_source_count == 0 && config.is_main_component {
        return Err(syn::Error::new_spanned(
            input.ident.clone(),
            "Main (application-root) components must specify a Pax template source, e.g. #[file(\"some-file.pax\")], #[svg(\"some-file.svg\")], or #[inlined(<SomePax />)]",
        )
        .to_compile_error()
        .into());
    }
    if config.is_primitive && template_source_count > 0 {
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

    let engine_import_path = match config.engine_import_path {
        Some(prefix) => prefix,
        None => "pax_kit::pax_engine".to_string(),
    };

    //wipe out the above derives if `#[custom(...)]` attrs are set
    if let Some(custom) = config.custom_values {
        let custom_str: Vec<&str> = custom.iter().map(String::as_str).collect();
        trait_impls.retain(|v| !custom_str.contains(v));

        if custom.contains(&"Interpolatable".to_string()) {
            is_custom_interpolatable = true;
        }
    }

    let is_pax_file = config.file_path.is_some();
    let is_pax_svg = config.svg_path.is_some();
    let is_pax_inlined = config.inlined_contents.is_some();

    let appended_tokens = if is_pax_file {
        let file_name = config.file_path.unwrap();

        let root = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());

        let path = if Path::new(&root).join(&file_name).exists() {
            Path::new(&root).join(&file_name)
        } else {
            Path::new(&root).join("src/").join(&file_name)
        };

        // generate_include to watch for changes in specified file, ensuring macro is re-evaluated when file changes
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
            engine_import_path,
        )
    } else if is_pax_svg {
        let file_name = config.svg_path.unwrap();

        let root = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());

        let path = if Path::new(&root).join(&file_name).exists() {
            Path::new(&root).join(&file_name)
        } else {
            Path::new(&root).join("src/").join(&file_name)
        };

        // generate_include to watch for changes in specified file, ensuring macro is re-evaluated when file changes
        let name = Ident::new(&pascal_identifier, Span::call_site());
        let include_fix = generate_include(&name, &path);
        pax_full_component(
            String::new(),
            &input,
            config.is_main_component,
            Some(include_fix),
            is_custom_interpolatable,
            None,
            engine_import_path,
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
            engine_import_path,
        )
    } else if config.is_primitive {
        pax_primitive(
            &input,
            config.primitive_instance_import_path.unwrap(),
            is_custom_interpolatable,
            engine_import_path,
        )
    } else {
        pax_struct_only_component(&input, is_custom_interpolatable, engine_import_path)
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
