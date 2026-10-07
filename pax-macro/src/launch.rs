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
        let manifest = manifest_dir.join("Cargo.toml");
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(&manifest)
            .other_options(vec!["--offline".into()])
            .exec()?;
        let caller = metadata
            .packages
            .iter()
            .find(|p| p.manifest_path.as_std_path() == manifest)
            .unwrap();
        let node = metadata
            .resolve
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == caller.id)
            .unwrap();
        let (library, target_package): (syn::Path, _) = if let Some(argument) = arguments.pop() {
            let path = match argument {
                Expr::Path(path) if path.path.segments.len() == 1 => path.path,
                _ => color_eyre::eyre::bail!("expected a UI library crate name, such as my_ui"),
            };
            let name = path.segments[0].ident.to_string();
            let package = if let Some(dependency) = node.deps.iter().find(|d| d.name == name) {
                metadata
                    .packages
                    .iter()
                    .find(|p| p.id == dependency.pkg)
                    .unwrap()
            } else if caller.targets.iter().any(|t| {
                t.name.replace('-', "_") == name && t.crate_types.iter().any(|k| k == "cdylib")
            }) {
                caller
            } else {
                color_eyre::eyre::bail!("UI library `{name}` must be this package's library or a direct Cargo dependency");
            };
            (path, package)
        } else {
            let layout = pax_codegen::project_layout::ProjectLayout::resolve(&manifest_dir)?;
            let package = metadata
                .packages
                .iter()
                .find(|p| p.manifest_path.parent().unwrap().as_std_path() == layout.cartridge_dir)
                .ok_or_else(|| {
                    color_eyre::eyre::eyre!(
                        "Selected UI library must be a Cargo dependency of the host"
                    )
                })?;
            let name = if package.id == caller.id {
                layout.library_name.replace('-', "_")
            } else {
                node.deps
                    .iter()
                    .find(|d| d.pkg == package.id)
                    .ok_or_else(|| {
                        color_eyre::eyre::eyre!(
                            "Add `{}` as a direct Cargo dependency of the host",
                            package.name
                        )
                    })?
                    .name
                    .clone()
            };
            (syn::parse_str(&name)?, package)
        };
        let target = target_package
            .targets
            .iter()
            .find(|t| t.crate_types.iter().any(|k| k == "cdylib"))
            .ok_or_else(|| {
                color_eyre::eyre::eyre!(
                    "The selected UI library needs crate-type = [\"rlib\", \"cdylib\"]"
                )
            })?;
        let helper = quote::format_ident!(
            "{}",
            crate::source::helper_name(target.src_path.as_std_path(), "program")
        );
        // Cargo examples select a UI dependency; a standalone host keeps its
        // own package metadata (including a separate cartridge selection).
        let launch_package = if caller.targets.iter().any(|target| {
            target.kind.iter().any(|kind| kind == "bin")
                && target
                    .required_features
                    .iter()
                    .any(|feature| feature == "standalone")
        }) {
            caller
        } else {
            target_package
        };
        let project = launch_package.manifest_path.parent().unwrap().as_str();
        Ok(quote!({
            const __PAX_ROLE: Option<&str> = option_env!("PAX_BUILD_ROLE");
            #library::#helper!(pax_kit::pax_engine::pax_macro::__pax_launch,
                (#options, #project, #with_options || __PAX_ROLE == Some("application")))
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
