//! Shared syntax for static runnable discovery and Rust macro expansion.
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Path, Result, Token};

/// One statically selected component and its optional initializer.
/// Paths are resolved in the module containing the declaration.
pub struct RunnableDeclaration {
    pub root: Path,
    pub init: Option<Path>,
    pub engine_import_path: Option<Path>,
}

impl Parse for RunnableDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut root = None;
        let mut init = None;
        let mut engine_import_path = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let slot = match key.to_string().as_str() {
                "root" => &mut root,
                "init" => &mut init,
                "engine_import_path" => &mut engine_import_path,
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        "expected `root`, `init`, or `engine_import_path`",
                    ))
                }
            };
            if slot.is_some() {
                return Err(syn::Error::new(
                    key.span(),
                    format!("duplicate `{key}` in runnable declaration"),
                ));
            }
            *slot = Some(input.parse::<Path>()?);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self {
            root: root
                .ok_or_else(|| input.error("runnable! requires `root: path::ToComponent`"))?,
            init,
            engine_import_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_requires_one_root_and_rejects_unknown_or_duplicate_keys() {
        for invalid in [
            "",
            "init: init",
            "root: App, root: Other",
            "root: App, init: a, init: b",
            "root: App, typo: f",
        ] {
            assert!(
                syn::parse_str::<RunnableDeclaration>(invalid).is_err(),
                "{invalid}"
            );
        }
        assert!(syn::parse_str::<RunnableDeclaration>(
            "root: crate::ui::App, init: crate::ui::init,"
        )
        .is_ok());
        assert!(syn::parse_str::<RunnableDeclaration>("root: App").is_ok());
    }
}
