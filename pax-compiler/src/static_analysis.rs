use cargo_metadata::{Metadata, MetadataCommand, Package};
use color_eyre::eyre::{eyre, Result};
use pax_lang::{parse_pax_str, Pair, Rule};
use pax_manifest::parsing::{
    assemble_component_definition, assemble_primitive_definition,
    assemble_struct_only_component_definition, ParsingContext,
};
use pax_manifest::{
    PaxManifest, PropertyDefinition, PropertyDefinitionFlags, TypeDefinition, TypeId,
};
use proc_macro2::TokenTree;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use syn::{
    Attribute, Field, Fields, GenericArgument, Item, ItemEnum, ItemMod, ItemStruct, Lit, Meta,
    NestedMeta, PathArguments, Type, UseTree,
};

const DEFAULT_ENGINE_IMPORT_PATH: &str = "pax_kit::pax_engine";

pub fn build_manifest(project_path: &Path) -> Result<PaxManifest> {
    let project_manifest_path = canonical_manifest_path(project_path)?;
    let project_dir = project_manifest_path
        .parent()
        .ok_or_else(|| eyre!("Project manifest has no parent directory"))?
        .to_path_buf();

    let metadata = MetadataCommand::new()
        .current_dir(&project_dir)
        .exec()
        .map_err(|err| eyre!("Failed to read cargo metadata: {err}"))?;

    let root_package = find_root_package(&metadata, &project_manifest_path)?;
    let registry = build_registry(&metadata, root_package)?;
    let main_item = registry.root_main_component()?;

    let mut ctx = ParsingContext::default();
    ctx.main_component_type_id = main_item.type_id();

    build_item_recursive(&mut ctx, &registry, &main_item.import_path)?;

    let assets_dir = project_dir.join("assets");
    if let Ok(canonical_assets_dir) = assets_dir.canonicalize() {
        if let Some(path_str) = canonical_assets_dir.to_str() {
            ctx.assets_dirs.push(path_str.to_string());
        }
    }

    Ok(PaxManifest {
        components: ctx.component_definitions,
        main_component_type_id: ctx.main_component_type_id,
        type_table: ctx.type_table,
        assets_dirs: ctx.assets_dirs,
        engine_import_path: main_item.engine_import_path.clone(),
    })
}

fn build_item_recursive(
    ctx: &mut ParsingContext,
    registry: &StaticRegistry,
    import_path: &str,
) -> Result<()> {
    let item = registry
        .items_by_import_path
        .get(import_path)
        .ok_or_else(|| eyre!("Static analysis could not find `{import_path}`"))?;
    let self_type_id = item.type_id();

    if ctx.visited_type_ids.contains(&self_type_id) {
        return Ok(());
    }
    ctx.visited_type_ids.insert(self_type_id.clone());

    let property_definitions = match &item.data {
        DataSummary::Struct(fields) => {
            let mut property_definitions = vec![];
            for field in fields {
                let type_id = ensure_syn_type(ctx, registry, &field.ty, &item.scope)?;
                let mut flags = PropertyDefinitionFlags::default();
                flags.is_property_wrapped = field.is_property_wrapped;
                property_definitions.push(PropertyDefinition {
                    name: field.name.clone(),
                    flags,
                    type_id,
                });
            }
            property_definitions
        }
        DataSummary::Enum(variants) => {
            for variant in variants {
                for field_ty in &variant.field_types {
                    let _ = ensure_syn_type(ctx, registry, field_ty, &item.scope)?;
                }
            }
            vec![]
        }
    };

    ctx.type_table.insert(
        self_type_id.clone(),
        TypeDefinition {
            type_id: self_type_id.clone(),
            inner_iterable_type_id: None,
            property_definitions: property_definitions.clone(),
        },
    );

    let component_definition = match &item.kind {
        PaxItemKind::FullComponent {
            raw_pax,
            is_main_component,
            associated_pax_file_path,
        } => {
            let mut template_dependencies =
                parse_pascal_identifiers_from_component_definition_string(raw_pax)
                    .map_err(|err| eyre!("Failed to statically parse template for `{}`: {err}", item.import_path))?;

            if *is_main_component {
                template_dependencies.push("BlankComponent".to_string());
            }

            let mut template_map = HashMap::new();
            for dependency_identifier in template_dependencies {
                let dependency_import_path =
                    resolve_identifier_to_import_path(&dependency_identifier, &item.scope, registry)?;
                build_item_recursive(ctx, registry, &dependency_import_path)?;
                let dependency_item = registry
                    .items_by_import_path
                    .get(&dependency_import_path)
                    .ok_or_else(|| eyre!("Resolved dependency `{dependency_import_path}` is not a Pax item"))?;
                template_map.insert(dependency_identifier, dependency_item.type_id());
            }

            let component_source_file_path = associated_pax_file_path
                .as_ref()
                .unwrap_or(&item.source_path)
                .to_string_lossy()
                .to_string();
            let owned_ctx = std::mem::take(ctx);
            let (new_ctx, component_definition) = assemble_component_definition(
                owned_ctx,
                raw_pax,
                *is_main_component,
                template_map,
                &item.module_path,
                self_type_id.clone(),
                &component_source_file_path,
            );
            *ctx = new_ctx;
            component_definition
        }
        PaxItemKind::Primitive {
            primitive_instance_import_path,
        } => assemble_primitive_definition(
            &item.module_path,
            primitive_instance_import_path.clone(),
            self_type_id.clone(),
        ),
        PaxItemKind::StructOnly => {
            let owned_ctx = std::mem::take(ctx);
            let (new_ctx, component_definition) =
                assemble_struct_only_component_definition(owned_ctx, &item.module_path, self_type_id.clone());
            *ctx = new_ctx;
            component_definition
        }
    };

    ctx.component_definitions
        .insert(self_type_id, component_definition);
    Ok(())
}

fn ensure_syn_type(
    ctx: &mut ParsingContext,
    registry: &StaticRegistry,
    ty: &Type,
    scope: &ScopeImports,
) -> Result<TypeId> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() => {
            let segments: Vec<String> = type_path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();

            let last_segment = type_path
                .path
                .segments
                .last()
                .ok_or_else(|| eyre!("Missing path segment in type"))?;
            let last_ident = last_segment.ident.to_string();

            if is_option_path(&segments) {
                let inner_ty = single_generic_type(last_segment)?;
                let inner_type_id = ensure_syn_type(ctx, registry, inner_ty, scope)?;
                let type_id = TypeId::build_option(&inner_type_id.to_string());
                ctx.type_table
                    .entry(type_id.clone())
                    .or_insert_with(|| TypeDefinition {
                        type_id: type_id.clone(),
                        inner_iterable_type_id: None,
                        property_definitions: vec![],
                    });
                return Ok(type_id);
            }

            if is_vec_path(&segments) {
                let inner_ty = single_generic_type(last_segment)?;
                let inner_type_id = ensure_syn_type(ctx, registry, inner_ty, scope)?;
                let type_id = TypeId::build_vector(&inner_type_id.to_string());
                ctx.type_table
                    .entry(type_id.clone())
                    .or_insert_with(|| TypeDefinition {
                        type_id: type_id.clone(),
                        inner_iterable_type_id: Some(inner_type_id),
                        property_definitions: vec![],
                    });
                return Ok(type_id);
            }

            if is_hash_map_path(&segments) {
                let (key_ty, value_ty) = pair_generic_types(last_segment)?;
                let key_type_id = ensure_syn_type(ctx, registry, key_ty, scope)?;
                let value_type_id = ensure_syn_type(ctx, registry, value_ty, scope)?;
                let type_id = TypeId::build_map(&key_type_id.to_string(), &value_type_id.to_string());
                ctx.type_table
                    .entry(type_id.clone())
                    .or_insert_with(|| TypeDefinition {
                        type_id: type_id.clone(),
                        inner_iterable_type_id: None,
                        property_definitions: vec![],
                    });
                return Ok(type_id);
            }

            if segments.len() == 1 {
                if let Some(primitive_type_id) = primitive_type_id_for_ident(&last_ident) {
                    ctx.type_table.entry(primitive_type_id.clone()).or_insert_with(|| {
                        TypeDefinition {
                            type_id: primitive_type_id.clone(),
                            inner_iterable_type_id: None,
                            property_definitions: vec![],
                        }
                    });
                    return Ok(primitive_type_id);
                }

                if let Some(import_path) = canonical_special_import_path_for_ident(&last_ident) {
                    return ensure_known_type_definition(ctx, import_path);
                }

                let import_path = resolve_identifier_to_import_path(&last_ident, scope, registry)?;
                return ensure_import_path_type(ctx, registry, &import_path);
            }

            let raw_path = segments.join("::");
            let canonical_path = canonicalize_path(&raw_path, scope)?;
            if let Some(import_path) = canonical_special_import_path_for_path(&canonical_path) {
                return ensure_known_type_definition(ctx, import_path);
            }

            if let Some(import_path) = canonical_special_import_path_for_ident(&last_ident) {
                if canonical_path.starts_with("pax_engine::api::")
                    || canonical_path.starts_with("pax_runtime::api::")
                    || canonical_path.starts_with("pax_runtime_api::")
                {
                    return ensure_known_type_definition(ctx, import_path);
                }
            }

            ensure_import_path_type(ctx, registry, &canonical_path)
        }
        Type::Tuple(tuple) if tuple.elems.is_empty() => {
            let type_id = TypeId::build_primitive("()");
            ctx.type_table.entry(type_id.clone()).or_insert_with(|| TypeDefinition {
                type_id: type_id.clone(),
                inner_iterable_type_id: None,
                property_definitions: vec![],
            });
            Ok(type_id)
        }
        other => Err(eyre!(
            "Unsupported static analysis type `{}` in module `{}`",
            type_to_string(other),
            scope.module_path
        )),
    }
}

fn ensure_import_path_type(
    ctx: &mut ParsingContext,
    registry: &StaticRegistry,
    import_path: &str,
) -> Result<TypeId> {
    if let Some(special_import_path) = canonical_special_import_path_for_path(import_path) {
        return ensure_known_type_definition(ctx, special_import_path);
    }

    if let Some(item) = registry.items_by_import_path.get(import_path) {
        build_item_recursive(ctx, registry, &item.import_path)?;
        return Ok(item.type_id());
    }

    Err(eyre!("Unresolved static type `{import_path}`"))
}

fn ensure_known_type_definition(ctx: &mut ParsingContext, import_path: &str) -> Result<TypeId> {
    let type_id = match import_path {
        "std::string::String" => TypeId::build_singleton(import_path, Some("String")),
        "pax_manifest::TypeId" => TypeId::build_singleton(import_path, Some("TypeId")),
        "pax_manifest::TemplateNodeId" => TypeId::build_singleton(import_path, Some("TemplateNodeId")),
        "pax_engine::api::Fill" => TypeId::build_singleton(import_path, Some("Fill")),
        "pax_engine::api::Stroke" => TypeId::build_singleton(import_path, Some("Stroke")),
        "pax_engine::api::Size" => TypeId::build_singleton(import_path, Some("Size")),
        "pax_engine::api::Color" => TypeId::build_singleton(import_path, Some("Color")),
        "pax_engine::api::PathElement" => TypeId::build_singleton(import_path, Some("PathElement")),
        "pax_engine::api::ColorChannel" => {
            TypeId::build_singleton(import_path, Some("ColorChannel"))
        }
        "pax_engine::api::Rotation" => TypeId::build_singleton(import_path, Some("Rotation")),
        "pax_engine::api::Numeric" => TypeId::build_singleton(import_path, Some("Numeric")),
        "pax_engine::api::Transform2D" => {
            TypeId::build_singleton(import_path, Some("Transform2D"))
        }
        "kurbo::Point" => TypeId::build_singleton(import_path, Some("Point")),
        other => return Err(eyre!("Unsupported canonical static type `{other}`")),
    };

    if ctx.type_table.contains_key(&type_id) {
        return Ok(type_id);
    }

    if import_path == "pax_engine::api::Stroke" {
        let color_type_id = ensure_known_type_definition(ctx, "pax_engine::api::Color")?;
        let size_type_id = ensure_known_type_definition(ctx, "pax_engine::api::Size")?;

        let mut flags = PropertyDefinitionFlags::default();
        flags.is_property_wrapped = true;

        ctx.type_table.insert(
            type_id.clone(),
            TypeDefinition {
                type_id: type_id.clone(),
                inner_iterable_type_id: None,
                property_definitions: vec![
                    PropertyDefinition {
                        name: "color".to_string(),
                        flags: flags.clone(),
                        type_id: color_type_id,
                    },
                    PropertyDefinition {
                        name: "width".to_string(),
                        flags,
                        type_id: size_type_id,
                    },
                ],
            },
        );
    } else {
        ctx.type_table.insert(
            type_id.clone(),
            TypeDefinition {
                type_id: type_id.clone(),
                inner_iterable_type_id: None,
                property_definitions: vec![],
            },
        );
    }

    Ok(type_id)
}

fn resolve_identifier_to_import_path(
    identifier: &str,
    scope: &ScopeImports,
    registry: &StaticRegistry,
) -> Result<String> {
    let same_module_candidate = format!("{}::{}", scope.module_path, identifier);
    if registry.items_by_import_path.contains_key(&same_module_candidate) {
        return Ok(same_module_candidate);
    }

    if let Some(import_path) = scope.explicit.get(identifier) {
        let canonical_import_path = canonicalize_path(import_path, scope)?;
        if let Some(special_import_path) = canonical_special_import_path_for_path(&canonical_import_path)
        {
            return Ok(special_import_path.to_string());
        }
        if let Some(special_import_path) = canonical_special_import_path_for_ident(identifier) {
            return Ok(special_import_path.to_string());
        }
        return Ok(canonical_import_path);
    }

    if let Some(special_import_path) = canonical_special_import_path_for_ident(identifier) {
        if scope
            .glob_roots
            .iter()
            .any(|root| root == "pax_kit" || root == "pax_engine" || root == "pax_engine::api")
        {
            return Ok(special_import_path.to_string());
        }
    }

    for glob_root in &scope.glob_roots {
        if glob_root == "pax_kit" {
            if let Some(import_path) = registry.unique_item_below_root("pax_std", identifier) {
                return Ok(import_path);
            }
            if let Some(special_import_path) = canonical_special_import_path_for_ident(identifier) {
                return Ok(special_import_path.to_string());
            }
            continue;
        }

        let canonical_root = canonicalize_path(glob_root, scope)?;
        let direct_candidate = format!("{}::{}", canonical_root, identifier);
        if registry.items_by_import_path.contains_key(&direct_candidate) {
            return Ok(direct_candidate);
        }

        if let Some(import_path) = registry.unique_item_below_root(&canonical_root, identifier) {
            return Ok(import_path);
        }

        if let Some(special_import_path) = canonical_special_import_path_for_ident(identifier) {
            if canonical_root == "pax_engine"
                || canonical_root == "pax_engine::api"
                || canonical_root == "pax_runtime::api"
                || canonical_root == "pax_runtime_api"
            {
                return Ok(special_import_path.to_string());
            }
        }
    }

    if let Some(import_paths) = registry.items_by_identifier.get(identifier) {
        if import_paths.len() == 1 {
            return Ok(import_paths[0].clone());
        }
    }

    Err(eyre!(
        "Static analysis could not resolve `{identifier}` in module `{}`",
        scope.module_path
    ))
}

fn build_registry(metadata: &Metadata, root_package: &Package) -> Result<StaticRegistry> {
    let mut registry = StaticRegistry::default();

    for package in metadata.packages.iter().filter(|package| package.source.is_none()) {
        let manifest_path = package.manifest_path.as_std_path();
        let manifest_dir = manifest_path
            .parent()
            .ok_or_else(|| eyre!("Package manifest has no parent directory"))?
            .to_path_buf();
        let src_dir = manifest_dir.join("src");
        if !src_dir.exists() {
            continue;
        }

        let entry_file = if src_dir.join("lib.rs").exists() {
            src_dir.join("lib.rs")
        } else if src_dir.join("main.rs").exists() {
            src_dir.join("main.rs")
        } else {
            continue;
        };

        let import_root = if package.name == root_package.name {
            "crate".to_string()
        } else {
            package.name.replace('-', "_")
        };

        let package_context = PackageContext {
            manifest_dir,
            entry_file,
            package_name: package.name.clone(),
            import_root,
        };

        let mut seen_files = HashSet::new();
        scan_module_file(&package_context, &package_context.entry_file, &package_context.import_root, &mut seen_files, &mut registry)?;
    }

    registry.root_package_name = root_package.name.clone();
    Ok(registry)
}

fn scan_module_file(
    package_context: &PackageContext,
    file_path: &Path,
    module_path: &str,
    seen_files: &mut HashSet<PathBuf>,
    registry: &mut StaticRegistry,
) -> Result<()> {
    let canonical_file_path = file_path
        .canonicalize()
        .map_err(|err| eyre!("Failed to canonicalize `{}`: {err}", file_path.display()))?;
    if !seen_files.insert(canonical_file_path.clone()) {
        return Ok(());
    }

    let file_contents = fs::read_to_string(&canonical_file_path).map_err(|err| {
        eyre!(
            "Failed to read source file `{}` for static analysis: {err}",
            canonical_file_path.display()
        )
    })?;
    let parsed_file = syn::parse_file(&file_contents).map_err(|err| {
        eyre!(
            "Failed to parse Rust source `{}` for static analysis: {err}",
            canonical_file_path.display()
        )
    })?;

    let scope = collect_scope(
        &parsed_file.items,
        module_path.to_string(),
        package_context.import_root.clone(),
    )?;
    let child_module_dir = child_module_dir_for_file(&canonical_file_path)?;

    for item in parsed_file.items {
        match item {
            Item::Struct(item_struct) if has_pax_attr(&item_struct.attrs) => {
                register_struct_item(
                    package_context,
                    &scope,
                    &canonical_file_path,
                    item_struct,
                    registry,
                )?;
            }
            Item::Enum(item_enum) if has_pax_attr(&item_enum.attrs) => {
                register_enum_item(
                    package_context,
                    &scope,
                    &canonical_file_path,
                    item_enum,
                    registry,
                )?;
            }
            Item::Mod(item_mod) => {
                scan_child_module(
                    package_context,
                    &canonical_file_path,
                    &child_module_dir,
                    &scope,
                    item_mod,
                    seen_files,
                    registry,
                )?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn scan_child_module(
    package_context: &PackageContext,
    source_file_path: &Path,
    child_module_dir: &Path,
    parent_scope: &ScopeImports,
    item_mod: ItemMod,
    seen_files: &mut HashSet<PathBuf>,
    registry: &mut StaticRegistry,
) -> Result<()> {
    let child_module_path = format!("{}::{}", parent_scope.module_path, item_mod.ident);
    if let Some((_, items)) = item_mod.content {
        let child_scope = collect_scope(&items, child_module_path, parent_scope.import_root.clone())?;
        for item in items {
            match item {
                Item::Struct(item_struct) if has_pax_attr(&item_struct.attrs) => {
                    register_struct_item(
                        package_context,
                        &child_scope,
                        source_file_path,
                        item_struct,
                        registry,
                    )?;
                }
                Item::Enum(item_enum) if has_pax_attr(&item_enum.attrs) => {
                    register_enum_item(
                        package_context,
                        &child_scope,
                        source_file_path,
                        item_enum,
                        registry,
                    )?;
                }
                Item::Mod(child_mod) => {
                    scan_child_module(
                        package_context,
                        source_file_path,
                        child_module_dir,
                        &child_scope,
                        child_mod,
                        seen_files,
                        registry,
                    )?;
                }
                _ => {}
            }
        }
        return Ok(());
    }

    let child_file_path = resolve_child_module_file(child_module_dir, &item_mod.ident.to_string())?;
    scan_module_file(
        package_context,
        &child_file_path,
        &child_module_path,
        seen_files,
        registry,
    )
}

fn register_struct_item(
    package_context: &PackageContext,
    scope: &ScopeImports,
    source_file_path: &Path,
    item_struct: ItemStruct,
    registry: &mut StaticRegistry,
) -> Result<()> {
    let config = parse_pax_config(&item_struct.attrs)?;
    let import_path = format!("{}::{}", scope.module_path, item_struct.ident);
    let kind = build_item_kind(package_context, source_file_path, &config)?;
    let data = build_struct_data_summary(&item_struct)?;
    registry.insert(ScannedPaxItem {
        ident: item_struct.ident.to_string(),
        import_path,
        module_path: scope.module_path.clone(),
        source_path: source_file_path.to_path_buf(),
        scope: scope.clone(),
        kind,
        data,
        engine_import_path: config
            .engine_import_path
            .unwrap_or_else(|| DEFAULT_ENGINE_IMPORT_PATH.to_string()),
        package_name: package_context.package_name.clone(),
    })
}

fn register_enum_item(
    package_context: &PackageContext,
    scope: &ScopeImports,
    source_file_path: &Path,
    item_enum: ItemEnum,
    registry: &mut StaticRegistry,
) -> Result<()> {
    let config = parse_pax_config(&item_enum.attrs)?;
    let import_path = format!("{}::{}", scope.module_path, item_enum.ident);
    let kind = build_item_kind(package_context, source_file_path, &config)?;
    let data = build_enum_data_summary(&item_enum);
    registry.insert(ScannedPaxItem {
        ident: item_enum.ident.to_string(),
        import_path,
        module_path: scope.module_path.clone(),
        source_path: source_file_path.to_path_buf(),
        scope: scope.clone(),
        kind,
        data,
        engine_import_path: config
            .engine_import_path
            .unwrap_or_else(|| DEFAULT_ENGINE_IMPORT_PATH.to_string()),
        package_name: package_context.package_name.clone(),
    })
}

fn build_item_kind(
    package_context: &PackageContext,
    source_file_path: &Path,
    config: &PaxConfig,
) -> Result<PaxItemKind> {
    if config.file_path.is_some() && config.inlined_contents.is_some() {
        return Err(eyre!(
            "`#[file(...)]` and `#[inlined(...)]` cannot be used together"
        ));
    }

    if let Some(inlined_contents) = &config.inlined_contents {
        return Ok(PaxItemKind::FullComponent {
            raw_pax: inlined_contents.clone(),
            is_main_component: config.is_main_component,
            associated_pax_file_path: None,
        });
    }

    if let Some(file_path) = &config.file_path {
        let resolved_file_path =
            resolve_template_file_path(&package_context.manifest_dir, file_path)?;
        let raw_pax = fs::read_to_string(&resolved_file_path).map_err(|err| {
            eyre!(
                "Failed to read Pax template `{}`: {err}",
                resolved_file_path.display()
            )
        })?;
        return Ok(PaxItemKind::FullComponent {
            raw_pax,
            is_main_component: config.is_main_component,
            associated_pax_file_path: Some(resolved_file_path),
        });
    }

    if config.is_main_component {
        return Err(eyre!(
            "Main component `{}` must have either `#[file(...)]` or `#[inlined(...)]`",
            source_file_path.display()
        ));
    }

    if config.is_primitive {
        let primitive_instance_import_path = config
            .primitive_instance_import_path
            .clone()
            .ok_or_else(|| eyre!("Primitive component is missing its instance import path"))?;
        return Ok(PaxItemKind::Primitive {
            primitive_instance_import_path,
        });
    }

    Ok(PaxItemKind::StructOnly)
}

fn build_struct_data_summary(item_struct: &ItemStruct) -> Result<DataSummary> {
    let named_fields = match &item_struct.fields {
        Fields::Named(named_fields) => &named_fields.named,
        _ => {
            return Err(eyre!(
                "Pax static analysis only supports structs with named fields for `{}`",
                item_struct.ident
            ))
        }
    };

    let mut fields = vec![];
    for field in named_fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| eyre!("Encountered unnamed field in `{}`", item_struct.ident))?
            .to_string();
        let (field_ty, is_property_wrapped) = get_field_type(field)?;
        fields.push(FieldSummary {
            name: field_name,
            ty: field_ty,
            is_property_wrapped,
        });
    }

    Ok(DataSummary::Struct(fields))
}

fn build_enum_data_summary(item_enum: &ItemEnum) -> DataSummary {
    let mut variants = vec![];
    for variant in &item_enum.variants {
        let field_types = variant
            .fields
            .iter()
            .map(|field| field.ty.clone())
            .collect::<Vec<_>>();
        variants.push(EnumVariantSummary { field_types });
    }
    DataSummary::Enum(variants)
}

fn find_root_package<'a>(metadata: &'a Metadata, project_manifest_path: &Path) -> Result<&'a Package> {
    metadata
        .packages
        .iter()
        .find(|package| {
            package
                .manifest_path
                .as_std_path()
                .canonicalize()
                .map(|path| path == project_manifest_path)
                .unwrap_or(false)
        })
        .ok_or_else(|| eyre!("Failed to resolve the package for `{}`", project_manifest_path.display()))
}

fn canonical_manifest_path(project_path: &Path) -> Result<PathBuf> {
    let manifest_path = if project_path.is_dir() {
        project_path.join("Cargo.toml")
    } else {
        project_path.to_path_buf()
    };
    manifest_path
        .canonicalize()
        .map_err(|err| eyre!("Failed to canonicalize `{}`: {err}", manifest_path.display()))
}

fn parse_pax_config(attrs: &[Attribute]) -> Result<PaxConfig> {
    let mut config = PaxConfig::default();

    for attr in attrs {
        match attr.path.get_ident().map(|ident| ident.to_string()) {
            Some(ref ident) if ident == "file" => {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    if let Some(NestedMeta::Lit(Lit::Str(file_str))) = meta_list.nested.first() {
                        config.file_path = Some(file_str.value());
                    }
                }
            }
            Some(ref ident) if ident == "engine_import_path" => {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    if let Some(NestedMeta::Lit(Lit::Str(path_str))) = meta_list.nested.first() {
                        config.engine_import_path = Some(path_str.value());
                    }
                }
            }
            Some(ref ident) if ident == "primitive" => {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    if let Some(NestedMeta::Lit(Lit::Str(path_str))) = meta_list.nested.first() {
                        config.is_primitive = true;
                        config.primitive_instance_import_path = Some(path_str.value());
                    }
                }
            }
            Some(ref ident) if ident == "inlined" => {
                let mut content = proc_macro2::TokenStream::new();
                for token in attr.tokens.clone() {
                    if let TokenTree::Group(group) = token {
                        if group.delimiter() == proc_macro2::Delimiter::Parenthesis {
                            content.extend(group.stream());
                        }
                    }
                }
                if !content.is_empty() {
                    config.inlined_contents = Some(content.to_string());
                }
            }
            Some(ref ident) if ident == "pax" => {}
            _ => {
                if let Ok(Meta::Path(path)) = attr.parse_meta() {
                    if path.is_ident("main") {
                        config.is_main_component = true;
                    }
                }
            }
        }
    }

    Ok(config)
}

fn get_field_type(field: &Field) -> Result<(Type, bool)> {
    let Type::Path(type_path) = &field.ty else {
        return Err(eyre!(
            "Unsupported field type `{}` in static analysis",
            type_to_string(&field.ty)
        ));
    };

    if type_path.qself.is_none() {
        for segment in &type_path.path.segments {
            if segment.ident.to_string().ends_with("Property") {
                if let PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    for argument in &arguments.args {
                        if let GenericArgument::Type(generic_ty) = argument {
                            return Ok((generic_ty.clone(), true));
                        }
                    }
                }
            }
        }
        return Ok((field.ty.clone(), false));
    }

    Err(eyre!(
        "Unsupported qself field type `{}` in static analysis",
        type_to_string(&field.ty)
    ))
}

fn single_generic_type(segment: &syn::PathSegment) -> Result<&Type> {
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(eyre!("Expected angle-bracketed generic arguments"));
    };
    arguments
        .args
        .iter()
        .find_map(|argument| match argument {
            GenericArgument::Type(generic_ty) => Some(generic_ty),
            _ => None,
        })
        .ok_or_else(|| eyre!("Missing generic type argument"))
}

fn pair_generic_types(segment: &syn::PathSegment) -> Result<(&Type, &Type)> {
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(eyre!("Expected angle-bracketed generic arguments"));
    };
    let mut generic_types = arguments.args.iter().filter_map(|argument| match argument {
        GenericArgument::Type(generic_ty) => Some(generic_ty),
        _ => None,
    });
    let key_ty = generic_types
        .next()
        .ok_or_else(|| eyre!("Missing first generic type argument"))?;
    let value_ty = generic_types
        .next()
        .ok_or_else(|| eyre!("Missing second generic type argument"))?;
    Ok((key_ty, value_ty))
}

fn primitive_type_id_for_ident(ident: &str) -> Option<TypeId> {
    match ident {
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64"
        | "i128" | "isize" | "f64" | "f32" | "bool" | "char" | "()" => {
            Some(TypeId::build_primitive(ident))
        }
        _ => None,
    }
}

fn canonical_special_import_path_for_ident(ident: &str) -> Option<&'static str> {
    match ident {
        "String" => Some("std::string::String"),
        "TypeId" => Some("pax_manifest::TypeId"),
        "TemplateNodeId" => Some("pax_manifest::TemplateNodeId"),
        "Fill" => Some("pax_engine::api::Fill"),
        "Stroke" => Some("pax_engine::api::Stroke"),
        "Size" => Some("pax_engine::api::Size"),
        "Color" => Some("pax_engine::api::Color"),
        "PathElement" => Some("pax_engine::api::PathElement"),
        "ColorChannel" => Some("pax_engine::api::ColorChannel"),
        "Rotation" => Some("pax_engine::api::Rotation"),
        "Numeric" => Some("pax_engine::api::Numeric"),
        "Transform2D" => Some("pax_engine::api::Transform2D"),
        "Point" => Some("kurbo::Point"),
        _ => None,
    }
}

fn canonical_special_import_path_for_path(path: &str) -> Option<&'static str> {
    match path {
        "std::string::String" => Some("std::string::String"),
        "pax_manifest::TypeId" => Some("pax_manifest::TypeId"),
        "pax_manifest::TemplateNodeId" => Some("pax_manifest::TemplateNodeId"),
        "pax_engine::api::Fill" | "pax_runtime::api::Fill" | "pax_runtime_api::Fill" => {
            Some("pax_engine::api::Fill")
        }
        "pax_engine::api::Stroke" | "pax_runtime::api::Stroke" | "pax_runtime_api::Stroke" => {
            Some("pax_engine::api::Stroke")
        }
        "pax_engine::api::Size" | "pax_runtime::api::Size" | "pax_runtime_api::Size" => {
            Some("pax_engine::api::Size")
        }
        "pax_engine::api::Color" | "pax_runtime::api::Color" | "pax_runtime_api::Color" => {
            Some("pax_engine::api::Color")
        }
        "pax_engine::api::PathElement"
        | "pax_runtime::api::PathElement"
        | "pax_runtime_api::PathElement" => Some("pax_engine::api::PathElement"),
        "pax_engine::api::ColorChannel"
        | "pax_runtime::api::ColorChannel"
        | "pax_runtime_api::ColorChannel" => Some("pax_engine::api::ColorChannel"),
        "pax_engine::api::Rotation"
        | "pax_runtime::api::Rotation"
        | "pax_runtime_api::Rotation" => Some("pax_engine::api::Rotation"),
        "pax_engine::api::Numeric"
        | "pax_runtime::api::Numeric"
        | "pax_runtime_api::Numeric" => Some("pax_engine::api::Numeric"),
        "pax_engine::api::Transform2D"
        | "pax_runtime::api::Transform2D"
        | "pax_runtime_api::Transform2D" => Some("pax_engine::api::Transform2D"),
        "kurbo::Point" => Some("kurbo::Point"),
        _ => None,
    }
}

fn is_option_path(segments: &[String]) -> bool {
    match segments {
        [ident] => ident == "Option",
        [std, option, ident] => std == "std" && option == "option" && ident == "Option",
        _ => false,
    }
}

fn is_vec_path(segments: &[String]) -> bool {
    match segments {
        [ident] => ident == "Vec",
        [std, vec, ident] => std == "std" && vec == "vec" && ident == "Vec",
        _ => false,
    }
}

fn is_hash_map_path(segments: &[String]) -> bool {
    match segments {
        [ident] => ident == "HashMap",
        [std, collections, ident] => {
            std == "std" && collections == "collections" && ident == "HashMap"
        }
        _ => false,
    }
}

fn canonicalize_path(raw_path: &str, scope: &ScopeImports) -> Result<String> {
    let parts = raw_path.split("::").collect::<Vec<_>>();
    if parts.is_empty() {
        return Err(eyre!("Cannot canonicalize empty path"));
    }

    let mut remaining_index = 0;
    let mut resolved_segments = match parts[0] {
        "crate" => {
            remaining_index = 1;
            scope.import_root.split("::").map(str::to_string).collect()
        }
        "self" => {
            remaining_index = 1;
            scope.module_path.split("::").map(str::to_string).collect()
        }
        "super" => {
            let mut segments = scope
                .module_path
                .split("::")
                .map(str::to_string)
                .collect::<Vec<_>>();
            while remaining_index < parts.len() && parts[remaining_index] == "super" {
                if segments.len() <= 1 {
                    return Err(eyre!("Cannot resolve path `{raw_path}` above the crate root"));
                }
                segments.pop();
                remaining_index += 1;
            }
            segments
        }
        _ => return Ok(raw_path.to_string()),
    };

    while remaining_index < parts.len() {
        resolved_segments.push(parts[remaining_index].to_string());
        remaining_index += 1;
    }

    Ok(resolved_segments.join("::"))
}

fn collect_scope(
    items: &[Item],
    module_path: String,
    import_root: String,
) -> Result<ScopeImports> {
    let mut scope = ScopeImports {
        module_path,
        import_root,
        explicit: HashMap::new(),
        glob_roots: vec![],
    };

    for item in items {
        if let Item::Use(item_use) = item {
            flatten_use_tree(&item_use.tree, "", &mut scope)?;
        }
    }

    Ok(scope)
}

fn flatten_use_tree(tree: &UseTree, prefix: &str, scope: &mut ScopeImports) -> Result<()> {
    match tree {
        UseTree::Path(path) => {
            let next_prefix = join_use_prefix(prefix, &path.ident.to_string());
            flatten_use_tree(&path.tree, &next_prefix, scope)
        }
        UseTree::Name(name) => {
            let ident = name.ident.to_string();
            if ident == "self" {
                scope.explicit.insert(scope_last_segment(prefix), prefix.to_string());
            } else {
                scope
                    .explicit
                    .insert(ident.clone(), join_use_prefix(prefix, &ident));
            }
            Ok(())
        }
        UseTree::Rename(rename) => {
            let import_path = join_use_prefix(prefix, &rename.ident.to_string());
            scope
                .explicit
                .insert(rename.rename.to_string(), import_path);
            Ok(())
        }
        UseTree::Glob(_) => {
            scope.glob_roots.push(prefix.to_string());
            Ok(())
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(item, prefix, scope)?;
            }
            Ok(())
        }
    }
}

fn join_use_prefix(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}::{segment}")
    }
}

fn scope_last_segment(prefix: &str) -> String {
    prefix
        .split("::")
        .last()
        .unwrap_or(prefix)
        .to_string()
}

fn child_module_dir_for_file(file_path: &Path) -> Result<PathBuf> {
    let parent_dir = file_path
        .parent()
        .ok_or_else(|| eyre!("Source file `{}` has no parent directory", file_path.display()))?;
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre!("Source file `{}` has no valid file name", file_path.display()))?;

    let child_dir = match file_name {
        "lib.rs" | "main.rs" | "mod.rs" => parent_dir.to_path_buf(),
        _ => parent_dir.join(
            file_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| eyre!("Source file `{}` has no valid stem", file_path.display()))?,
        ),
    };

    Ok(child_dir)
}

fn resolve_child_module_file(child_module_dir: &Path, module_name: &str) -> Result<PathBuf> {
    let flat_file = child_module_dir.join(format!("{module_name}.rs"));
    if flat_file.exists() {
        return Ok(flat_file);
    }

    let module_dir_file = child_module_dir.join(module_name).join("mod.rs");
    if module_dir_file.exists() {
        return Ok(module_dir_file);
    }

    Err(eyre!(
        "Failed to resolve module file for `{module_name}` under `{}`",
        child_module_dir.display()
    ))
}

fn resolve_template_file_path(manifest_dir: &Path, file_path: &str) -> Result<PathBuf> {
    let direct_path = manifest_dir.join(file_path);
    if direct_path.exists() {
        return direct_path
            .canonicalize()
            .map_err(|err| eyre!("Failed to canonicalize `{}`: {err}", direct_path.display()));
    }

    let src_path = manifest_dir.join("src").join(file_path);
    if src_path.exists() {
        return src_path
            .canonicalize()
            .map_err(|err| eyre!("Failed to canonicalize `{}`: {err}", src_path.display()));
    }

    Err(eyre!(
        "Failed to resolve template file `{file_path}` from `{}`",
        manifest_dir.display()
    ))
}

fn has_pax_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path.is_ident("pax"))
}

fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        Type::Tuple(tuple) if tuple.elems.is_empty() => "()".to_string(),
        _ => "<unsupported>".to_string(),
    }
}

fn parse_pascal_identifiers_from_component_definition_string(
    pax: &str,
) -> std::result::Result<Vec<String>, String> {
    let pax_component_definition = parse_pax_str(Rule::pax_component_definition, pax)?;
    let pascal_identifiers: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(HashSet::new()));

    pax_component_definition
        .into_inner()
        .for_each(|pair| match pair.as_rule() {
            Rule::root_tag_pair => {
                recurse_visit_tag_pairs_for_pascal_identifiers(
                    pair.into_inner().next().unwrap(),
                    Rc::clone(&pascal_identifiers),
                );
            }
            _ => {}
        });

    let identifiers = Rc::try_unwrap(pascal_identifiers).unwrap().into_inner();
    Ok(identifiers.into_iter().collect())
}

fn recurse_visit_tag_pairs_for_pascal_identifiers(
    any_tag_pair: Pair<Rule>,
    pascal_identifiers: Rc<RefCell<HashSet<String>>>,
) {
    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            let matched_tag = any_tag_pair;
            let open_tag = matched_tag.clone().into_inner().next().unwrap();
            let pascal_identifier = open_tag.into_inner().next().unwrap().as_str();
            pascal_identifiers
                .borrow_mut()
                .insert(pascal_identifier.to_string());

            let prospective_inner_nodes = matched_tag.clone().into_inner().nth(1).unwrap();
            if prospective_inner_nodes.as_rule() == Rule::inner_nodes {
                prospective_inner_nodes.into_inner().for_each(|sub_tag_pair| {
                    match sub_tag_pair.as_rule() {
                        Rule::matched_tag | Rule::self_closing_tag | Rule::statement_control_flow => {
                            recurse_visit_tag_pairs_for_pascal_identifiers(
                                sub_tag_pair,
                                Rc::clone(&pascal_identifiers),
                            );
                        }
                        Rule::node_inner_content | Rule::comment => {}
                        _ => unreachable!(),
                    }
                });
            }
        }
        Rule::self_closing_tag => {
            let pascal_identifier = any_tag_pair.into_inner().next().unwrap().as_str();
            pascal_identifiers
                .borrow_mut()
                .insert(pascal_identifier.to_string());
        }
        Rule::statement_control_flow => {
            let matched_tag = any_tag_pair.into_inner().next().unwrap();
            let inner_index = match matched_tag.as_rule() {
                Rule::statement_if => 1,
                Rule::statement_for => 2,
                Rule::statement_slot => 0,
                _ => unreachable!(),
            };
            let prospective_inner_nodes = matched_tag.into_inner().nth(inner_index).unwrap();
            if prospective_inner_nodes.as_rule() == Rule::inner_nodes {
                prospective_inner_nodes.into_inner().for_each(|sub_tag_pair| {
                    recurse_visit_tag_pairs_for_pascal_identifiers(
                        sub_tag_pair,
                        Rc::clone(&pascal_identifiers),
                    );
                });
            }
        }
        Rule::comment => {}
        _ => unreachable!(),
    }
}

#[derive(Default)]
struct StaticRegistry {
    items_by_import_path: HashMap<String, ScannedPaxItem>,
    items_by_identifier: HashMap<String, Vec<String>>,
    root_package_name: String,
}

impl StaticRegistry {
    fn insert(&mut self, item: ScannedPaxItem) -> Result<()> {
        if self.items_by_import_path.contains_key(&item.import_path) {
            return Err(eyre!(
                "Duplicate static Pax item discovered at `{}`",
                item.import_path
            ));
        }
        self.items_by_identifier
            .entry(item.ident.clone())
            .or_default()
            .push(item.import_path.clone());
        self.items_by_import_path.insert(item.import_path.clone(), item);
        Ok(())
    }

    fn unique_item_below_root(&self, root: &str, identifier: &str) -> Option<String> {
        let candidates = self.items_by_identifier.get(identifier)?;
        let mut matching_candidates = candidates
            .iter()
            .filter(|candidate| candidate.starts_with(&format!("{root}::")))
            .cloned();
        let first = matching_candidates.next()?;
        if matching_candidates.next().is_some() {
            return None;
        }
        Some(first)
    }

    fn root_main_component(&self) -> Result<&ScannedPaxItem> {
        let mut candidates = self
            .items_by_import_path
            .values()
            .filter(|item| {
                item.package_name == self.root_package_name
                    && matches!(
                        item.kind,
                        PaxItemKind::FullComponent {
                            is_main_component: true,
                            ..
                        }
                    )
            })
            .collect::<Vec<_>>();

        if candidates.len() != 1 {
            return Err(eyre!(
                "Expected exactly one root `#[main]` Pax component, found {}",
                candidates.len()
            ));
        }

        Ok(candidates.remove(0))
    }
}

#[derive(Clone)]
struct ScannedPaxItem {
    ident: String,
    import_path: String,
    module_path: String,
    source_path: PathBuf,
    scope: ScopeImports,
    kind: PaxItemKind,
    data: DataSummary,
    engine_import_path: String,
    package_name: String,
}

impl ScannedPaxItem {
    fn type_id(&self) -> TypeId {
        TypeId::build_singleton(&self.import_path, Some(&self.ident))
    }
}

#[derive(Clone)]
enum PaxItemKind {
    FullComponent {
        raw_pax: String,
        is_main_component: bool,
        associated_pax_file_path: Option<PathBuf>,
    },
    Primitive {
        primitive_instance_import_path: String,
    },
    StructOnly,
}

#[derive(Clone)]
enum DataSummary {
    Struct(Vec<FieldSummary>),
    Enum(Vec<EnumVariantSummary>),
}

#[derive(Clone)]
struct FieldSummary {
    name: String,
    ty: Type,
    is_property_wrapped: bool,
}

#[derive(Clone)]
struct EnumVariantSummary {
    field_types: Vec<Type>,
}

#[derive(Clone)]
struct ScopeImports {
    module_path: String,
    import_root: String,
    explicit: HashMap<String, String>,
    glob_roots: Vec<String>,
}

struct PackageContext {
    manifest_dir: PathBuf,
    entry_file: PathBuf,
    package_name: String,
    import_root: String,
}

#[derive(Default)]
struct PaxConfig {
    is_main_component: bool,
    file_path: Option<String>,
    inlined_contents: Option<String>,
    engine_import_path: Option<String>,
    primitive_instance_import_path: Option<String>,
    is_primitive: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_parser_binary;
    use std::sync::{Arc, Mutex};

    #[test]
    fn increment_static_manifest_matches_parser_core() {
        let project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/src/increment");

        let static_manifest = build_manifest(&project_path).expect("static manifest should build");

        let output = run_parser_binary(
            &project_path,
            Arc::new(Mutex::new(vec![])),
            false,
        );
        assert!(
            output.status.success(),
            "parser binary failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let mut manifests: Vec<PaxManifest> =
            serde_json::from_slice(&output.stdout).expect("parser output should be valid JSON");
        let parser_manifest = manifests.remove(0);

        assert_eq!(
            sorted_component_ids(&static_manifest),
            sorted_component_ids(&parser_manifest)
        );
        assert_eq!(sorted_type_ids(&static_manifest), sorted_type_ids(&parser_manifest));
        assert_eq!(static_manifest.engine_import_path, parser_manifest.engine_import_path);
        assert_eq!(static_manifest.assets_dirs, parser_manifest.assets_dirs);

        for import_path in [
            "crate::Example",
            "pax_std::core::group::Group",
            "pax_std::core::text::Text",
            "pax_std::drawing::rectangle::Rectangle",
            "pax_std::drawing::rectangle::RectangleCornerRadii",
            "pax_std::core::text::TextStyle",
            "pax_std::core::text::Font",
            "pax_std::core::text::FontStyle",
            "pax_std::core::text::FontWeight",
            "pax_std::core::text::TextAlignHorizontal",
            "pax_std::core::text::TextAlignVertical",
            "pax_engine::api::Fill",
            "pax_engine::api::Stroke",
            "pax_engine::api::Color",
            "pax_engine::api::Size",
            "pax_engine::api::Numeric",
            "std::string::String",
        ] {
            assert_eq!(
                property_signature(&static_manifest, import_path),
                property_signature(&parser_manifest, import_path),
                "property signature mismatch for {import_path}"
            );
        }
    }

    fn sorted_component_ids(manifest: &PaxManifest) -> Vec<String> {
        let mut ids = manifest
            .components
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    fn sorted_type_ids(manifest: &PaxManifest) -> Vec<String> {
        let mut ids = manifest
            .type_table
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    fn property_signature(manifest: &PaxManifest, import_path: &str) -> Vec<(String, String, bool)> {
        let type_id = manifest
            .type_table
            .keys()
            .find(|type_id| type_id.import_path().as_deref() == Some(import_path))
            .unwrap_or_else(|| panic!("missing type `{import_path}`"));
        let mut signature = manifest
            .type_table
            .get(type_id)
            .unwrap()
            .property_definitions
            .iter()
            .map(|property| {
                (
                    property.name.clone(),
                    property.type_id.to_string(),
                    property.flags.is_property_wrapped,
                )
            })
            .collect::<Vec<_>>();
        signature.sort();
        signature
    }
}
