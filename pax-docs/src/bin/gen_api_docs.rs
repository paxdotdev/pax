use serde_json::Value;
use std::collections::HashSet;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .ok_or("pax-docs must live one level below workspace root")?
        .to_path_buf();
    let book_dir = manifest_dir.join("book").join("src");
    let api_dir = book_dir.join("api");

    let crates_path = api_dir.join("crates.txt");
    let crates = read_crates_list(&crates_path)?;
    if crates.is_empty() {
        return Err("No crates listed in api/crates.txt".into());
    }

    let public_crates_path = api_dir.join("public_crates.txt");
    let public_crates = read_public_crates(&public_crates_path)?;

    fs::create_dir_all(&api_dir)?;

    for krate in &crates {
        run_rustdoc_json(&workspace_root, krate)?;
    }
    let link_resolver = build_link_resolver(&workspace_root, &crates, &public_crates)?;

    let mut public_summary = Vec::new();
    let mut internal_summary = Vec::new();
    let mut generated_files = BTreeSet::new();
    let mut generated_public_crates = Vec::new();
    let mut generated_internal_crates = Vec::new();
    for krate in &crates {
        let is_public = public_crates.contains(krate);
        let (crate_dir, prefix) = if is_public {
            (api_dir.join(krate), format!("api/{krate}"))
        } else {
            (
                api_dir.join("internal").join(krate),
                format!("api/internal/{krate}"),
            )
        };
        let summary_target = if is_public {
            &mut public_summary
        } else {
            &mut internal_summary
        };
        let crate_list_target = if is_public {
            &mut generated_public_crates
        } else {
            &mut generated_internal_crates
        };

        generate_crate_docs(
            &workspace_root,
            &crate_dir,
            krate,
            &prefix,
            summary_target,
            &mut generated_files,
            crate_list_target,
            &link_resolver,
        )?;
    }

    write_api_index(
        &api_dir,
        &generated_public_crates,
        &generated_internal_crates,
        &mut generated_files,
    )?;
    update_summary(
        &book_dir.join("SUMMARY.md"),
        &public_summary,
        &internal_summary,
    )?;
    cleanup_stale_api_files(&api_dir, &generated_files)?;
    prune_empty_dirs(&api_dir, true)?;

    Ok(())
}

fn read_crates_list(path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if !path.exists() {
        let mut file = fs::File::create(path)?;
        writeln!(file, "# One crate name per line")?;
        writeln!(file, "pax-runtime-api")?;
    }

    let contents = fs::read_to_string(path)?;
    let crates = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    Ok(crates)
}

fn read_public_crates(path: &Path) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    if !path.exists() {
        let mut file = fs::File::create(path)?;
        writeln!(file, "# Public-facing crates (one per line)")?;
        writeln!(file, "pax-runtime-api")?;
        writeln!(file, "pax-std")?;
        writeln!(file, "pax-macro")?;
        writeln!(file, "pax-kit")?;
    }

    let contents = fs::read_to_string(path)?;
    let crates = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect::<HashSet<_>>();
    Ok(crates)
}

#[derive(Default)]
struct LinkResolver {
    targets_by_name: BTreeMap<String, LinkTarget>,
}

struct LinkTarget {
    target: Option<String>,
    priority: u8,
}

impl LinkResolver {
    fn insert(&mut self, name: String, target: String, priority: u8) {
        match self.targets_by_name.get_mut(&name) {
            Some(existing) if existing.target.as_ref() == Some(&target) => {}
            Some(existing) if priority < existing.priority => {
                *existing = LinkTarget {
                    target: Some(target),
                    priority,
                };
            }
            Some(existing) if priority == existing.priority => existing.target = None,
            Some(_) => {}
            None => {
                self.targets_by_name.insert(
                    name,
                    LinkTarget {
                        target: Some(target),
                        priority,
                    },
                );
            }
        }
    }

    fn target_for_name(&self, name: &str) -> Option<&str> {
        if is_unqualified_standard_type(name) {
            return None;
        }
        self.targets_by_name.get(name)?.target.as_deref()
    }
}

fn is_unqualified_standard_type(name: &str) -> bool {
    matches!(
        name,
        "Arc"
            | "BTreeMap"
            | "BTreeSet"
            | "Box"
            | "Cell"
            | "HashMap"
            | "HashSet"
            | "Option"
            | "Range"
            | "Rc"
            | "RefCell"
            | "Result"
            | "String"
            | "Vec"
    )
}

fn build_link_resolver(
    workspace_root: &Path,
    crates: &[String],
    public_crates: &HashSet<String>,
) -> Result<LinkResolver, Box<dyn std::error::Error>> {
    let generated_pages = collect_generated_api_pages(workspace_root, crates, public_crates)?;
    let crate_prefixes = crates
        .iter()
        .map(|krate| {
            let rustdoc_name = krate.replace('-', "_");
            let prefix = if public_crates.contains(krate) {
                format!("/api/{krate}")
            } else {
                format!("/api/internal/{krate}")
            };
            (rustdoc_name, prefix)
        })
        .collect::<BTreeMap<_, _>>();
    let crate_priorities = crates
        .iter()
        .map(|krate| {
            let rustdoc_name = krate.replace('-', "_");
            let priority = if public_crates.contains(krate) { 0 } else { 1 };
            (rustdoc_name, priority)
        })
        .collect::<BTreeMap<_, _>>();

    let mut resolver = LinkResolver::default();
    for krate in crates {
        let json = serde_json::from_str::<Value>(&fs::read_to_string(rustdoc_json_path(
            workspace_root,
            krate,
        ))?)?;
        let Some(paths) = json.get("paths").and_then(Value::as_object) else {
            continue;
        };
        for path_info in paths.values() {
            if !is_linkable_type_path(path_info) {
                continue;
            }
            let Some(path_parts) = path_info.get("path").and_then(Value::as_array) else {
                continue;
            };
            let parts = path_parts
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            let Some(target) =
                generated_doc_target_for_path(&parts, &crate_prefixes, &generated_pages)
            else {
                continue;
            };
            let Some(name) = parts.last() else {
                continue;
            };
            let priority = crate_priorities.get(parts[0]).copied().unwrap_or(1);
            resolver.insert((*name).to_string(), target, priority);
        }
    }

    Ok(resolver)
}

fn collect_generated_api_pages(
    workspace_root: &Path,
    crates: &[String],
    public_crates: &HashSet<String>,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let empty_resolver = LinkResolver::default();
    let mut pages = BTreeSet::new();
    for krate in crates {
        let json = serde_json::from_str::<Value>(&fs::read_to_string(rustdoc_json_path(
            workspace_root,
            krate,
        ))?)?;
        let index = json
            .get("index")
            .and_then(Value::as_object)
            .ok_or("rustdoc json missing index")?;
        let root_id = json
            .get("root")
            .and_then(Value::as_i64)
            .ok_or("rustdoc json missing root")?
            .to_string();
        let root_item = index.get(&root_id).ok_or("rustdoc root item missing")?;
        let root_items = module_items(root_item)?;

        let mut modules = Vec::new();
        collect_modules(index, &root_items, Vec::new(), &mut modules);
        let mut rendered_modules = Vec::new();
        for module in &modules {
            if let Some(rendered) = build_rendered_module(index, module, &empty_resolver) {
                rendered_modules.push(rendered);
            }
        }

        let mut root_items_filtered = filter_items(index, &root_items, &empty_resolver);
        root_items_filtered.retain(|item| item.kind != ItemKind::Module);
        let crate_docs = extract_docs(root_item);
        let prefix = if public_crates.contains(krate) {
            format!("/api/{krate}")
        } else {
            format!("/api/internal/{krate}")
        };
        if !crate_docs.is_empty() || !rendered_modules.is_empty() || !root_items_filtered.is_empty()
        {
            pages.insert(format!("{prefix}/index.md"));
        }

        let mut all_modules = Vec::new();
        flatten_rendered_modules(&rendered_modules, &mut all_modules);
        for module in &all_modules {
            let (_, rel_path) = module.path.display_name_and_path();
            pages.insert(format!("{prefix}/{rel_path}"));
        }
    }
    Ok(pages)
}

fn is_linkable_type_path(path_info: &Value) -> bool {
    matches!(
        path_info.get("kind").and_then(Value::as_str),
        Some("struct" | "enum" | "trait" | "type_alias" | "union")
    )
}

fn generated_doc_target_for_path(
    path_parts: &[&str],
    crate_prefixes: &BTreeMap<String, String>,
    generated_pages: &BTreeSet<String>,
) -> Option<String> {
    if path_parts.len() < 2 {
        return None;
    }
    let prefix = crate_prefixes.get(path_parts[0])?;
    let name = *path_parts.last()?;
    let module_parts = &path_parts[1..path_parts.len() - 1];
    let page = if module_parts.is_empty() {
        format!("{prefix}/index.md")
    } else {
        format!("{prefix}/{}.md", module_parts.join("/"))
    };
    if !generated_pages.contains(&page) {
        return None;
    }
    Some(format!("{page}#{}", mdbook_anchor(name)))
}

fn mdbook_anchor(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn generate_crate_docs(
    workspace_root: &Path,
    crate_dir: &Path,
    krate: &str,
    path_prefix: &str,
    api_summary: &mut Vec<ApiSummaryEntry>,
    generated_files: &mut BTreeSet<PathBuf>,
    generated_crates: &mut Vec<String>,
    link_resolver: &LinkResolver,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_path = rustdoc_json_path(workspace_root, krate);
    let json = serde_json::from_str::<Value>(&fs::read_to_string(&json_path)?)?;

    let index = json
        .get("index")
        .and_then(Value::as_object)
        .ok_or("rustdoc json missing index")?;
    let root_id = json
        .get("root")
        .and_then(Value::as_i64)
        .ok_or("rustdoc json missing root")?;
    let root_id = root_id.to_string();

    let root_item = index.get(&root_id).ok_or("rustdoc root item missing")?;
    let root_items = module_items(root_item)?;

    fs::create_dir_all(&crate_dir)?;

    let mut modules = Vec::new();
    collect_modules(index, &root_items, Vec::new(), &mut modules);
    let mut rendered_modules = Vec::new();
    for module in &modules {
        if let Some(rendered) = build_rendered_module(index, module, link_resolver) {
            rendered_modules.push(rendered);
        }
    }

    let mut all_modules = Vec::new();
    flatten_rendered_modules(&rendered_modules, &mut all_modules);
    all_modules.sort_by(|a, b| module_sort_key_rendered(a).cmp(&module_sort_key_rendered(b)));

    let mut root_items_filtered = filter_items(index, &root_items, link_resolver);
    root_items_filtered.retain(|item| item.kind != ItemKind::Module);

    let crate_docs = extract_docs(root_item);
    let crate_summary =
        summary_from_docs(&crate_docs).unwrap_or_else(|| format!("API reference for {krate}."));

    let crate_index_path = crate_dir.join("index.md");
    let crate_written = write_module_doc(
        &crate_index_path,
        krate,
        &crate_docs,
        &crate_summary,
        krate,
        &rendered_modules,
        &root_items_filtered,
        link_resolver,
    )?;

    if crate_written {
        record_generated_file(generated_files, &crate_index_path);
        generated_crates.push(krate.to_string());
        api_summary.push(ApiSummaryEntry {
            title: krate.to_string(),
            path: format!("{path_prefix}/index.md"),
            depth: 1,
        });
    }

    for module in &all_modules {
        let module_path = &module.path;
        let (display_name, rel_path) = module_path.display_name_and_path();

        let file_path = crate_dir.join(&rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let module_written = write_module_doc(
            &file_path,
            &display_name,
            &module.docs,
            &summary_from_docs(&module.docs)
                .unwrap_or_else(|| format!("API docs for {krate}::{display_name}.")),
            krate,
            &module.submodules,
            &module.items,
            link_resolver,
        )?;

        if module_written {
            record_generated_file(generated_files, &file_path);
            api_summary.push(ApiSummaryEntry {
                title: display_name,
                path: format!("{path_prefix}/{rel_path}"),
                depth: 1 + module_path.segments.len() as u8,
            });
        }
    }

    Ok(())
}

fn run_rustdoc_json(workspace_root: &Path, krate: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("cargo")
        .arg("+nightly")
        .arg("rustdoc")
        .arg("-p")
        .arg(krate)
        .arg("--")
        .arg("-Z")
        .arg("unstable-options")
        .arg("--output-format")
        .arg("json")
        .current_dir(workspace_root)
        .status()?;

    if !status.success() {
        return Err(format!("rustdoc json generation failed for {krate}").into());
    }

    Ok(())
}

fn rustdoc_json_path(workspace_root: &Path, krate: &str) -> PathBuf {
    let file_name = krate.replace('-', "_");
    workspace_root
        .join("target")
        .join("doc")
        .join(format!("{file_name}.json"))
}

#[derive(Clone)]
struct ModuleDoc {
    path: Option<ModulePath>,
    docs: String,
    items: Vec<String>,
    submodules: Vec<ModuleDoc>,
}

#[derive(Clone)]
struct ModuleRendered {
    path: ModulePath,
    docs: String,
    items: Vec<ItemDoc>,
    submodules: Vec<ModuleRendered>,
}

#[derive(Clone)]
struct ModulePath {
    segments: Vec<String>,
}

impl ModulePath {
    fn display_name_and_path(&self) -> (String, String) {
        let display_name = self.segments.join("::");
        let rel_path = format!("{}.md", self.segments.join("/"));
        (display_name, rel_path)
    }
}

#[derive(Clone)]
struct ItemDoc {
    name: String,
    kind: ItemKind,
    docs: String,
    signature: Option<String>,
    properties: Vec<PropertyDoc>,
    variants: Vec<VariantDoc>,
    impls: Vec<ImplDoc>,
}

#[derive(Clone)]
struct PropertyDoc {
    name: String,
    type_name: Option<String>,
    docs: String,
}

#[derive(Clone)]
struct VariantDoc {
    name: String,
    type_name: Option<String>,
    docs: String,
}

#[derive(Clone)]
struct ImplDoc {
    name: Option<String>,
    docs: String,
    items: Vec<ImplItemDoc>,
}

#[derive(Clone)]
struct ImplItemDoc {
    name: String,
    kind: ItemKind,
    signature: Option<String>,
    docs: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ItemKind {
    Module,
    Struct,
    Enum,
    Trait,
    Function,
    TypeAlias,
    Constant,
    Static,
    Macro,
    Reexport,
    Other(String),
}

impl ItemKind {
    fn label(&self) -> &str {
        match self {
            ItemKind::Struct => "Structs",
            ItemKind::Enum => "Enums",
            ItemKind::Trait => "Traits",
            ItemKind::Function => "Functions",
            ItemKind::TypeAlias => "Type Aliases",
            ItemKind::Constant => "Constants",
            ItemKind::Static => "Statics",
            ItemKind::Macro => "Macros",
            ItemKind::Reexport => "Re-exports",
            ItemKind::Other(_) => "Other",
            ItemKind::Module => "Modules",
        }
    }
}

#[derive(Clone)]
struct ApiSummaryEntry {
    title: String,
    path: String,
    depth: u8,
}

fn collect_modules(
    index: &serde_json::Map<String, Value>,
    item_ids: &[String],
    current_segments: Vec<String>,
    modules: &mut Vec<ModuleDoc>,
) {
    for item_id in item_ids {
        let item = match index.get(item_id) {
            Some(item) => item,
            None => continue,
        };
        if !is_public(item) {
            continue;
        }
        if item_kind(item) != ItemKind::Module {
            continue;
        }
        let name = item_name(item).unwrap_or_else(|| "module".to_string());
        let mut segments = current_segments.clone();
        segments.push(name);

        let module_items = match module_items(item) {
            Ok(items) => items,
            Err(_) => Vec::new(),
        };

        let submodules = collect_submodules(index, &module_items, segments.clone());
        let docs = extract_docs(item);
        let module_doc = ModuleDoc {
            path: Some(ModulePath { segments }),
            docs,
            items: module_items,
            submodules,
        };

        modules.push(module_doc);
    }
}

fn collect_submodules(
    index: &serde_json::Map<String, Value>,
    item_ids: &[String],
    current_segments: Vec<String>,
) -> Vec<ModuleDoc> {
    let mut submodules = Vec::new();
    collect_modules(index, item_ids, current_segments, &mut submodules);
    submodules
}

fn build_rendered_module(
    index: &serde_json::Map<String, Value>,
    module: &ModuleDoc,
    link_resolver: &LinkResolver,
) -> Option<ModuleRendered> {
    let path = module.path.clone()?;
    let items = filter_items(index, &module.items, link_resolver);
    let mut submodules = Vec::new();
    for submodule in &module.submodules {
        if let Some(rendered) = build_rendered_module(index, submodule, link_resolver) {
            submodules.push(rendered);
        }
    }

    if items.is_empty() && submodules.is_empty() {
        return None;
    }

    Some(ModuleRendered {
        path,
        docs: module.docs.clone(),
        items,
        submodules,
    })
}

fn module_sort_key_rendered(module: &ModuleRendered) -> String {
    module.path.segments.join("::")
}

fn flatten_rendered_modules(source: &[ModuleRendered], out: &mut Vec<ModuleRendered>) {
    for module in source {
        out.push(module.clone());
        flatten_rendered_modules(&module.submodules, out);
    }
}

fn module_items(item: &Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let module = item
        .get("inner")
        .and_then(|inner| inner.get("module"))
        .ok_or("item is not a module")?;
    let items = module
        .get("items")
        .and_then(Value::as_array)
        .ok_or("module items missing")?;

    Ok(items
        .iter()
        .filter_map(|id| id.as_i64().map(|id| id.to_string()))
        .collect())
}

fn filter_items(
    index: &serde_json::Map<String, Value>,
    item_ids: &[String],
    link_resolver: &LinkResolver,
) -> Vec<ItemDoc> {
    let mut items = Vec::new();
    for item_id in item_ids {
        let item = match index.get(item_id) {
            Some(item) => item,
            None => continue,
        };
        if !is_public(item) {
            continue;
        }
        let kind = item_kind(item);
        if kind == ItemKind::Module {
            continue;
        }
        let name = match item_name(item) {
            Some(name) => name,
            None => continue,
        };
        let docs = extract_docs(item);
        let signature = extract_signature(item, &name, &kind);
        let include_undocumented_struct_members = kind == ItemKind::Struct && !docs.is_empty();
        let properties = extract_properties(
            index,
            item,
            &kind,
            include_undocumented_struct_members,
            link_resolver,
        );
        let variants = extract_variants(index, item, &kind, link_resolver);
        let impls = extract_impls(index, item, &kind);
        let has_documented_variant = variants.iter().any(|variant| !variant.docs.is_empty());
        if docs.is_empty() && properties.is_empty() && impls.is_empty() && !has_documented_variant {
            continue;
        }
        items.push(ItemDoc {
            name,
            kind,
            docs,
            signature,
            properties,
            variants,
            impls,
        });
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

fn extract_properties(
    index: &serde_json::Map<String, Value>,
    item: &Value,
    kind: &ItemKind,
    include_undocumented: bool,
    link_resolver: &LinkResolver,
) -> Vec<PropertyDoc> {
    if *kind != ItemKind::Struct {
        return Vec::new();
    }
    let inner = match item.get("inner") {
        Some(inner) => inner,
        None => return Vec::new(),
    };
    let struct_obj = match inner.get("struct") {
        Some(struct_obj) => struct_obj,
        None => return Vec::new(),
    };

    let mut properties = Vec::new();
    if let Some(fields) = struct_field_ids(struct_obj) {
        for field_id in fields {
            let field_id = match field_id.as_i64() {
                Some(id) => id.to_string(),
                None => continue,
            };
            let field_item = match index.get(&field_id) {
                Some(item) => item,
                None => continue,
            };
            push_property(
                &mut properties,
                field_item,
                include_undocumented,
                link_resolver,
            );
        }
        return properties;
    }

    let (filename, start_line, end_line) = match span_lines(item) {
        Some(span) => span,
        None => return properties,
    };
    for field_item in index.values() {
        if !is_struct_field(field_item) {
            continue;
        }
        let (field_file, field_line, _) = match span_lines(field_item) {
            Some(span) => span,
            None => continue,
        };
        if field_file != filename {
            continue;
        }
        if field_line < start_line || field_line > end_line {
            continue;
        }
        push_property(
            &mut properties,
            field_item,
            include_undocumented,
            link_resolver,
        );
    }
    properties
}

fn extract_variants(
    index: &serde_json::Map<String, Value>,
    item: &Value,
    kind: &ItemKind,
    link_resolver: &LinkResolver,
) -> Vec<VariantDoc> {
    if *kind != ItemKind::Enum {
        return Vec::new();
    }
    let variants = item
        .get("inner")
        .and_then(|inner| inner.get("enum"))
        .and_then(|inner| inner.get("variants"))
        .and_then(Value::as_array);
    let Some(variants) = variants else {
        return Vec::new();
    };

    let mut docs = Vec::new();
    for variant_id in variants {
        let variant_id = match variant_id.as_i64() {
            Some(id) => id.to_string(),
            None => continue,
        };
        let variant_item = match index.get(&variant_id) {
            Some(item) => item,
            None => continue,
        };
        let name = match item_name(variant_item) {
            Some(name) => name,
            None => continue,
        };
        let variant_docs = extract_docs(variant_item);
        let type_name = enum_variant_type(index, variant_item, &name, link_resolver);
        docs.push(VariantDoc {
            name,
            type_name,
            docs: variant_docs,
        });
    }
    docs
}

fn extract_impls(
    index: &serde_json::Map<String, Value>,
    item: &Value,
    kind: &ItemKind,
) -> Vec<ImplDoc> {
    match kind {
        ItemKind::Struct | ItemKind::Enum | ItemKind::Trait => {}
        _ => return Vec::new(),
    }

    let impl_ids = match kind {
        ItemKind::Struct => item
            .get("inner")
            .and_then(|inner| inner.get("struct"))
            .and_then(|inner| inner.get("impls"))
            .and_then(Value::as_array),
        ItemKind::Enum => item
            .get("inner")
            .and_then(|inner| inner.get("enum"))
            .and_then(|inner| inner.get("impls"))
            .and_then(Value::as_array),
        ItemKind::Trait => item
            .get("inner")
            .and_then(|inner| inner.get("trait"))
            .and_then(|inner| inner.get("impls"))
            .and_then(Value::as_array),
        _ => None,
    };

    let mut impls = Vec::new();
    let Some(impl_ids) = impl_ids else {
        return impls;
    };

    for impl_id in impl_ids {
        let impl_id = match impl_id.as_i64() {
            Some(id) => id.to_string(),
            None => continue,
        };
        let impl_item = match index.get(&impl_id) {
            Some(item) => item,
            None => continue,
        };
        let impl_inner = match impl_item.get("inner").and_then(|inner| inner.get("impl")) {
            Some(inner) => inner,
            None => continue,
        };
        if impl_inner
            .get("is_synthetic")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || !impl_inner
                .get("blanket_impl")
                .unwrap_or(&Value::Null)
                .is_null()
        {
            continue;
        }

        let trait_name = impl_inner
            .get("trait")
            .and_then(|trait_ref| trait_ref.get("path"))
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        let name = trait_name.map(|trait_name| format!("impl {trait_name}"));

        let docs = extract_docs(impl_item);
        let items = extract_impl_items(index, impl_inner);

        if docs.is_empty() && items.is_empty() {
            continue;
        }

        impls.push(ImplDoc { name, docs, items });
    }

    impls.sort_by(|a, b| a.name.cmp(&b.name));
    impls
}

fn extract_impl_items(
    index: &serde_json::Map<String, Value>,
    impl_inner: &Value,
) -> Vec<ImplItemDoc> {
    let mut items = Vec::new();
    let Some(item_ids) = impl_inner.get("items").and_then(Value::as_array) else {
        return items;
    };
    for item_id in item_ids {
        let item_id = match item_id.as_i64() {
            Some(id) => id.to_string(),
            None => continue,
        };
        let item = match index.get(&item_id) {
            Some(item) => item,
            None => continue,
        };
        if !is_public_or_default(item) {
            continue;
        }
        let name = match item_name(item) {
            Some(name) => name,
            None => continue,
        };
        if name.starts_with('_') {
            continue;
        }
        let docs = extract_docs(item);
        let kind = item_kind(item);
        if docs.is_empty() {
            continue;
        }
        let signature = extract_signature(item, &name, &kind);
        items.push(ImplItemDoc {
            name,
            kind,
            signature,
            docs,
        });
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

fn push_property(
    properties: &mut Vec<PropertyDoc>,
    field_item: &Value,
    include_undocumented: bool,
    link_resolver: &LinkResolver,
) {
    if !is_public(field_item) {
        return;
    }
    let name = match item_name(field_item) {
        Some(name) => name,
        None => return,
    };
    if name.starts_with('_') {
        return;
    }
    let docs = extract_docs(field_item);
    if docs.is_empty() && !include_undocumented {
        return;
    }
    let type_name = field_type_name(field_item, link_resolver);
    properties.push(PropertyDoc {
        name,
        type_name,
        docs,
    });
}

fn struct_field_ids(struct_obj: &Value) -> Option<&Vec<Value>> {
    struct_obj
        .get("fields")
        .and_then(Value::as_array)
        .or_else(|| {
            struct_obj
                .get("kind")
                .and_then(|kind| kind.get("plain"))
                .and_then(|plain| plain.get("fields"))
                .and_then(Value::as_array)
        })
}

fn is_struct_field(item: &Value) -> bool {
    item.get("inner")
        .and_then(Value::as_object)
        .and_then(|inner| inner.keys().next())
        .map(|key| key == "struct_field")
        .unwrap_or(false)
}

fn is_public_or_default(item: &Value) -> bool {
    matches!(
        item.get("visibility").and_then(Value::as_str),
        Some("public") | Some("default")
    )
}

fn span_lines(item: &Value) -> Option<(String, u64, u64)> {
    let span = item.get("span")?;
    let filename = span.get("filename")?.as_str()?.to_string();
    let begin_line = span.get("begin")?.as_array()?.get(0)?.as_u64()?;
    let end_line = span.get("end")?.as_array()?.get(0)?.as_u64()?;
    Some((filename, begin_line, end_line))
}

fn item_kind(item: &Value) -> ItemKind {
    let inner = match item.get("inner") {
        Some(inner) => inner,
        None => return ItemKind::Other("unknown".to_string()),
    };
    let inner_obj = match inner.as_object() {
        Some(obj) => obj,
        None => return ItemKind::Other("unknown".to_string()),
    };
    let key = match inner_obj.keys().next() {
        Some(key) => key.as_str(),
        None => return ItemKind::Other("unknown".to_string()),
    };
    match key {
        "module" => ItemKind::Module,
        "struct" => ItemKind::Struct,
        "enum" => ItemKind::Enum,
        "trait" => ItemKind::Trait,
        "function" => ItemKind::Function,
        "type_alias" => ItemKind::TypeAlias,
        "constant" => ItemKind::Constant,
        "static" => ItemKind::Static,
        "macro" => ItemKind::Macro,
        "import" => ItemKind::Reexport,
        other => ItemKind::Other(other.to_string()),
    }
}

fn item_name(item: &Value) -> Option<String> {
    item.get("name")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

fn is_public(item: &Value) -> bool {
    matches!(
        item.get("visibility").and_then(Value::as_str),
        Some("public")
    )
}

fn extract_docs(item: &Value) -> String {
    item.get("docs")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn extract_signature(item: &Value, name: &str, kind: &ItemKind) -> Option<String> {
    if *kind != ItemKind::Function {
        return None;
    }
    let function = item.get("inner").and_then(|inner| inner.get("function"))?;
    Some(function_signature(item, function, name))
}

fn function_signature(item: &Value, function: &Value, name: &str) -> String {
    let mut out = String::new();
    if is_public(item) {
        out.push_str("pub ");
    }

    let header = function.get("header").unwrap_or(&Value::Null);
    if header
        .get("is_const")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        out.push_str("const ");
    }
    if header
        .get("is_async")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        out.push_str("async ");
    }
    if header
        .get("is_unsafe")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        out.push_str("unsafe ");
    }
    if let Some(abi) = header.get("abi").and_then(Value::as_str) {
        if abi != "Rust" {
            out.push_str(&format!("extern \"{abi}\" "));
        }
    }

    out.push_str("fn ");
    out.push_str(name);
    out.push_str(&generic_params_to_string(function.get("generics")));
    out.push('(');
    let sig = function.get("sig").unwrap_or(&Value::Null);
    if let Some(inputs) = sig.get("inputs").and_then(Value::as_array) {
        let args = inputs
            .iter()
            .filter_map(function_arg_to_string)
            .collect::<Vec<_>>();
        out.push_str(&args.join(", "));
    }
    if sig
        .get("is_c_variadic")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        if sig
            .get("inputs")
            .and_then(Value::as_array)
            .map(|inputs| !inputs.is_empty())
            .unwrap_or(false)
        {
            out.push_str(", ");
        }
        out.push_str("...");
    }
    out.push(')');
    if let Some(output) = sig.get("output") {
        if !output.is_null() {
            out.push_str(" -> ");
            out.push_str(&type_to_string(output));
        }
    }
    let where_clause = where_predicates_to_string(function.get("generics"));
    if !where_clause.is_empty() {
        out.push(' ');
        out.push_str(&where_clause);
    }
    out
}

fn function_arg_to_string(arg: &Value) -> Option<String> {
    let arg = arg.as_array()?;
    let name = arg.get(0)?.as_str()?;
    let ty = arg.get(1)?;
    if name == "self" {
        return Some(
            self_arg_to_string(ty).unwrap_or_else(|| format!("self: {}", type_to_string(ty))),
        );
    }
    Some(format!("{name}: {}", type_to_string(ty)))
}

fn self_arg_to_string(ty: &Value) -> Option<String> {
    if ty.get("generic").and_then(Value::as_str) == Some("Self") {
        return Some("self".to_string());
    }
    let borrowed_ref = ty.get("borrowed_ref")?;
    let inner = borrowed_ref.get("type")?;
    if inner.get("generic").and_then(Value::as_str) != Some("Self") {
        return None;
    }
    let mut out = String::from("&");
    if let Some(lifetime) = borrowed_ref.get("lifetime").and_then(Value::as_str) {
        out.push_str(lifetime);
        out.push(' ');
    }
    if borrowed_ref
        .get("is_mutable")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        out.push_str("mut ");
    }
    out.push_str("self");
    Some(out)
}

fn link_type_string(type_name: &str, link_resolver: &LinkResolver) -> String {
    let mut out = String::new();
    let mut chars = type_name.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if is_identifier_start(ch) {
            let start = idx;
            let mut end = idx + ch.len_utf8();
            while let Some((next_idx, next_ch)) = chars.peek().copied() {
                if is_identifier_continue(next_ch) {
                    chars.next();
                    end = next_idx + next_ch.len_utf8();
                } else {
                    break;
                }
            }
            let ident = &type_name[start..end];
            if let Some(target) = link_resolver.target_for_name(ident) {
                out.push_str(&format!("[{}]({target})", markdown_code(ident)));
            } else {
                out.push_str(&markdown_code(ident));
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn markdown_code(text: &str) -> String {
    format!("`{}`", text.replace('`', "\\`"))
}

fn field_type_name(field_item: &Value, link_resolver: &LinkResolver) -> Option<String> {
    field_item
        .get("inner")
        .and_then(|inner| inner.get("struct_field"))
        .map(|ty| link_type_string(&type_to_string(ty), link_resolver))
}

fn enum_variant_type(
    index: &serde_json::Map<String, Value>,
    variant_item: &Value,
    name: &str,
    link_resolver: &LinkResolver,
) -> Option<String> {
    let kind = variant_item
        .get("inner")
        .and_then(|inner| inner.get("variant"))
        .and_then(|variant| variant.get("kind"))?;

    if kind.as_str() == Some("plain") {
        return Some(markdown_code(name));
    }

    if let Some(tuple_fields) = kind.get("tuple").and_then(Value::as_array) {
        let fields = tuple_fields
            .iter()
            .filter_map(|field_id| field_id.as_i64())
            .filter_map(|field_id| index.get(&field_id.to_string()))
            .filter_map(|field_item| field_type_name(field_item, link_resolver))
            .collect::<Vec<_>>();
        return Some(format!("{}({})", markdown_code(name), fields.join(", ")));
    }

    if let Some(struct_variant) = kind.get("struct") {
        let fields = struct_variant
            .get("fields")
            .and_then(Value::as_array)?
            .iter()
            .filter_map(|field_id| field_id.as_i64())
            .filter_map(|field_id| index.get(&field_id.to_string()))
            .filter_map(|field_item| {
                let field_name = item_name(field_item)?;
                let type_name = field_type_name(field_item, link_resolver)?;
                Some(format!("{}: {type_name}", markdown_code(&field_name)))
            })
            .collect::<Vec<_>>();
        return Some(format!(
            "{} {{ {} }}",
            markdown_code(name),
            fields.join(", ")
        ));
    }

    None
}

fn type_to_string(ty: &Value) -> String {
    if let Some(path) = ty.get("resolved_path") {
        return path_to_string(path);
    }
    if let Some(generic) = ty.get("generic").and_then(Value::as_str) {
        return generic.to_string();
    }
    if let Some(primitive) = ty.get("primitive").and_then(Value::as_str) {
        return primitive.to_string();
    }
    if let Some(borrowed_ref) = ty.get("borrowed_ref") {
        let mut out = String::from("&");
        if let Some(lifetime) = borrowed_ref.get("lifetime").and_then(Value::as_str) {
            out.push_str(lifetime);
            out.push(' ');
        }
        if borrowed_ref
            .get("is_mutable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            out.push_str("mut ");
        }
        if let Some(inner) = borrowed_ref.get("type") {
            out.push_str(&type_to_string(inner));
        } else {
            out.push('_');
        }
        return out;
    }
    if let Some(raw_pointer) = ty.get("raw_pointer") {
        let mut out = if raw_pointer
            .get("is_mutable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "*mut ".to_string()
        } else {
            "*const ".to_string()
        };
        if let Some(inner) = raw_pointer.get("type") {
            out.push_str(&type_to_string(inner));
        } else {
            out.push('_');
        }
        return out;
    }
    if let Some(tuple) = ty.get("tuple").and_then(Value::as_array) {
        if tuple.is_empty() {
            return "()".to_string();
        }
        let items = tuple.iter().map(type_to_string).collect::<Vec<_>>();
        if items.len() == 1 {
            return format!("({},)", items[0]);
        }
        return format!("({})", items.join(", "));
    }
    if let Some(slice) = ty.get("slice") {
        return format!("[{}]", type_to_string(slice));
    }
    if let Some(array) = ty.get("array") {
        let inner = array
            .get("type")
            .map(type_to_string)
            .unwrap_or_else(|| "_".to_string());
        let len = array
            .get("len")
            .map(constant_to_string)
            .unwrap_or_else(|| "_".to_string());
        return format!("[{inner}; {len}]");
    }
    if let Some(dyn_trait) = ty.get("dyn_trait") {
        let mut bounds = trait_bounds_to_string(
            dyn_trait
                .get("traits")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        if let Some(lifetime) = dyn_trait.get("lifetime").and_then(Value::as_str) {
            if !bounds.is_empty() {
                bounds.push_str(" + ");
            }
            bounds.push_str(lifetime);
        }
        return format!("dyn {bounds}");
    }
    if let Some(function_pointer) = ty.get("function_pointer") {
        return function_pointer_to_string(function_pointer);
    }
    if let Some(qualified_path) = ty.get("qualified_path") {
        return qualified_path_to_string(qualified_path);
    }
    if let Some(bounds) = ty.get("impl_trait").and_then(Value::as_array) {
        return format!("impl {}", generic_bounds_to_string(bounds));
    }
    if ty.get("infer").is_some() {
        return "_".to_string();
    }
    "_".to_string()
}

fn path_to_string(path: &Value) -> String {
    let mut out = path
        .get("path")
        .and_then(Value::as_str)
        .map(display_path)
        .unwrap_or_else(|| "_".to_string());
    if let Some(args) = path.get("args") {
        out.push_str(&generic_args_to_string(args));
    }
    out
}

fn display_path(path: &str) -> String {
    path.trim_start_matches("$crate::")
        .trim_start_matches("crate::")
        .rsplit("::")
        .next()
        .unwrap_or(path)
        .to_string()
}

fn generic_args_to_string(args: &Value) -> String {
    if args.is_null() {
        return String::new();
    }
    if let Some(angle) = args.get("angle_bracketed") {
        let mut parts = Vec::new();
        if let Some(generic_args) = angle.get("args").and_then(Value::as_array) {
            parts.extend(generic_args.iter().filter_map(generic_arg_to_string));
        }
        if let Some(constraints) = angle.get("constraints").and_then(Value::as_array) {
            parts.extend(constraints.iter().filter_map(generic_constraint_to_string));
        }
        if parts.is_empty() {
            return String::new();
        }
        return format!("<{}>", parts.join(", "));
    }
    if let Some(parenthesized) = args.get("parenthesized") {
        let inputs = parenthesized
            .get("inputs")
            .and_then(Value::as_array)
            .map(|inputs| inputs.iter().map(type_to_string).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut out = format!("({})", inputs.join(", "));
        if let Some(output) = parenthesized.get("output") {
            if !output.is_null() {
                out.push_str(" -> ");
                out.push_str(&type_to_string(output));
            }
        }
        return out;
    }
    String::new()
}

fn generic_arg_to_string(arg: &Value) -> Option<String> {
    if let Some(ty) = arg.get("type") {
        return Some(type_to_string(ty));
    }
    if let Some(lifetime) = arg.get("lifetime").and_then(Value::as_str) {
        return Some(lifetime.to_string());
    }
    if let Some(constant) = arg.get("const") {
        return Some(constant_to_string(constant));
    }
    if arg.get("infer").is_some() {
        return Some("_".to_string());
    }
    None
}

fn generic_constraint_to_string(constraint: &Value) -> Option<String> {
    let name = constraint.get("name").and_then(Value::as_str)?;
    if let Some(term) = constraint.get("term") {
        return Some(format!("{name} = {}", term_to_string(term)));
    }
    if let Some(bounds) = constraint.get("bounds").and_then(Value::as_array) {
        return Some(format!("{name}: {}", generic_bounds_to_string(bounds)));
    }
    None
}

fn generic_params_to_string(generics: Option<&Value>) -> String {
    let Some(params) = generics
        .and_then(|generics| generics.get("params"))
        .and_then(Value::as_array)
    else {
        return String::new();
    };
    let rendered = params
        .iter()
        .filter(|param| !is_synthetic_generic_param(param))
        .filter_map(generic_param_to_string)
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        String::new()
    } else {
        format!("<{}>", rendered.join(", "))
    }
}

fn generic_param_to_string(param: &Value) -> Option<String> {
    let name = param.get("name").and_then(Value::as_str)?;
    let kind = param.get("kind")?;
    if let Some(type_param) = kind.get("type") {
        let bounds = type_param
            .get("bounds")
            .and_then(Value::as_array)
            .map(|bounds| generic_bounds_to_string(bounds))
            .unwrap_or_default();
        let mut out = name.to_string();
        if !bounds.is_empty() {
            out.push_str(": ");
            out.push_str(&bounds);
        }
        if let Some(default) = type_param.get("default") {
            if !default.is_null() {
                out.push_str(" = ");
                out.push_str(&type_to_string(default));
            }
        }
        return Some(out);
    }
    if let Some(lifetime) = kind.get("lifetime") {
        let outlives = lifetime
            .get("outlives")
            .and_then(Value::as_array)
            .map(|outlives| {
                outlives
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" + ")
            })
            .unwrap_or_default();
        let mut out = name.to_string();
        if !outlives.is_empty() {
            out.push_str(": ");
            out.push_str(&outlives);
        }
        return Some(out);
    }
    if let Some(const_param) = kind.get("const") {
        let type_name = const_param
            .get("type")
            .map(type_to_string)
            .unwrap_or_else(|| "_".to_string());
        let mut out = format!("const {name}: {type_name}");
        if let Some(default) = const_param.get("default") {
            if !default.is_null() {
                out.push_str(" = ");
                out.push_str(&constant_to_string(default));
            }
        }
        return Some(out);
    }
    Some(name.to_string())
}

fn is_synthetic_generic_param(param: &Value) -> bool {
    param
        .get("kind")
        .and_then(|kind| kind.get("type"))
        .and_then(|type_param| type_param.get("is_synthetic"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn where_predicates_to_string(generics: Option<&Value>) -> String {
    let Some(predicates) = generics
        .and_then(|generics| generics.get("where_predicates"))
        .and_then(Value::as_array)
    else {
        return String::new();
    };
    let rendered = predicates
        .iter()
        .filter_map(where_predicate_to_string)
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        String::new()
    } else {
        format!("where {}", rendered.join(", "))
    }
}

fn where_predicate_to_string(predicate: &Value) -> Option<String> {
    if let Some(bound) = predicate.get("bound_predicate") {
        let ty = bound.get("type").map(type_to_string)?;
        let bounds = bound
            .get("bounds")
            .and_then(Value::as_array)
            .map(|bounds| generic_bounds_to_string(bounds))
            .unwrap_or_default();
        if bounds.is_empty() {
            return Some(ty);
        }
        return Some(format!("{ty}: {bounds}"));
    }
    if let Some(lifetime) = predicate.get("lifetime_predicate") {
        let name = lifetime.get("lifetime").and_then(Value::as_str)?;
        let outlives = lifetime
            .get("outlives")
            .and_then(Value::as_array)?
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" + ");
        return Some(format!("{name}: {outlives}"));
    }
    if let Some(eq) = predicate.get("eq_predicate") {
        let lhs = eq.get("lhs").map(term_to_string)?;
        let rhs = eq.get("rhs").map(term_to_string)?;
        return Some(format!("{lhs} = {rhs}"));
    }
    None
}

fn generic_bounds_to_string(bounds: &[Value]) -> String {
    bounds
        .iter()
        .filter_map(generic_bound_to_string)
        .collect::<Vec<_>>()
        .join(" + ")
}

fn trait_bounds_to_string(bounds: &[Value]) -> String {
    bounds
        .iter()
        .filter_map(|bound| {
            bound
                .get("trait")
                .map(path_to_string)
                .or_else(|| generic_bound_to_string(bound))
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn generic_bound_to_string(bound: &Value) -> Option<String> {
    if let Some(trait_bound) = bound.get("trait_bound") {
        return trait_bound.get("trait").map(path_to_string);
    }
    if let Some(outlives) = bound.get("outlives").and_then(Value::as_str) {
        return Some(outlives.to_string());
    }
    if let Some(use_bound) = bound.get("use") {
        if let Some(params) = use_bound.as_array() {
            let params = params
                .iter()
                .filter_map(|param| {
                    param
                        .as_str()
                        .map(str::to_string)
                        .or_else(|| generic_arg_to_string(param))
                })
                .collect::<Vec<_>>();
            return Some(format!("use<{}>", params.join(", ")));
        }
    }
    None
}

fn term_to_string(term: &Value) -> String {
    if let Some(ty) = term.get("type") {
        return type_to_string(ty);
    }
    if let Some(constant) = term.get("constant") {
        return constant_to_string(constant);
    }
    if let Some(lifetime) = term.get("lifetime").and_then(Value::as_str) {
        return lifetime.to_string();
    }
    type_to_string(term)
}

fn constant_to_string(constant: &Value) -> String {
    if let Some(value) = constant.get("expr").and_then(Value::as_str) {
        return value.to_string();
    }
    if let Some(value) = constant.get("value").and_then(Value::as_str) {
        return value.to_string();
    }
    if let Some(value) = constant.as_str() {
        return value.to_string();
    }
    if let Some(value) = constant.as_i64() {
        return value.to_string();
    }
    "_".to_string()
}

fn function_pointer_to_string(function_pointer: &Value) -> String {
    let mut out = String::new();
    let header = function_pointer.get("header").unwrap_or(&Value::Null);
    if header
        .get("is_unsafe")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        out.push_str("unsafe ");
    }
    if let Some(abi) = header.get("abi").and_then(Value::as_str) {
        if abi != "Rust" {
            out.push_str(&format!("extern \"{abi}\" "));
        }
    }
    out.push_str("fn");
    out.push_str(&generic_params_to_string(
        function_pointer.get("generic_params"),
    ));
    let decl = function_pointer.get("decl").unwrap_or(&Value::Null);
    let inputs = decl
        .get("inputs")
        .and_then(Value::as_array)
        .map(|inputs| inputs.iter().map(type_to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    out.push('(');
    out.push_str(&inputs.join(", "));
    out.push(')');
    if let Some(output) = decl.get("output") {
        if !output.is_null() {
            out.push_str(" -> ");
            out.push_str(&type_to_string(output));
        }
    }
    out
}

fn qualified_path_to_string(qualified_path: &Value) -> String {
    let name = qualified_path
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("_");
    let self_type = qualified_path
        .get("self_type")
        .map(type_to_string)
        .unwrap_or_else(|| "_".to_string());
    let mut out = format!("{self_type}::{name}");
    if let Some(args) = qualified_path.get("args") {
        out.push_str(&generic_args_to_string(args));
    }
    out
}

fn summary_from_docs(docs: &str) -> Option<String> {
    for line in docs.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        return Some(trimmed.to_string());
    }
    None
}

fn write_module_doc(
    path: &Path,
    title: &str,
    docs: &str,
    summary: &str,
    crate_name: &str,
    submodules: &[ModuleRendered],
    items: &[ItemDoc],
    link_resolver: &LinkResolver,
) -> Result<bool, Box<dyn std::error::Error>> {
    if docs.is_empty() && submodules.is_empty() && items.is_empty() {
        return Ok(false);
    }
    let mut out = String::new();
    out.push_str(&format!("# {title}\n"));
    out.push_str(&format!("<!-- summary: {summary} -->\n"));
    out.push_str(&format!("<!-- tags: api, {crate_name} -->\n\n"));

    if !docs.is_empty() {
        out.push_str(docs);
        out.push_str("\n\n");
    }

    if !submodules.is_empty() {
        out.push_str("## Submodules\n");
        for module in submodules {
            let (display_name, rel_path) = module.path.display_name_and_path();
            out.push_str(&format!("- [{display_name}]({rel_path})\n"));
        }
        out.push('\n');
    }

    let mut grouped: BTreeMap<ItemKind, Vec<&ItemDoc>> = BTreeMap::new();
    for item in items {
        grouped.entry(item.kind.clone()).or_default().push(item);
    }

    for (kind, items) in grouped {
        if items.is_empty() {
            continue;
        }
        out.push_str(&format!("## {}\n", kind.label()));
        for (idx, item) in items.iter().enumerate() {
            if item.kind == ItemKind::Function {
                push_function_entry(
                    &mut out,
                    3,
                    &item.name,
                    item.signature.as_deref(),
                    link_resolver,
                );
            } else {
                out.push_str(&format!("### `{}`\n", item.name));
                if let Some(signature) = &item.signature {
                    push_linked_signature_block(&mut out, signature, link_resolver);
                }
            }

            if !item.docs.is_empty() {
                out.push_str(&item.docs);
                out.push_str("\n\n");
            }
            if !item.properties.is_empty() {
                out.push_str("#### Properties\n");
                for property in &item.properties {
                    out.push_str(&format!("##### `{}`\n", property.name));
                    if let Some(type_name) = &property.type_name {
                        out.push_str(&format!("Type: {type_name}\n\n"));
                    }
                    if !property.docs.is_empty() {
                        out.push_str(&property.docs);
                        out.push_str("\n\n");
                    }
                }
            }
            if !item.variants.is_empty() {
                out.push_str("#### Variants\n");
                for variant in &item.variants {
                    let heading = variant.type_name.as_deref().unwrap_or(&variant.name);
                    out.push_str(&format!("##### {heading}\n"));
                    if !variant.docs.is_empty() {
                        out.push_str(&variant.docs);
                        out.push_str("\n\n");
                    }
                }
            }
            if !item.impls.is_empty() {
                out.push_str("#### Implementations\n");
                for impl_doc in &item.impls {
                    if let Some(name) = &impl_doc.name {
                        out.push_str(&format!("##### `{name}`\n"));
                    }
                    if !impl_doc.docs.is_empty() {
                        out.push_str(&impl_doc.docs);
                        out.push_str("\n\n");
                    }
                    for impl_item in &impl_doc.items {
                        if impl_item.kind == ItemKind::Function {
                            push_function_entry(
                                &mut out,
                                5,
                                &impl_item.name,
                                impl_item.signature.as_deref(),
                                link_resolver,
                            );
                        } else {
                            out.push_str(&format!("###### `{}`\n", impl_item_heading(impl_item)));
                            if let Some(signature) = &impl_item.signature {
                                push_linked_signature_block(&mut out, signature, link_resolver);
                            }
                        }
                        if !impl_item.docs.is_empty() {
                            out.push_str(&impl_item.docs);
                            out.push_str("\n\n");
                        }
                    }
                }
            }
            if idx + 1 < items.len() {
                out.push_str("---\n\n");
            }
        }
    }

    write_if_changed(path, &out)?;
    Ok(true)
}

fn push_function_entry(
    out: &mut String,
    heading_level: usize,
    name: &str,
    signature: Option<&str>,
    link_resolver: &LinkResolver,
) {
    out.push_str(&format!("{} `{name}`\n", "#".repeat(heading_level)));
    if let Some(signature) = signature {
        push_linked_signature_block(out, signature, link_resolver);
    }
}

fn push_linked_signature_block(out: &mut String, code: &str, link_resolver: &LinkResolver) {
    out.push_str("<pre><code class=\"api-signature language-rust ignore\">");
    out.push_str(&linked_signature_html(code, link_resolver));
    out.push_str("</code></pre>\n\n");
}

fn linked_signature_html(code: &str, link_resolver: &LinkResolver) -> String {
    let mut out = String::new();
    let mut last_idx = 0usize;
    let mut chars = code.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if !is_identifier_start(ch) {
            continue;
        }

        html_escape_into(&mut out, &code[last_idx..idx]);
        let start = idx;
        let mut end = idx + ch.len_utf8();
        while let Some((next_idx, next_ch)) = chars.peek().copied() {
            if is_identifier_continue(next_ch) {
                chars.next();
                end = next_idx + next_ch.len_utf8();
            } else {
                break;
            }
        }

        let ident = &code[start..end];
        if let Some(target) = link_resolver.target_for_name(ident) {
            out.push_str("<a href=\"");
            html_escape_into(&mut out, target);
            out.push_str("\">");
            html_escape_into(&mut out, ident);
            out.push_str("</a>");
        } else {
            html_escape_into(&mut out, ident);
        }
        last_idx = end;
    }
    html_escape_into(&mut out, &code[last_idx..]);
    out
}

fn html_escape_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

fn impl_item_heading(item: &ImplItemDoc) -> String {
    match item.kind {
        ItemKind::Function => format!("fn {}", item.name),
        ItemKind::Constant => format!("const {}", item.name),
        ItemKind::TypeAlias => format!("type {}", item.name),
        ItemKind::Static => format!("static {}", item.name),
        ItemKind::Macro => format!("macro {}", item.name),
        _ => item.name.clone(),
    }
}

fn write_api_index(
    api_dir: &Path,
    public_crates: &[String],
    internal_crates: &[String],
    generated_files: &mut BTreeSet<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = include_str!("../../templates/api-index.md")
        .trim_end()
        .to_string();
    out.push_str("\n\n");
    for krate in public_crates {
        out.push_str(&format!("- [{krate}]({krate}/index.md)\n"));
    }
    if !internal_crates.is_empty() {
        out.push('\n');
        out.push_str("<a id=\"internal-crates\"></a>\n\n");
        out.push_str("## Engine and maintainer APIs\n\n");
        out.push_str("For primitive implementations and work on Pax itself, continue with the\n");
        out.push_str("[Maintainer Reference](internal/index.md). It separates current internal\n");
        out.push_str("APIs from historical architecture and design notes.\n");
    }
    let index_path = api_dir.join("index.md");
    write_if_changed(&index_path, &out)?;
    record_generated_file(generated_files, &index_path);

    if !internal_crates.is_empty() {
        let mut internal_out = include_str!("../../templates/internal-api-index.md")
            .trim_end()
            .to_string();
        internal_out.push_str("\n\n");
        for krate in internal_crates {
            internal_out.push_str(&format!("- [{krate}]({krate}/index.md)\n"));
        }
        let internal_index = api_dir.join("internal").join("index.md");
        write_if_changed(&internal_index, &internal_out)?;
        record_generated_file(generated_files, &internal_index);
    }
    Ok(())
}

fn update_summary(
    summary_path: &Path,
    public_entries: &[ApiSummaryEntry],
    internal_entries: &[ApiSummaryEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(summary_path)?;
    let mut lines = contents
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    let api_line_index = lines
        .iter()
        .position(|line| line.contains("(api/index.md)"))
        .ok_or("SUMMARY.md missing api/index.md entry")?;

    let mut start_idx = None;
    let mut end_idx = None;
    for (idx, line) in lines.iter().enumerate() {
        if line.contains("<!-- API-START -->") {
            start_idx = Some(idx);
        }
        if line.contains("<!-- API-END -->") {
            end_idx = Some(idx);
        }
    }

    let block = build_api_summary_block(public_entries, internal_entries);
    if let (Some(start), Some(end)) = (start_idx, end_idx) {
        lines.splice(start + 1..end, block);
    } else {
        let insert_at = api_line_index + 1;
        let mut new_block = Vec::new();
        new_block.push("  <!-- API-START -->".to_string());
        new_block.extend(block);
        new_block.push("  <!-- API-END -->".to_string());
        lines.splice(insert_at..insert_at, new_block);
    }

    write_if_changed(summary_path, &(lines.join("\n") + "\n"))?;
    Ok(())
}

fn build_api_summary_block(
    public_entries: &[ApiSummaryEntry],
    internal_entries: &[ApiSummaryEntry],
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut seen = BTreeSet::new();

    for entry in public_entries {
        if !seen.insert(entry.path.clone()) {
            continue;
        }
        let indent = "  ".repeat(entry.depth as usize);
        lines.push(format!("{indent}- [{}]({})", entry.title, entry.path));
    }

    if !internal_entries.is_empty() {
        lines.push(String::new());
        lines.push("- [Maintainer Reference](api/internal/index.md)".to_string());
        lines.push(String::new());
        for entry in internal_entries {
            let internal_path = entry.path.clone();
            if !seen.insert(internal_path) {
                continue;
            }
            let indent = "  ".repeat(entry.depth as usize);
            lines.push(format!("{indent}- [{}]({})", entry.title, entry.path));
        }
        // Keep one contiguous child list: a second list after API-END can
        // replace the first in mdBook's SUMMARY parser.
        lines.push(
            "  - [Runtime & Cartridge Notes (historical)](architecture-runtime-cartridge.md)"
                .to_string(),
        );
    }
    lines
}

fn record_generated_file(generated_files: &mut BTreeSet<PathBuf>, path: &Path) {
    let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    generated_files.insert(normalized);
}

fn write_if_changed(path: &Path, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    let contents = format!("{}\n", contents.trim_end());
    if let Ok(existing) = fs::read_to_string(path) {
        if existing == contents {
            return Ok(());
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn cleanup_stale_api_files(
    api_dir: &Path,
    generated_files: &BTreeSet<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut md_files = Vec::new();
    collect_md_files(api_dir, &mut md_files)?;
    for file_path in md_files {
        let normalized = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        if !generated_files.contains(&normalized) {
            fs::remove_file(&file_path)?;
        }
    }
    Ok(())
}

fn prune_empty_dirs(dir: &Path, is_root: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    let mut has_contents = false;
    for entry in &entries {
        let path = entry.path();
        if path.is_dir() {
            let empty = prune_empty_dirs(&path, false)?;
            if empty {
                fs::remove_dir(&path)?;
            } else {
                has_contents = true;
            }
        } else {
            has_contents = true;
        }
    }
    if is_root {
        return Ok(false);
    }
    Ok(!has_contents)
}

fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "pax-api-index-test-{}-{nonce}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn indexes_include_orientation_and_preserve_anchors() {
        let dir = TestDirectory::new();
        let mut generated = BTreeSet::new();
        write_api_index(
            &dir.0,
            &["pax-runtime-api".into(), "pax-std".into()],
            &["pax-runtime".into()],
            &mut generated,
        )
        .unwrap();
        let public = fs::read_to_string(dir.0.join("index.md")).unwrap();
        let internal = fs::read_to_string(dir.0.join("internal/index.md")).unwrap();
        assert!(public.contains("# API Reference"));
        assert!(public.contains("id=\"api-docs\""));
        assert!(public.contains("## Public crates"));
        assert!(public.contains("id=\"internal-crates\""));
        assert!(public.contains("\n\n- [pax-runtime-api](pax-runtime-api/index.md)"));
        assert!(public.contains("[Maintainer Reference](internal/index.md)"));
        assert!(internal.contains("# Maintainer Reference"));
        assert!(internal.contains("id=\"internal-api-docs\""));
        assert!(internal.contains("../../primitives.md"));
        assert!(internal.contains("- [pax-runtime](pax-runtime/index.md)"));
        assert_eq!(generated.len(), 2);
    }

    #[test]
    fn public_only_index_does_not_link_missing_internal_index() {
        let dir = TestDirectory::new();
        let mut generated = BTreeSet::new();
        write_api_index(&dir.0, &["pax-std".into()], &[], &mut generated).unwrap();
        let public = fs::read_to_string(dir.0.join("index.md")).unwrap();
        assert!(!public.contains("(internal/index.md)"));
        assert!(!dir.0.join("internal/index.md").exists());
        assert_eq!(generated.len(), 1);
    }

    #[test]
    fn summary_keeps_maintainer_reference_separate_and_regenerates_idempotently() {
        let dir = TestDirectory::new();
        let summary = dir.0.join("SUMMARY.md");
        fs::write(
            &summary,
            "# Summary\n\n- [Getting Started](getting-started.md)\n- [API Reference](api/index.md)\n  <!-- API-START -->\n  <!-- API-END -->\n\n# Further reading\n\n[Team notes](notes.md)\n",
        )
        .unwrap();
        let public = vec![ApiSummaryEntry {
            title: "pax-std".into(),
            path: "api/pax-std/index.md".into(),
            depth: 1,
        }];
        let internal = vec![
            ApiSummaryEntry {
                title: "pax-runtime".into(),
                path: "api/internal/pax-runtime/index.md".into(),
                depth: 1,
            },
            ApiSummaryEntry {
                title: "rendering".into(),
                path: "api/internal/pax-runtime/rendering.md".into(),
                depth: 2,
            },
        ];
        update_summary(&summary, &public, &internal).unwrap();
        let first = fs::read_to_string(&summary).unwrap();
        assert!(first.contains("- [Getting Started](getting-started.md)"));
        assert!(first
            .contains("\n- [Maintainer Reference](api/internal/index.md)\n\n  - [pax-runtime]"));
        assert!(first.contains("\n    - [rendering](api/internal/pax-runtime/rendering.md)"));
        assert!(first.contains("  - [Runtime & Cartridge Notes (historical)](architecture-runtime-cartridge.md)\n  <!-- API-END -->"));
        assert!(first.ends_with("# Further reading\n\n[Team notes](notes.md)\n"));
        update_summary(&summary, &public, &internal).unwrap();
        assert_eq!(first, fs::read_to_string(summary).unwrap());
    }
}
