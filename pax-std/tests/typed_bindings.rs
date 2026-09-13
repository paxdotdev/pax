use pax_engine::api::pax_value::is_typed_binding_safe;
use pax_engine::api::{CoercionRules, Color, Property, ToPaxValue, Variable};
use pax_engine::*;

#[pax]
#[engine_import_path("pax_engine")]
pub struct Piece {
    pub points: Vec<f64>,
    pub color: Color,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct Panel {
    pub pieces: Vec<Piece>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct Stateful {
    pub value: Property<f64>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct StatefulParent {
    pub children: Vec<Stateful>,
}

#[pax]
#[engine_import_path("pax_engine")]
#[custom(CoercionRules)]
pub struct Custom {
    pub value: f64,
}
impl CoercionRules for Custom {
    fn try_coerce(_: PaxValue) -> Result<Self, String> {
        Ok(Self { value: 42.0 })
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct CustomParent {
    pub children: Vec<Custom>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum Choice {
    #[default]
    Empty,
    Panel(Panel),
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum StatefulChoice {
    #[default]
    Empty,
    Stateful(Stateful),
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct Recursive {
    pub children: Vec<Recursive>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct PartiallyRepresented {
    pub name: String,
    pub omitted: [u8; 4],
}

#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
pub struct CustomDefault {
    pub value: f64,
}
impl Default for CustomDefault {
    fn default() -> Self {
        Self { value: 7.0 }
    }
}

#[test]
fn generated_plain_records_and_enums_opt_in_recursively() {
    assert!(is_typed_binding_safe::<Vec<Panel>>());
    assert!(is_typed_binding_safe::<Choice>());
    let source = Property::new(vec![Panel {
        pieces: vec![Piece {
            points: vec![1.0, 2.0, 3.0],
            color: Color::RED,
        }],
    }]);
    let destination = Variable::new_from_typed_property(source.clone())
        .try_typed_binding::<Vec<Panel>>("panels")
        .unwrap();
    assert_eq!(
        destination.get().to_pax_value(),
        source.get().to_pax_value()
    );
    destination.update(|panels| panels[0].pieces[0].points[0] = 9.0);
    assert_eq!(source.get()[0].pieces[0].points[0], 1.0);
}

#[test]
fn property_wrapped_fields_and_nested_custom_types_keep_conversion() {
    assert!(!is_typed_binding_safe::<Stateful>());
    assert!(!is_typed_binding_safe::<StatefulParent>());
    assert!(!is_typed_binding_safe::<StatefulChoice>());
    assert!(!is_typed_binding_safe::<Custom>());
    assert!(!is_typed_binding_safe::<CustomParent>());
    let original = Stateful {
        value: Property::new(1.0),
    };
    let snapshot = Stateful::try_coerce(original.clone().to_pax_value()).unwrap();
    snapshot.value.set(5.0);
    assert_eq!(original.value.get(), 1.0);
    assert!(Variable::new_from_typed_property(Property::new(original))
        .try_typed_binding::<Stateful>("stateful")
        .is_none());
    assert_eq!(
        Custom::try_coerce(Custom { value: 1.0 }.to_pax_value())
            .unwrap()
            .value,
        42.0
    );
}

#[test]
fn recursive_generated_records_fall_back_without_recursing_forever() {
    assert!(!is_typed_binding_safe::<Recursive>());
    assert!(is_typed_binding_safe::<Panel>());
}

#[test]
fn generated_sparse_object_defaults_remain_unchanged() {
    let value = Piece::try_coerce(PaxValue::Object(vec![(
        "points".into(),
        vec![4.0_f64].to_pax_value(),
    )]))
    .unwrap();
    assert_eq!(value.points, vec![4.0]);
    assert_eq!(value.color, Piece::default().color);
}

#[test]
fn omitted_fields_and_custom_default_prevent_automatic_opt_in() {
    assert!(!is_typed_binding_safe::<PartiallyRepresented>());
    assert!(!is_typed_binding_safe::<CustomDefault>());
    let source = PartiallyRepresented {
        name: "record".into(),
        omitted: [1, 2, 3, 4],
    };
    let roundtrip = PartiallyRepresented::try_coerce(source.to_pax_value()).unwrap();
    assert_eq!(roundtrip.omitted, [0; 4]);
    let sparse = CustomDefault::try_coerce(PaxValue::Object(vec![])).unwrap();
    assert_eq!(sparse.value, 7.0);
}
