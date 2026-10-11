//! Compiler-selected source discovery for the macro frontend.
//!
//! Facts must come from `cfg` branches expanded in the owning compilation unit.
//! Missing facts are requests, never assumed false. This module neither reads
//! Cargo feature environment variables nor evaluates Rust predicates itself.

use color_eyre::eyre::{eyre, Result};
use quote::{quote, ToTokens};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use syn::parse::Parser;
use syn::{Attribute, Fields, Item, Meta, NestedMeta};

pub type Facts = BTreeMap<String, bool>;
pub type Sources = BTreeMap<PathBuf, syn::File>;

/// Source identity and dependency edges supplied by the owning compilation.
/// Declarations come from no-deps metadata; links come from dependency helpers,
/// so Cargo's selected version, rename, and configuration remain authoritative.
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourcePackage {
    pub entry: PathBuf,
    pub manifest_dir: PathBuf,
    pub name: String,
    pub crate_name: String,
    pub dependencies: BTreeMap<String, String>,
    pub links: BTreeMap<String, PathBuf>,
}

#[derive(Debug)]
pub struct NeedsDependency {
    pub owner: PathBuf,
    pub name: String,
}

impl std::fmt::Display for NeedsDependency {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} needs compiler-selected dependency {}",
            self.owner.display(),
            self.name
        )
    }
}
impl std::error::Error for NeedsDependency {}

#[derive(Debug)]
pub struct NeedsPredicate(pub String);

impl std::fmt::Display for NeedsPredicate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "owning compiler must evaluate cfg({})", self.0)
    }
}
impl std::error::Error for NeedsPredicate {}

#[derive(Debug)]
pub struct NeedsSource(pub PathBuf);

impl std::fmt::Display for NeedsSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Missing compiler-selected source for {}",
            self.0.display()
        )
    }
}
impl std::error::Error for NeedsSource {}

/// A cache of original parsed files, scoped to one compiler source session.
/// Configured ASTs are rebuilt from these originals for each fact set.
#[derive(Default)]
pub struct SourceCache {
    files: HashMap<PathBuf, syn::File>,
}

impl SourceCache {
    pub fn parsed_files(&self) -> usize {
        self.files.len()
    }

    fn read(&mut self, path: &Path) -> Result<syn::File> {
        if !self.files.contains_key(path) {
            let source = std::fs::read_to_string(path)?;
            self.files
                .insert(path.to_owned(), syn::parse_file(&source)?);
        }
        Ok(self.files[path].clone())
    }
}

fn evaluate(predicate: &NestedMeta, facts: &Facts) -> Result<bool> {
    let predicate = predicate.to_token_stream().to_string();
    facts
        .get(&predicate)
        .copied()
        .ok_or_else(|| NeedsPredicate(predicate).into())
}

/// Expand conditional attributes before interpreting Pax or module attributes.
/// The original token spans survive, including nested inline Pax templates.
pub fn configure_attributes(attrs: &mut Vec<Attribute>, facts: &Facts) -> Result<bool> {
    let mut output = Vec::new();
    for attr in std::mem::take(attrs) {
        if attr.path.is_ident("cfg") || attr.path.is_ident("cfg_attr") {
            let (predicate, rest) = attr.parse_args_with(|input: syn::parse::ParseStream| {
                let predicate: NestedMeta = input.parse()?;
                if !input.is_empty() {
                    input.parse::<syn::Token![,]>()?;
                }
                Ok((predicate, input.parse::<proc_macro2::TokenStream>()?))
            })?;
            let enabled = evaluate(&predicate, facts)?;
            if attr.path.is_ident("cfg") {
                if !rest.is_empty() {
                    return Err(eyre!("cfg takes one predicate"));
                }
                if !enabled {
                    return Ok(false);
                }
            } else if enabled {
                // Attribute payloads can contain Pax DSL, not just Rust Meta.
                // Groups stay intact while separating outer attributes.
                let mut expanded = Vec::new();
                let mut payload = proc_macro2::TokenStream::new();
                for token in rest
                    .into_iter()
                    .chain(std::iter::once(proc_macro2::TokenTree::Punct(
                        proc_macro2::Punct::new(',', proc_macro2::Spacing::Alone),
                    )))
                {
                    if matches!(&token, proc_macro2::TokenTree::Punct(p) if p.as_char() == ',') {
                        if !payload.is_empty() {
                            expanded.extend(Attribute::parse_outer.parse2(quote!(#[#payload]))?);
                            payload = proc_macro2::TokenStream::new();
                        }
                    } else {
                        payload.extend(std::iter::once(token));
                    }
                }
                if !configure_attributes(&mut expanded, facts)? {
                    return Ok(false);
                }
                output.extend(expanded);
            }
        } else {
            output.push(attr);
        }
    }
    *attrs = output;
    Ok(true)
}

fn configure_fields(fields: &mut Fields, facts: &Facts) -> Result<()> {
    let fields = match fields {
        Fields::Named(fields) => &mut fields.named,
        Fields::Unnamed(fields) => &mut fields.unnamed,
        Fields::Unit => return Ok(()),
    };
    let mut output = syn::punctuated::Punctuated::new();
    for mut field in std::mem::take(fields) {
        if configure_attributes(&mut field.attrs, facts)? {
            output.push(field);
        }
    }
    *fields = output;
    Ok(())
}

fn attributes(item: &mut Item) -> Result<&mut Vec<Attribute>> {
    Ok(match item {
        Item::Const(x) => &mut x.attrs,
        Item::Enum(x) => &mut x.attrs,
        Item::ExternCrate(x) => &mut x.attrs,
        Item::Fn(x) => &mut x.attrs,
        Item::ForeignMod(x) => &mut x.attrs,
        Item::Impl(x) => &mut x.attrs,
        Item::Macro(x) => &mut x.attrs,
        Item::Macro2(x) => &mut x.attrs,
        Item::Mod(x) => &mut x.attrs,
        Item::Static(x) => &mut x.attrs,
        Item::Struct(x) => &mut x.attrs,
        Item::Trait(x) => &mut x.attrs,
        Item::TraitAlias(x) => &mut x.attrs,
        Item::Type(x) => &mut x.attrs,
        Item::Union(x) => &mut x.attrs,
        Item::Use(x) => &mut x.attrs,
        _ => return Err(eyre!("Unsupported source item in configured discovery")),
    })
}

fn configure_items(items: &mut Vec<Item>, facts: &Facts) -> Result<()> {
    let mut output = Vec::new();
    for mut item in std::mem::take(items) {
        if !configure_attributes(attributes(&mut item)?, facts)? {
            continue;
        }
        match &mut item {
            Item::Struct(item) => configure_fields(&mut item.fields, facts)?,
            Item::Enum(item) => {
                let mut variants = syn::punctuated::Punctuated::new();
                for mut variant in std::mem::take(&mut item.variants) {
                    if configure_attributes(&mut variant.attrs, facts)? {
                        configure_fields(&mut variant.fields, facts)?;
                        variants.push(variant);
                    }
                }
                item.variants = variants;
            }
            Item::Mod(item) => {
                if let Some((_, items)) = &mut item.content {
                    configure_items(items, facts)?;
                }
            }
            Item::Impl(item) => {
                let mut members = Vec::new();
                for mut member in std::mem::take(&mut item.items) {
                    let attrs = match &mut member {
                        syn::ImplItem::Const(x) => &mut x.attrs,
                        syn::ImplItem::Method(x) => &mut x.attrs,
                        syn::ImplItem::Type(x) => &mut x.attrs,
                        syn::ImplItem::Macro(x) => &mut x.attrs,
                        _ => return Err(eyre!("Unsupported impl item in configured discovery")),
                    };
                    if configure_attributes(attrs, facts)? {
                        members.push(member);
                    }
                }
                item.items = members;
            }
            _ => {}
        }
        output.push(item);
    }
    *items = output;
    Ok(())
}

/// Filter the source forms consumed by Pax's scanner. Function bodies and macro
/// expansions remain rustc's responsibility; this is not a Rust expander.
pub fn configure_file(file: &mut syn::File, facts: &Facts) -> Result<()> {
    if configure_attributes(&mut file.attrs, facts)? {
        configure_items(&mut file.items, facts)
    } else {
        file.items.clear();
        Ok(())
    }
}

pub fn module_directory(path: &Path) -> PathBuf {
    let parent = path.parent().expect("source file has parent");
    if matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("lib.rs" | "main.rs" | "mod.rs")
    ) {
        parent.to_owned()
    } else {
        parent.join(path.file_stem().expect("source file has stem"))
    }
}

/// Resolve a module only after its cfg/cfg_attr attributes have been processed.
pub fn module_file(
    item: &syn::ItemMod,
    source: &Path,
    directory: &Path,
    inline: bool,
) -> Result<PathBuf> {
    for attr in &item.attrs {
        if attr.path.is_ident("path") {
            if let Meta::NameValue(value) = attr.parse_meta()? {
                if let syn::Lit::Str(path) = value.lit {
                    return Ok(if inline {
                        directory
                    } else {
                        source.parent().unwrap()
                    }
                    .join(path.value()));
                }
            }
            return Err(eyre!("Expected a string in #[path]"));
        }
    }
    let direct = directory.join(format!("{}.rs", item.ident));
    let nested = directory.join(item.ident.to_string()).join("mod.rs");
    match (direct.exists(), nested.exists()) {
        (true, false) => Ok(direct),
        (false, true) => Ok(nested),
        (true, true) => Err(eyre!("Ambiguous module {}", item.ident)),
        (false, false) => Err(eyre!(
            "Missing active module {} in {}",
            item.ident,
            directory.display()
        )),
    }
}

/// Traverse only active module files. Missing facts stop traversal before a
/// conditional file or template is opened. Repeated passes reuse parsed files.
pub fn discover(entry: &Path, facts: &Facts, cache: &mut SourceCache) -> Result<Sources> {
    fn file(
        path: &Path,
        facts: &Facts,
        cache: &mut SourceCache,
        sources: &mut Sources,
    ) -> Result<()> {
        let path = path.canonicalize()?;
        if sources.contains_key(&path) {
            return Ok(());
        }
        let mut parsed = cache.read(&path)?;
        configure_file(&mut parsed, facts)?;
        sources.insert(path.clone(), parsed.clone());
        modules(
            &parsed.items,
            &path,
            &module_directory(&path),
            false,
            facts,
            cache,
            sources,
        )
    }
    fn modules(
        items: &[Item],
        source: &Path,
        directory: &Path,
        inline: bool,
        facts: &Facts,
        cache: &mut SourceCache,
        sources: &mut Sources,
    ) -> Result<()> {
        for item in items {
            if let Item::Mod(item) = item {
                if let Some((_, items)) = &item.content {
                    // Inline #[path] changes Rust's directory search semantics;
                    // reject until that form has its own qualification fixture.
                    if item.attrs.iter().any(|a| a.path.is_ident("path")) {
                        return Err(eyre!(
                            "Configured discovery does not yet support #[path] on inline modules"
                        ));
                    }
                    modules(
                        items,
                        source,
                        &directory.join(item.ident.to_string()),
                        true,
                        facts,
                        cache,
                        sources,
                    )?;
                } else {
                    file(
                        &module_file(item, source, directory, inline)?,
                        facts,
                        cache,
                        sources,
                    )?;
                }
            }
        }
        Ok(())
    }
    let mut sources = Sources::new();
    file(entry, facts, cache, &mut sources)?;
    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(values: &[(&str, bool)]) -> Facts {
        values
            .iter()
            .map(|(p, value)| {
                let predicate: proc_macro2::TokenStream = p.parse().unwrap();
                (predicate.to_string(), *value)
            })
            .collect()
    }

    #[test]
    fn unknown_configuration_stops_before_opening_disabled_missing_modules() {
        let directory = tempfile::tempdir().unwrap();
        let entry = directory.path().join("lib.rs");
        std::fs::write(
            &entry,
            "#[cfg(optional_ui)] mod missing; pub struct Active;",
        )
        .unwrap();
        let mut cache = SourceCache::default();
        let error = discover(&entry, &Facts::new(), &mut cache).err().unwrap();
        assert_eq!(
            error.downcast_ref::<NeedsPredicate>().unwrap().0,
            "optional_ui"
        );
        let sources = discover(&entry, &facts(&[("optional_ui", false)]), &mut cache).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(cache.parsed_files(), 1);
        assert!(
            discover(&entry, &facts(&[("optional_ui", true)]), &mut cache)
                .err()
                .unwrap()
                .to_string()
                .contains("Missing active module")
        );
    }

    #[test]
    fn cfg_attr_selects_module_path_before_reading_source() {
        let directory = tempfile::tempdir().unwrap();
        let entry = directory.path().join("lib.rs");
        std::fs::write(
            &entry,
            r#"
            #[cfg_attr(web, path="selected.rs")]
            #[cfg_attr(not(web), path="missing.rs")]
            mod ui;
        "#,
        )
        .unwrap();
        std::fs::write(directory.path().join("selected.rs"), "pub struct Web;").unwrap();
        let mut cache = SourceCache::default();
        let sources = discover(
            &entry,
            &facts(&[("web", true), ("not(web)", false)]),
            &mut cache,
        )
        .unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(cache.parsed_files(), 2);
    }

    #[test]
    fn nested_cfg_attr_preserves_inline_template_and_filters_data_and_handlers() {
        let mut file = syn::parse_file(
            r#"
            #[cfg_attr(enabled, cfg_attr(nested, inlined(<Widget width=100%/>)))]
            struct App {
                #[cfg(enabled)] pub value: u32,
                #[cfg(disabled)] pub absent: Undefined,
            }
            enum State {
                #[cfg(disabled)] Missing(Undefined),
                Present { #[cfg(disabled)] absent: Undefined, value: u32 },
            }
            impl App {
                #[cfg(disabled)] fn missing(&self) { invalid(); }
                #[cfg(enabled)] fn active(&self) {}
            }
        "#,
        )
        .unwrap();
        configure_file(
            &mut file,
            &facts(&[("enabled", true), ("nested", true), ("disabled", false)]),
        )
        .unwrap();
        let syn::Item::Struct(item) = &file.items[0] else {
            panic!()
        };
        assert_eq!(item.fields.len(), 1);
        assert!(item.attrs[0].path.is_ident("inlined"));
        assert!(item.attrs[0].tokens.to_string().contains("Widget"));
        let syn::Item::Enum(item) = &file.items[1] else {
            panic!()
        };
        assert_eq!(item.variants.len(), 1);
        assert_eq!(item.variants[0].fields.len(), 1);
        let syn::Item::Impl(item) = &file.items[2] else {
            panic!()
        };
        assert_eq!(item.items.len(), 1);
    }

    #[test]
    fn disabled_parent_does_not_request_child_configuration() {
        let mut file =
            syn::parse_file("#[cfg(parent)] mod missing { #[cfg(child)] struct Child; }").unwrap();
        configure_file(&mut file, &facts(&[("parent", false)])).unwrap();
        assert!(file.items.is_empty());
    }
}
