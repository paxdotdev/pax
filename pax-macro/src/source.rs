//! Compiler-selected source discovery shared by components and runnable roots.
extern crate proc_macro;
use color_eyre::eyre::{eyre, Result};
use pax_codegen::configured_source::{
    self as source, Facts, NeedsPredicate, NeedsSource, SourceCache, Sources,
};
use quote::ToTokens;
use sailfish::TemplateOnce;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use syn::spanned::Spanned;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct Site {
    file: PathBuf,
    line: usize,
    column: usize,
}
#[derive(Clone, Serialize, Deserialize)]
struct Snapshot {
    entry: PathBuf,
    facts: Facts,
}
#[derive(Clone, Serialize, Deserialize)]
struct Request {
    site: Site,
    runnable: bool,
    attributes: Vec<String>,
    engine: String,
    snapshot: Snapshot,
    dependencies: Vec<Snapshot>,
}
#[derive(Default)]
struct Session {
    source: SourceCache,
    facts: Facts,
    metadata: Option<cargo_metadata::Metadata>,
    package_metadata: Option<cargo_metadata::Metadata>,
    programs: BTreeMap<String, String>,
    discoveries: BTreeMap<String, Sources>,
}
thread_local! { static SESSION: RefCell<Session> = RefCell::new(Session::default()); }

fn compiler_site(span: proc_macro::Span) -> Site {
    Site {
        file: span.local_file().unwrap().canonicalize().unwrap(),
        line: span.line(),
        column: span.column() - 1,
    }
}

fn with_source(
    anchor: proc_macro::Span,
    f: impl FnOnce(&mut Session) -> Result<String>,
) -> proc_macro::TokenStream {
    // Never retain compiler tokens across the source-parser backend switch.
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            proc_macro2::fallback::unforce();
        }
    }
    let result = {
        proc_macro2::fallback::force();
        let _restore = Restore;
        SESSION.with(|s| f(&mut s.borrow_mut()))
    };
    match result {
        Ok(source) => anchor_tokens(source.parse().unwrap(), anchor),
        Err(e) => format!("compile_error!({:?});", format!("{e:?}"))
            .parse()
            .unwrap(),
    }
}

fn package_metadata(session: &mut Session) -> Result<&cargo_metadata::Metadata> {
    if session.package_metadata.is_none() {
        session.package_metadata = Some(
            cargo_metadata::MetadataCommand::new()
                .manifest_path(
                    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
                        .join("Cargo.toml"),
                )
                .no_deps()
                .other_options(vec!["--offline".into()])
                .exec()?,
        );
    }
    Ok(session.package_metadata.as_ref().unwrap())
}

fn package(session: &mut Session) -> Result<cargo_metadata::Package> {
    let manifest =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
    package_metadata(session)?
        .packages
        .iter()
        .find(|p| p.manifest_path.as_std_path() == manifest)
        .cloned()
        .ok_or_else(|| eyre!("Current Cargo package is missing from metadata"))
}

fn metadata<'a>(session: &'a mut Session, facts: &Facts) -> Result<&'a cargo_metadata::Metadata> {
    if session.metadata.is_none() {
        let selected_features = package(session)?
            .features
            .keys()
            .filter(|name| facts.get(&format!("feature = {name:?}")) == Some(&true))
            .cloned()
            .collect::<Vec<_>>();
        session.metadata = Some(
            cargo_metadata::MetadataCommand::new()
                .features(cargo_metadata::CargoOpt::NoDefaultFeatures)
                .features(cargo_metadata::CargoOpt::SomeFeatures(selected_features))
                .manifest_path(
                    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
                        .join("Cargo.toml"),
                )
                .other_options(vec!["--offline".into()])
                .exec()?,
        );
    }
    Ok(session.metadata.as_ref().unwrap())
}

fn begin(
    anchor: proc_macro::Span,
    runnable: bool,
    engine: String,
    attributes: Vec<String>,
) -> proc_macro::TokenStream {
    let site = compiler_site(anchor);
    with_source(anchor, |session| {
        let manifest =
            PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
        let package = package_metadata(session)?
            .packages
            .iter()
            .find(|p| p.manifest_path.as_std_path() == manifest)
            .unwrap();
        let target_name = std::env::var("CARGO_CRATE_NAME")?;
        let target = package
            .targets
            .iter()
            .find(|t| t.name.replace('-', "_") == target_name)
            .ok_or_else(|| eyre!("No target {target_name}"))?;
        drive(
            Request {
                site,
                runnable,
                attributes,
                engine,
                snapshot: Snapshot {
                    entry: target.src_path.clone().into(),
                    facts: Facts::new(),
                },
                dependencies: Vec::new(),
            },
            session,
        )
    })
}

pub fn pax(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let anchor = input.ident.span().unwrap();
    let engine = input
        .attrs
        .iter()
        .find(|a| a.path.is_ident("engine_import_path"))
        .and_then(|a| a.parse_args::<syn::LitStr>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "pax_kit::pax_engine".into());
    let attributes = input
        .attrs
        .iter()
        .map(|a| a.path.to_token_stream().to_string())
        .collect();
    begin(anchor, false, engine, attributes)
}
pub fn runnable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let anchor = input.clone().into_iter().next().unwrap().span();
    let declaration: pax_manifest::runnable::RunnableDeclaration = syn::parse(input).unwrap();
    let engine = declaration
        .engine_import_path
        .map(|p| p.to_token_stream().to_string())
        .unwrap_or_else(|| "pax_kit::pax_engine".into());
    begin(anchor, true, engine, Vec::new())
}

pub fn resume(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::ExprTuple = syn::parse(input).unwrap();
    fn literal(e: &syn::Expr) -> String {
        match e {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) => s.value(),
            syn::Expr::Group(g) => literal(&g.expr),
            _ => panic!("literal protocol"),
        }
    }
    let anchor = input.elems.first().unwrap().span().unwrap();
    let strings: Vec<String> = input.elems.iter().skip(1).map(literal).collect();
    let mut request: Request = serde_json::from_str(&strings[0]).unwrap();
    match strings[1].as_str() {
        "predicate" => {
            request
                .snapshot
                .facts
                .insert(strings[2].clone(), strings[3] == "true");
        }
        "dependency" => {
            let snapshots: Vec<Snapshot> = serde_json::from_str(&strings[2]).unwrap();
            for snapshot in snapshots {
                if let Some(existing) = request
                    .dependencies
                    .iter()
                    .find(|s| s.entry == snapshot.entry)
                {
                    assert_eq!(
                        existing.facts, snapshot.facts,
                        "conflicting configuration for a dependency source"
                    );
                } else {
                    request.dependencies.push(snapshot);
                }
            }
        }
        _ => panic!("unknown message"),
    }
    with_source(anchor, |session| drive(request, session))
}

fn sites(sources: &Sources, runnable: bool) -> BTreeMap<Site, syn::Item> {
    fn visit(
        items: &[syn::Item],
        file: &Path,
        runnable: bool,
        result: &mut BTreeMap<Site, syn::Item>,
    ) {
        for item in items {
            let span = match item {
                syn::Item::Struct(x)
                    if !runnable
                        && x.attrs.iter().any(|a| {
                            a.path
                                .segments
                                .last()
                                .is_some_and(|segment| segment.ident == "pax")
                        }) =>
                {
                    Some(x.ident.span())
                }
                syn::Item::Enum(x)
                    if !runnable
                        && x.attrs.iter().any(|a| {
                            a.path
                                .segments
                                .last()
                                .is_some_and(|segment| segment.ident == "pax")
                        }) =>
                {
                    Some(x.ident.span())
                }
                syn::Item::Struct(x)
                    if runnable && x.attrs.iter().any(|a| a.path.is_ident("main")) =>
                {
                    Some(x.ident.span())
                }
                syn::Item::Enum(x)
                    if runnable && x.attrs.iter().any(|a| a.path.is_ident("main")) =>
                {
                    Some(x.ident.span())
                }
                syn::Item::Macro(x)
                    if runnable && x.mac.path.segments.last().unwrap().ident == "runnable" =>
                {
                    Some(x.mac.tokens.span())
                }
                _ => None,
            };
            if let Some(span) = span {
                let pos = span.start();
                result.insert(
                    Site {
                        file: file.to_owned(),
                        line: pos.line,
                        column: pos.column,
                    },
                    item.clone(),
                );
            }
            if let syn::Item::Mod(x) = item {
                if let Some((_, items)) = &x.content {
                    visit(items, file, runnable, result);
                }
            }
        }
    }
    let mut result = BTreeMap::new();
    for (file, parsed) in sources {
        visit(&parsed.items, file, runnable, &mut result);
    }
    result
}

fn discover(snapshot: &Snapshot, session: &mut Session) -> Result<Sources> {
    let key = serde_json::to_string(snapshot)?;
    if let Some(sources) = session.discoveries.get(&key) {
        return Ok(sources.clone());
    }
    let sources = source::discover(&snapshot.entry, &snapshot.facts, &mut session.source)?;
    session.discoveries.insert(key, sources.clone());
    Ok(sources)
}

fn drive(mut request: Request, session: &mut Session) -> Result<String> {
    for (predicate, enabled) in &request.snapshot.facts {
        session.facts.insert(predicate.clone(), *enabled);
    }
    request.snapshot.facts.extend(session.facts.clone());
    let callback = format!("{}::pax_macro::__pax_resume", request.engine);
    let expand = format!("{}::pax_macro::__pax_expand", request.engine);
    let mut predicates = package(session)?
        .features
        .keys()
        .map(|name| format!("feature = {name:?}"))
        .collect::<Vec<_>>();
    predicates.extend(
        [
            "target_arch = \"wasm32\"",
            "target_os = \"macos\"",
            "target_os = \"ios\"",
            "debug_assertions",
            "test",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    for predicate in &predicates {
        if !request.snapshot.facts.contains_key(predicate) {
            let json = serde_json::to_string(&request)?;
            return Ok(format!(
                r#"
                #[cfg({predicate})] {callback}!((__pax_anchor, {json:?}, "predicate", {predicate:?}, "true"));
                #[cfg(not({predicate}))] {callback}!((__pax_anchor, {json:?}, "predicate", {predicate:?}, "false"));
            "#
            ));
        }
    }
    let sources = match discover(&request.snapshot, session) {
        Ok(sources) => sources,
        Err(e) => {
            if let Some(NeedsPredicate(predicate)) = e.downcast_ref::<NeedsPredicate>() {
                let json = serde_json::to_string(&request)?;
                return Ok(format!(
                    r#"
                    #[cfg({predicate})] {callback}!((__pax_anchor, {json:?}, "predicate", {predicate:?}, "true"));
                    #[cfg(not({predicate}))] {callback}!((__pax_anchor, {json:?}, "predicate", {predicate:?}, "false"));
                "#
                ));
            }
            return Err(e);
        }
    };
    let mut output = String::new();
    let components = sites(&sources, false);
    let runnables = sites(&sources, true);
    let legacy = runnables
        .get(&request.site)
        .is_some_and(|item| !matches!(item, syn::Item::Macro(_)));
    if runnables.len() > 1
        && runnables
            .values()
            .any(|item| !matches!(item, syn::Item::Macro(_)))
    {
        return Err(eyre!(
            "Use runnable! for multiple roots; do not mix it with legacy #[main]"
        ));
    }
    let emitter = components
        .keys()
        .next()
        .or_else(|| runnables.keys().next())
        .unwrap();
    if &request.site == emitter {
        let mut library_sources = sources.clone();
        for dependency in &request.dependencies {
            library_sources.extend(discover(dependency, session)?);
        }
        let metadata = metadata(session, &request.snapshot.facts)?.clone();
        let project = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        if let Err(error) = pax_codegen::static_analysis::resolve_library_sources(
            &project,
            &metadata,
            library_sources,
        ) {
            return request_dependency(&request, &metadata, error);
        }
    }
    if !request.runnable {
        let mut item = components
            .get(&request.site)
            .ok_or_else(|| {
                eyre!(
                    "Missing component site {:?}; candidates {:?}",
                    request.site,
                    components.keys()
                )
            })?
            .clone();
        let attrs = match &mut item {
            syn::Item::Struct(x) => &mut x.attrs,
            syn::Item::Enum(x) => &mut x.attrs,
            _ => unreachable!(),
        };
        // Attributes before #[pax] may already have expanded (notably derive).
        // Replaying them from disk would produce duplicate implementations.
        attrs.retain(|a| {
            !a.path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "pax")
                && !a.path.is_ident("main")
                && !(legacy && a.path.is_ident("init"))
                && request
                    .attributes
                    .contains(&a.path.to_token_stream().to_string())
        });
        output.push_str(&format!("#[{expand}] {}", item.to_token_stream()));
    }
    if request.runnable || legacy {
        let item = runnables
            .get(&request.site)
            .ok_or_else(|| eyre!("Runnable is not in the selected source graph"))?;
        let declaration = runnable_declaration(item)?;
        let root = declaration.root.to_token_stream().to_string();
        let initializer = declaration
            .init
            .map(|p| p.to_token_stream().to_string())
            .unwrap_or_else(|| format!("|_| Ok(<{root}>::default())"));
        let name = initializer_name(&request.site);
        let engine = &request.engine;
        output.push_str(&format!(r#"
            #[doc(hidden)] pub(crate) fn {name}(ctx: &{engine}::api::InitContext) -> {engine}::api::AppResult<{engine}::api::pax_value::PaxAny> {{
                let result: {engine}::api::AppResult<{root}> = ({initializer})(ctx);
                let root = result?;
                Ok({engine}::api::pax_value::ToFromPaxAny::to_pax_any(root))
            }}
        "#));
    }
    if &request.site == emitter {
        let mut snapshots = vec![request.snapshot.clone()];
        snapshots.extend(request.dependencies.clone());
        let json = serde_json::to_string(&snapshots)?;
        let helper = helper_name(&request.snapshot.entry, "source_config");
        output.push_str(&format!(r#"
            #[doc(hidden)] #[macro_export]
            macro_rules! {helper} {{
                ($callback:path, $anchor:ident, $state:literal) => {{ $callback!(($anchor, $state, "dependency", {json:?})); }};
            }}
        "#));
        for path in sources.keys() {
            output.push_str(&format!("const _: &[u8] = include_bytes!({path:?});"));
        }
    }
    if (!request.runnable && !legacy) || Some(&request.site) != runnables.keys().next() {
        return Ok(output);
    }
    let key = serde_json::to_string(&(request.snapshot.clone(), request.dependencies.clone()))?;
    if let Some(program) = session.programs.get(&key) {
        output.push_str(program);
        return Ok(output);
    }
    let mut all_sources = sources;
    // The legacy attribute is a single default-initialized runnable. Normalize
    // only this crate's selected AST, preserving its original source spans.
    for file in all_sources.values_mut() {
        normalize_legacy(&mut file.items)?;
    }
    for dependency in &request.dependencies {
        all_sources.extend(discover(dependency, session)?);
    }
    let project = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let metadata = metadata(session, &request.snapshot.facts)?.clone();
    let catalog = match pax_codegen::static_analysis::build_program_catalog_from_sources(
        &project,
        &metadata,
        pax_codegen::static_analysis::BuildManifestOptions {
            is_designtime: crate::designtime_enabled(),
        },
        all_sources,
    ) {
        Ok(catalog) => catalog,
        Err(e) => {
            return request_dependency(&request, &metadata, e);
        }
    };
    let manifest = catalog.manifest_for(Some(&catalog.entries()[0].root))?;
    for name in [
        "PAX_DIR",
        "PAX_MANIFEST_OUTPUT",
        "PAX_CARTRIDGE_MANIFEST_DIR",
        "PAX_BUILD_TARGET",
        "PAX_BUILD_DESIGNTIME",
    ] {
        output.push_str(&format!("const _: Option<&str> = option_env!({name:?});"));
    }
    if crate::is_root_crate() {
        if let Some(path) = std::env::var_os("PAX_MANIFEST_OUTPUT") {
            let path = PathBuf::from(path);
            let bytes = manifest_bytes(&manifest)?;
            if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
                std::fs::write(&path, bytes)?;
            }
            // Removing the sidecar invalidates Cargo's dependency fingerprint,
            // so a warm packaging build recreates it before include_bytes runs.
            output.push_str(&format!("const _: &[u8] = include_bytes!({path:?});"));
        }
    }
    let mut roots = Vec::new();
    let mut arms = String::new();
    for entry in catalog.entries() {
        let name = entry.root.to_string();
        roots.push(name.clone());
        let declaration = &entry.declarations[0];
        if !entry.declarations.iter().all(|other| {
            other.initializer == declaration.initializer
                && (other.initializer.is_none() || other.module == declaration.module)
        }) {
            return Err(eyre!(
                "Runnable root {name} has ambiguous initializers; use one initializer per root"
            ));
        }
        let site = Site {
            file: declaration.file.clone(),
            line: declaration.line,
            column: declaration.column,
        };
        let function = initializer_name(&site);
        arms.push_str(&format!(
            "{name:?} => {}::{function}(ctx),",
            declaration.module
        ));
    }
    let engine = &request.engine;
    let designtime = crate::designtime_enabled();
    let mut generated = pax_codegen::generate(&manifest, designtime, !designtime);
    let mut hasher = Sha256::new();
    hasher.update(pax_manifest::binary::to_vec(&manifest)?);
    hasher.update([u8::from(designtime)]);
    hasher.update(serde_json::to_vec(&(
        request.snapshot.clone(),
        request.dependencies.clone(),
    ))?);
    for path in session.source_paths() {
        hasher.update(std::fs::read(&path)?);
        generated.push_str(&format!("const _: &[u8] = include_bytes!({path:?});"));
    }
    let id = format!("{:x}", hasher.finalize());
    generated.push_str(&format!(r#"
        fn __pax_select_entry(entry: &str) -> Result<&'static str, String> {{
            const ROOTS: &[&str] = &{roots:?};
            if entry.is_empty() && ROOTS.len() == 1 {{ return Ok(ROOTS[0]); }}
            ROOTS.iter().copied().find(|root| *root == entry)
                .ok_or_else(|| format!("Select a declared runnable root: {{:?}}", ROOTS))
        }}
        fn __pax_initialize(entry: &str, ctx: &{engine}::api::InitContext) -> {engine}::api::AppResult<{engine}::api::pax_value::PaxAny> {{
            match entry {{ {arms} _ => Err("unknown runnable root".into()) }}
        }}
        pub(crate) fn init_manifest_for(entry: &str) -> pax_manifest::PaxManifest {{
            let mut manifest = init_manifest();
            let root = manifest.components.keys().find(|id|id.to_string() == entry).unwrap().clone();
            manifest.main_component_type_id = root.clone();
            for component in manifest.components.values_mut() {{ component.is_main_component = component.type_id == root; }}
            let wrapper = pax_manifest::TypeId::build_singleton("ROOT_COMPONENT", Some("RootComponent"));
            let mut template = pax_manifest::ComponentTemplate::new(wrapper.clone(), None);
            let mut node = pax_manifest::TemplateNodeDefinition::default(); node.type_id = root; template.add(node);
            manifest.components.get_mut(&wrapper).unwrap().template = Some(template);
            manifest
        }}
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        #[no_mangle]
        pub unsafe extern "C" fn pax_cartridge_program_id(buffer: *mut u8, capacity: usize) -> usize {{
            let bytes = {id:?}.as_bytes();
            if !buffer.is_null() && capacity >= bytes.len() {{ std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()); }}
            bytes.len()
        }}
    "#));
    let build_config = crate::TemplateBuildConfig {
        web: request.snapshot.facts["target_arch = \"wasm32\""],
        macos: request.snapshot.facts["target_os = \"macos\""],
        ios: request.snapshot.facts["target_os = \"ios\""],
        ipados: std::env::var("PAX_BUILD_TARGET").as_deref() == Ok("ipados"),
        designtime,
    };
    let package = metadata
        .packages
        .iter()
        .find(|p| p.manifest_path.as_std_path() == project.join("Cargo.toml"))
        .unwrap();
    let library = package
        .targets
        .iter()
        .find(|t| t.src_path.as_std_path() == request.snapshot.entry)
        .unwrap();
    let adapter = crate::TemplateArgsCartridge {
        catalog: true,
        web_entrypoint: pax_codegen::project_layout::web_entrypoint(
            &package.id.repr,
            &library.name,
        ),
        root_path: format!("{engine}::api::pax_value::PaxAny"),
        initializer: "|ctx| __pax_initialize(entry, ctx)".into(),
        cartridge_snippet: generated,
        engine_import_path: engine.clone(),
        build_config,
    }
    .render_once()?;
    let recipe = serde_json::to_string(&serde_json::json!({
        "library": library.name, "programId": id, "assets":manifest.assets_dirs,
        "title": package.metadata.get("pax").and_then(|p|p.get("title")).and_then(|s|s.as_str()).unwrap_or(&package.name),
        "roots": roots,
    }))?;
    let helper = helper_name(&request.snapshot.entry, "program");
    let standard_library = if engine.replace(' ', "") == "pax_kit::pax_engine" {
        "use pax_kit::pax_std;"
    } else {
        ""
    };
    let program = format!(
        r#"
        #[allow(unused_imports)] mod __pax_cartridge {{
            use super::*;
            use {engine} as pax_engine;
            use {engine}::*;
            use {engine}::api::*;
            {standard_library}
            {adapter}
        }}
        pub use __pax_cartridge::*;
        #[doc(hidden)] #[macro_export]
        macro_rules! {helper} {{ ($($callback:ident)::+, $options:expr) => {{ $($callback)::+!(({recipe:?}, $options)) }}; }}
    "#
    );
    session.programs.insert(key, program.clone());
    output.push_str(&program);
    Ok(output)
}

fn manifest_bytes(manifest: &impl Serialize) -> Result<Vec<u8>> {
    // Cargo tracks this sidecar through include_bytes!. Hash-map iteration
    // order must not rewrite it and invalidate an otherwise unchanged build.
    let mut value = serde_json::to_value(manifest)?;
    value.sort_all_objects();
    Ok(serde_json::to_vec(&value)?)
}

#[cfg(test)]
mod manifest_output_tests {
    use super::*;

    #[test]
    fn sidecar_order_is_stable_without_changing_array_order_or_values() {
        #[derive(Serialize)]
        struct Forward {
            a: u8,
            z: u8,
        }
        #[derive(Serialize)]
        struct Reverse {
            z: u8,
            a: u8,
        }
        #[derive(Serialize)]
        struct Nested<T> {
            items: Vec<T>,
        }

        let forward = Nested {
            items: vec![Forward { a: 1, z: 2 }, Forward { a: 3, z: 4 }],
        };
        let reverse = Nested {
            items: vec![Reverse { z: 2, a: 1 }, Reverse { z: 4, a: 3 }],
        };
        assert_ne!(
            serde_json::to_vec(&forward).unwrap(),
            serde_json::to_vec(&reverse).unwrap()
        );
        let bytes = manifest_bytes(&forward).unwrap();
        assert_eq!(bytes, manifest_bytes(&reverse).unwrap());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            serde_json::to_value(&forward).unwrap()
        );
        let changed = Nested {
            items: vec![Forward { a: 3, z: 4 }, Forward { a: 1, z: 2 }],
        };
        assert_ne!(bytes, manifest_bytes(&changed).unwrap());
    }
}

fn initializer_name(site: &Site) -> String {
    let digest = Sha256::digest(serde_json::to_vec(site).unwrap());
    format!("__pax_initialize_{digest:x}")
}

impl Session {
    fn source_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<_> = self
            .discoveries
            .values()
            .flat_map(|s| s.keys().cloned())
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }
}

fn runnable_declaration(item: &syn::Item) -> Result<pax_manifest::runnable::RunnableDeclaration> {
    let tokens = match item {
        syn::Item::Macro(item) => item.mac.tokens.clone(),
        syn::Item::Struct(item) => legacy_tokens(&item.ident, &item.attrs)?,
        syn::Item::Enum(item) => legacy_tokens(&item.ident, &item.attrs)?,
        _ => return Err(eyre!("Unsupported runnable declaration")),
    };
    Ok(syn::parse2(tokens)?)
}

fn normalize_legacy(items: &mut Vec<syn::Item>) -> Result<()> {
    let mut declarations = Vec::new();
    for item in items.iter_mut() {
        let declaration = match item {
            syn::Item::Struct(item) => legacy_tokens(&item.ident, &item.attrs)?,
            syn::Item::Enum(item) => legacy_tokens(&item.ident, &item.attrs)?,
            _ => proc_macro2::TokenStream::new(),
        };
        let ident = match item {
            syn::Item::Struct(item) if item.attrs.iter().any(|a| a.path.is_ident("main")) => {
                item.attrs.retain(|a| !a.path.is_ident("main"));
                Some(item.ident.clone())
            }
            syn::Item::Enum(item) if item.attrs.iter().any(|a| a.path.is_ident("main")) => {
                item.attrs.retain(|a| !a.path.is_ident("main"));
                Some(item.ident.clone())
            }
            syn::Item::Mod(item) => {
                if let Some((_, items)) = &mut item.content {
                    normalize_legacy(items)?;
                }
                None
            }
            _ => None,
        };
        if let Some(ident) = ident {
            declarations.push(syn::parse2(
                quote::quote_spanned!(ident.span()=> pax_kit::runnable! { #declaration }),
            )?);
        }
    }
    items.extend(declarations);
    Ok(())
}

fn request_dependency(
    request: &Request,
    metadata: &cargo_metadata::Metadata,
    error: color_eyre::Report,
) -> Result<String> {
    let project = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let callback = format!("{}::pax_macro::__pax_resume", request.engine);
    if let Some(NeedsSource(file)) = error.downcast_ref::<NeedsSource>() {
        if request
            .dependencies
            .iter()
            .any(|dependency| &dependency.entry == file)
        {
            return Err(eyre!("Configuration helper returned {} again without supplying the requested source graph", file.display()));
        }
        let package = metadata
            .packages
            .iter()
            .find(|p| p.targets.iter().any(|t| t.src_path.as_std_path() == file))
            .ok_or_else(|| eyre!("No package for {}", file.display()))?;
        let root = metadata
            .packages
            .iter()
            .find(|p| p.manifest_path.as_std_path() == project.join("Cargo.toml"))
            .unwrap();
        let node = metadata
            .resolve
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == root.id)
            .unwrap();
        let name = if let Some(dependency) = node.deps.iter().find(|d| d.pkg == package.id) {
            dependency.name.clone()
        } else if package.name == "pax-std" {
            let kit = node
                .deps
                .iter()
                .find(|d| {
                    metadata
                        .packages
                        .iter()
                        .any(|p| p.id == d.pkg && p.name == "pax-kit")
                })
                .ok_or_else(|| eyre!("Cannot reach pax-std configuration from this cartridge"))?;
            format!("{}::pax_std", kit.name)
        } else {
            return Err(eyre!(
                "Configuration provider for {} must be reachable through a direct dependency",
                package.name
            ));
        };
        let json = serde_json::to_string(&request)?;
        let helper = helper_name(file, "source_config");
        return Ok(format!(
            "::{name}::{helper}!({callback}, __pax_anchor, {json:?});"
        ));
    }
    Err(error)
}

fn legacy_tokens(
    ident: &syn::Ident,
    attributes: &[syn::Attribute],
) -> Result<proc_macro2::TokenStream> {
    let initializer = attributes
        .iter()
        .find(|a| a.path.is_ident("init"))
        .map(|a| a.parse_args::<syn::Path>())
        .transpose()?;
    Ok(if let Some(initializer) = initializer {
        quote::quote_spanned!(ident.span()=> root: #ident, init: #initializer)
    } else {
        quote::quote_spanned!(ident.span()=> root: #ident)
    })
}

// Dependency macro_rules helpers must forward the requesting crate's syntax
// context. A fresh call_site after a helper expansion belongs to the helper's
// crate and cannot resolve the caller's other dependency aliases.
fn anchor_tokens(
    tokens: proc_macro::TokenStream,
    anchor: proc_macro::Span,
) -> proc_macro::TokenStream {
    tokens
        .into_iter()
        .map(|mut token| {
            if let proc_macro::TokenTree::Group(group) = token {
                let mut rebuilt = proc_macro::Group::new(
                    group.delimiter(),
                    anchor_tokens(group.stream(), anchor),
                );
                rebuilt.set_span(anchor);
                token = proc_macro::TokenTree::Group(rebuilt);
            }
            token.set_span(anchor);
            token
        })
        .collect()
}

pub(crate) fn helper_name(entry: &Path, kind: &str) -> String {
    let path = entry.canonicalize().unwrap_or_else(|_| entry.to_owned());
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    format!("__pax_{kind}_{digest:x}")
}
