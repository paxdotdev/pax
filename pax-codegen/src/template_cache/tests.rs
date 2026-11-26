use super::*;
use pax_manifest::parsing::ParsingContext;
use pax_manifest::{ComponentTemplate, RouteBranchDescriptor, SettingsBlockElement};

fn context() -> TemplateNodeParseContext {
    let label = TypeId::build_singleton("crate::Text", Some("Text"));
    TemplateNodeParseContext {
        template: ComponentTemplate::new(
            TypeId::build_singleton("crate::App", None),
            Some("App.pax".into()),
        ),
        content_properties: [(label.clone(), "text".into())].into(),
        pascal_identifier_to_type_id_map: [("Text".into(), label)].into(),
        route_branch_descriptors: Default::default(),
    }
}

#[cfg(unix)]
#[test]
fn source_bytes_and_component_identity_govern_reuse() {
    let dir = tempfile::tempdir().unwrap();
    let slot = |path, owner, text| {
        Entry::new(
            dir.path(),
            path,
            &TypeId::build_singleton(owner, None),
            text,
        )
        .unwrap()
    };
    let first = slot("app.pax", "crate::App", "<Text/>");
    let changed = slot("app.pax", "crate::App", " <Text/>");
    assert_eq!(first.path, changed.path); // edits replace the same slot
    assert_ne!(first.source, changed.source); // even offsets in diagnostics matter
    for other in [
        slot("app.pax", "crate::Other", "<Text/>"),
        slot("moved.pax", "crate::App", "<Text/>"),
    ] {
        assert_ne!(first.path, other.path);
        assert_ne!(first.source, other.source);
    }
}

#[test]
fn revalidates_template_inputs_and_recovers_from_a_bad_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let entry = Entry {
        path: dir.path().join("slot"),
        source: [42; 32],
    };
    let tags = || Ok::<_, ()>(vec!["Text".to_owned()]);
    assert_eq!(entry.tags(tags).unwrap(), ["Text"]);
    assert_eq!(entry.tags(|| Err(())).unwrap(), ["Text"]);
    let changed = Entry {
        path: entry.path.clone(),
        source: [43; 32],
    };
    assert!(changed.tags(|| Err(())).is_err());
    let path = entry.path.with_extension("tags");
    let mut damaged = fs::read(&path).unwrap();
    // Still valid JSON, but not the data corresponding to the receipt.
    let len = damaged.len();
    damaged[len - 3] = b'X';
    fs::write(&path, damaged).unwrap();
    assert_eq!(entry.tags(tags).unwrap(), ["Text"]);

    let mut ctx = context();
    let original = entry.bindings(&ctx).unwrap();
    let id = ctx.pascal_identifier_to_type_id_map["Text"].clone();
    ctx.content_properties.insert(id.clone(), "value".into());
    assert_ne!(entry.bindings(&ctx).unwrap(), original);
    let mut ctx = context();
    ctx.route_branch_descriptors
        .insert(id, RouteBranchDescriptor::default());
    assert_ne!(entry.bindings(&ctx).unwrap(), original);
    let mut ctx = context();
    ctx.pascal_identifier_to_type_id_map
        .insert("Text".into(), TypeId::build_singleton("other::Text", None));
    assert_ne!(entry.bindings(&ctx).unwrap(), original);
    let unavailable = Entry {
        path: path.join("not-a-directory"),
        source: entry.source,
    };
    assert_eq!(unavailable.tags(tags).unwrap(), ["Text"]);
}

#[test]
fn unrelated_receivers_do_not_invalidate_a_template() {
    use pax_manifest::{PropertyDefinition, TypeDefinition, TypeTable};
    let receiver = |name: &str| {
        let mut p = PropertyDefinition::default();
        p.flags.is_content = true;
        p.name = name.into();
        TypeDefinition {
            property_definitions: vec![p],
            ..Default::default()
        }
    };
    let text = TypeId::build_singleton("crate::Text", None);
    let owner = TypeId::build_singleton("crate::App", None);
    let make = |types: &TypeTable| {
        TemplateNodeParseContext::for_component(
            types,
            [("Text".into(), text.clone())].into(),
            Default::default(),
            owner.clone(),
            "app.pax".into(),
        )
    };
    let mut types: TypeTable = [(text.clone(), receiver("text"))].into();
    let entry = Entry {
        path: "slot".into(),
        source: [0; 32],
    };
    let first = make(&types);
    assert_eq!(first.content_properties[&text], "text");
    types.insert(
        TypeId::build_singleton("other::Badge", None),
        receiver("label"),
    );
    assert_eq!(entry.bindings(&first), entry.bindings(&make(&types)));
    types.insert(text.clone(), receiver("label"));
    assert_ne!(entry.bindings(&first), entry.bindings(&make(&types)));
}

#[test]
fn cached_lowering_links_current_rust_lifecycle_methods() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("lib.rs");
    let entry = Entry {
        path: dir.path().join("slot"),
        source: [1; 32],
    };
    let text = "<Text>Click me</Text>";
    let parsed = entry.lower(text, context());
    let mount_handlers = |lowered: LoweredTemplate| {
        let definition = lowered.link(
            &mut ParsingContext::default(),
            true,
            None,
            "crate",
            TypeId::build_singleton("crate::App", None),
            source.to_str().unwrap(),
        );
        definition
            .settings
            .unwrap()
            .into_iter()
            .filter_map(|s| match s {
                SettingsBlockElement::Handler(k, _) => Some(k.token_value),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    fs::write(
        &source,
        "impl App { pub fn mount(&mut self, ctx: &NodeContext) {} }",
    )
    .unwrap();
    assert_eq!(mount_handlers(parsed), ["mount"]);
    // Reload the stored intermediate rather than parsing anything. Implicit
    // hooks must not have been persisted with the previous Rust definition.
    let hit = Entry::read(
        &entry.path.with_extension("template"),
        &entry.bindings(&context()).unwrap(),
    )
    .unwrap();
    fs::write(
        &source,
        "impl App { pub fn tick(&mut self, ctx: &NodeContext) {} }",
    )
    .unwrap();
    assert_eq!(mount_handlers(hit), ["tick"]);
}
