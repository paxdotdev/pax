use pax_language::interpreter::property_resolution::IdentifierResolver;
use pax_runtime_api::{Numeric, PaxValue};
use std::collections::HashMap;

#[test]
#[cfg(feature = "parser")]
fn standalone_expression_uses_map_values_without_entering_a_shared_graph() {
    use pax_language::interpreter::compute_paxel;
    use pax_runtime_api::Functions;
    use std::rc::Rc;

    Functions::register_all_functions();
    let values = HashMap::from([("count".to_string(), PaxValue::Numeric(Numeric::I64(3)))]);
    let result = compute_paxel("count * 2", Rc::new(values)).unwrap();
    assert_eq!(result, PaxValue::Numeric(Numeric::I64(6)));
}

#[test]
fn resolved_variables_keep_their_snapshot_after_the_map_changes_or_drops() {
    let mut values = HashMap::from([("count".to_string(), PaxValue::Numeric(Numeric::I64(3)))]);
    let original = values.resolve("count".to_string()).unwrap();
    values.insert("count".to_string(), PaxValue::Numeric(Numeric::I64(5)));
    let updated = values.resolve("count".to_string()).unwrap();
    assert!(matches!(
        values.resolve("missing".to_string()),
        Err(message) if message == "Identifier not found: missing"
    ));
    drop(values);

    assert_eq!(
        original.get_as_pax_value(),
        PaxValue::Numeric(Numeric::I64(3))
    );
    assert_eq!(
        updated.get_as_pax_value(),
        PaxValue::Numeric(Numeric::I64(5))
    );
}
