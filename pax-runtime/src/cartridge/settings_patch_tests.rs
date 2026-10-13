use super::*;
use crate::{ExpandedNodeIdentifier, RuntimePropertyDefinition};

fn entry(value: f64) -> RuntimeResolvedPropertyEntry {
    runtime_property_entry(
        RuntimeSettingsSource::Inline,
        None,
        None,
        None,
        ValueDefinition::LiteralValue(value.to_pax_value()),
        None,
        None,
    )
}
fn columns(entries: Vec<RuntimeResolvedPropertyEntry>) -> RuntimeResolvedPropertyColumns {
    BTreeMap::from([("x".into(), entries)])
}

#[test]
fn patch_preserves_order_shares_definitions_and_explicitly_resets_removals() {
    let old = columns(vec![entry(1.), entry(2.)]);
    assert!(SettingsPatch::between(&old, &old).columns.is_empty());
    let mut new = old.clone();
    new.get_mut("x").unwrap().reverse();
    let patch = SettingsPatch::between(&old, &new);
    assert!(Rc::ptr_eq(
        &patch.columns["x"][0].definition,
        &new["x"][0].definition
    ));
    assert_eq!(patch.columns.len(), 1);
    let removed = SettingsPatch::between(&old, &BTreeMap::new());
    assert!(removed.columns["x"].is_empty());
    assert!(SettingsPatch::between(&removed.columns, &BTreeMap::new())
        .columns
        .is_empty());
    let added = SettingsPatch::between(&BTreeMap::new(), &new);
    assert_eq!(added.columns["x"].len(), 2);
}

#[test]
fn patch_compares_definition_and_runtime_context_instead_of_evaluated_values() {
    let a = entry(1.);
    let old = columns(vec![a.clone()]);
    let changed = |b| {
        assert_eq!(
            SettingsPatch::between(&old, &columns(vec![b]))
                .columns
                .len(),
            1
        )
    };
    // Equal-valued but newly prepared syntax must not conceal a template revision.
    changed(entry(1.));
    let mut b = a.clone();
    Rc::make_mut(&mut b.definition).axis_index = Some(0);
    changed(b);
    let mut b = a.clone();
    b.source_stack = Some(RuntimePropertiesStackFrame::new(HashMap::new()));
    changed(b);
    let mut b = a.clone();
    b.source = RuntimeSettingsSource::ImportedLayer {
        provider_id: ExpandedNodeIdentifier(1),
        provider_type_id: TypeId::default(),
    };
    changed(b);
    let mut b = a.clone();
    b.transition = Some(LocalProperty::new(None));
    changed(b.clone());
    let owner = columns(vec![b.clone()]);
    assert!(SettingsPatch::between(&owner, &owner).columns.is_empty());
    b.transition = Some(LocalProperty::new(None));
    assert_eq!(
        SettingsPatch::between(&owner, &columns(vec![b]))
            .columns
            .len(),
        1
    );
    let mut b = a.clone();
    b.definition = Rc::new(RuntimePropertyDefinition {
        condition: Some(RuntimeSettingsCondition {
            positive: vec![ExpressionInfo::new(
                pax_language::parse_pax_expression("enabled").unwrap(),
            )],
            negative: vec![],
        }),
        ..(*a.definition).clone()
    });
    changed(b);
}
