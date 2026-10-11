use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parser, punctuated::Punctuated, Expr, Token};

pub fn init(input: TokenStream, with_options: bool) -> TokenStream {
    let result = (|| -> color_eyre::eyre::Result<proc_macro2::TokenStream> {
        let arguments = Punctuated::<Expr, Token![,]>::parse_terminated.parse(input)?;
        let mut arguments = arguments.into_iter().collect::<Vec<_>>();
        let options = if with_options {
            arguments
                .pop()
                .ok_or_else(|| color_eyre::eyre::eyre!("init_with! requires InitOptions"))?
        } else {
            syn::parse_quote!(pax_kit::host::InitOptions::default())
        };
        if arguments.len() > 1 {
            color_eyre::eyre::bail!("expected an optional UI library path");
        }
        let manifest_dir =
            std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let manifest = manifest_dir.join("Cargo.toml").canonicalize()?;
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(&manifest)
            .no_deps()
            .other_options(vec!["--offline".into()])
            .exec()?;
        let caller = metadata
            .packages
            .iter()
            .find(|p| {
                p.manifest_path.as_std_path().canonicalize().ok().as_deref()
                    == Some(manifest.as_path())
            })
            .unwrap();
        let (library, helper_package): (syn::Path, String) = if let Some(argument) = arguments.pop()
        {
            let path = match argument {
                Expr::Path(path) if path.path.segments.len() == 1 => path.path,
                _ => color_eyre::eyre::bail!("expected a UI library crate name, such as my_ui"),
            };
            let name = path.segments[0].ident.to_string();
            let package = if caller.targets.iter().any(|t| {
                t.name.replace('-', "_") == name
                    && t.kind
                        .iter()
                        .any(|k| k == "lib" || k == "cdylib" || k == "rlib")
            }) {
                caller.name.clone()
            } else {
                caller
                    .dependencies
                    .iter()
                    .find(|d| d.rename.as_ref().unwrap_or(&d.name).replace('-', "_") == name)
                    .map(|d| d.name.clone())
                    // A custom library target can differ from its package name.
                    // Rust resolves this path; that library also exports a helper
                    // under its target name. No second dependency graph is needed.
                    .unwrap_or(name)
            };
            (path, package)
        } else {
            let layout = pax_codegen::project_layout::ProjectLayout::resolve(&manifest_dir)?;
            let package = metadata
                .packages
                .iter()
                .find(|p| {
                    p.manifest_path
                        .parent()
                        .unwrap()
                        .as_std_path()
                        .canonicalize()
                        .ok()
                        .as_deref()
                        == Some(layout.cartridge_dir.as_path())
                })
                .ok_or_else(|| {
                    color_eyre::eyre::eyre!(
                        "Selected UI library must be a Cargo dependency of the host"
                    )
                })?;
            let name = if package.id == caller.id {
                layout.library_name.replace('-', "_")
            } else {
                let dependency = caller
                    .dependencies
                    .iter()
                    .find(|d| d.name == package.name)
                    .ok_or_else(|| {
                        color_eyre::eyre::eyre!(
                            "Add `{}` as a direct Cargo dependency of the host",
                            package.name
                        )
                    })?;
                dependency
                    .rename
                    .clone()
                    .unwrap_or(layout.library_name)
                    .replace('-', "_")
            };
            (syn::parse_str(&name)?, package.name.clone())
        };
        let helper = quote::format_ident!(
            "{}",
            crate::source::handoff_helper_name(&helper_package, "program")
        );
        Ok(quote!({
            const _: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
            #library::#helper!(pax_kit::pax_engine::pax_macro::__pax_launch,
                #options)
        }))
    })();
    match result {
        Ok(tokens) => tokens.into(),
        Err(error) => syn::Error::new(proc_macro2::Span::call_site(), format!("{error:#}"))
            .into_compile_error()
            .into(),
    }
}

pub fn expand(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::ExprTuple);
    let mut arguments = input.elems.iter();
    let recipe = arguments.next().expect("generated launch recipe");
    let options = arguments.next().expect("generated launch options");
    quote!(pax_kit::host::__init_cartridge(#recipe, #options)).into()
}
