//! Static validation of appearance values at their expected property boundary.
use crate::{SettingElement, ValueDefinition};
use pax_runtime_api::{
    CoercionRules, Fill, Material, Opacity, Paint, PaxValue, Size, Stroke, StrokeCap, StrokeJoin,
};

// Preserve the structural facts of wrapped PAXEL objects/lists even when
// their fields depend on runtime values. Do not execute application helpers.
fn expression_shape(value: &ValueDefinition) -> Option<ValueDefinition> {
    use crate::{ExpressionInfo, LiteralBlockDefinition, Token};
    use pax_language::interpreter::{PaxExpression, PaxPrimary};
    let ValueDefinition::Expression(info) = value else {
        return None;
    };
    let PaxExpression::Primary(primary) = &info.expression else {
        return None;
    };
    let wrap = |expression: &PaxExpression| {
        let value = ValueDefinition::Expression(ExpressionInfo::new(expression.clone()));
        expression_shape(&value).unwrap_or(value)
    };
    let block = |fields: &Vec<(String, PaxExpression)>, name: Option<String>| {
        ValueDefinition::Block(LiteralBlockDefinition {
            explicit_type_pascal_identifier: name.map(Token::new_without_location),
            elements: fields
                .iter()
                .map(|(name, expression)| {
                    SettingElement::Setting(
                        Token::new_without_location(name.clone()),
                        wrap(expression),
                    )
                })
                .collect(),
        })
    };
    match primary.as_ref() {
        PaxPrimary::Literal(value) => Some(ValueDefinition::LiteralValue(value.clone())),
        PaxPrimary::List(values) => Some(ValueDefinition::List(values.iter().map(wrap).collect())),
        PaxPrimary::Object(fields) => Some(block(fields, None)),
        PaxPrimary::Grouped(value, None) => expression_shape(&ValueDefinition::Expression(
            ExpressionInfo::new((**value).clone()),
        )),
        PaxPrimary::FunctionOrEnum(name, variant, args)
            if matches!(name.as_str(), "Fill" | "Stroke") =>
        {
            if variant == "__layer" {
                if let Some(PaxExpression::Primary(value)) = args.first() {
                    if let PaxPrimary::Object(fields) = value.as_ref() {
                        return Some(block(fields, Some(name.clone())));
                    }
                }
            }
            Some(ValueDefinition::LiteralValue(PaxValue::Enum(Box::new((
                name.clone(),
                variant.clone(),
                vec![],
            )))))
        }
        _ => None,
    }
}

/// Checks the statically known parts of a fill/stroke stack. Dynamic bindings
/// are checked by the same layer coercions when their values become available.
pub fn validate_appearance(value: &ValueDefinition, stroke: bool) -> Result<(), String> {
    fn one(value: &ValueDefinition, stroke: bool) -> Result<(), String> {
        if let Some(value) = expression_shape(value) {
            return one(&value, stroke);
        }
        let expected = if stroke { "Stroke" } else { "Fill" };
        match value {
            ValueDefinition::List(_) => Err("nested appearance lists are unsupported".into()),
            ValueDefinition::LiteralValue(value) => {
                if matches!(value, PaxValue::Vec(_)) {
                    return Err("nested appearance lists are unsupported".into());
                }
                if stroke {
                    Stroke::try_coerce(value.clone()).map(|_| ())
                } else {
                    Fill::try_coerce(value.clone()).map(|_| ())
                }
            }
            ValueDefinition::Block(block) => {
                if let Some(name) = &block.explicit_type_pascal_identifier {
                    if name.token_value != expected {
                        return Err(format!(
                            "expected {expected}, received {}",
                            name.token_value
                        ));
                    }
                }
                for field in &block.elements {
                    let SettingElement::Setting(name, value) = field else {
                        continue;
                    };
                    let name = &name.token_value;
                    let known = matches!(name.as_str(), "paint" | "material" | "opacity")
                        || (stroke && matches!(name.as_str(), "width" | "cap" | "join"));
                    if !known {
                        return Err(if name == "color" {
                            format!("{expected}.color was renamed to paint")
                        } else {
                            format!("unknown {expected} field `{name}`")
                        });
                    }
                    if let ValueDefinition::LiteralValue(value) = value {
                        let result = match name.as_str() {
                            "paint" => Paint::try_coerce(value.clone()).map(|_| ()),
                            "material" => Material::try_coerce(value.clone()).map(|_| ()),
                            "opacity" => Opacity::try_coerce(value.clone()).map(|_| ()),
                            "cap" => StrokeCap::try_coerce(value.clone()).map(|_| ()),
                            "join" => StrokeJoin::try_coerce(value.clone()).map(|_| ()),
                            "width" => Size::try_coerce(value.clone()).and_then(|value| {
                                if matches!(value, Size::Pixels(_)) {
                                    Ok(())
                                } else {
                                    Err("expected logical pixels".into())
                                }
                            }),
                            _ => unreachable!(),
                        };
                        result.map_err(|error| format!("{expected}.{name}: {error}"))?;
                    }
                }
                Ok(())
            }
            ValueDefinition::Timeline(track) => {
                for element in &track.elements {
                    if let crate::TimelineTrackElement::Keyframe(keyframe) = element {
                        validate_appearance(&keyframe.value, stroke)?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    if let Some(value) = expression_shape(value) {
        return validate_appearance(&value, stroke);
    }
    match value {
        ValueDefinition::List(values) => {
            for (i, value) in values.iter().enumerate() {
                one(value, stroke).map_err(|error| format!("entry [{i}]: {error}"))?;
            }
            Ok(())
        }
        ValueDefinition::LiteralValue(PaxValue::Vec(values)) => {
            for (i, value) in values.iter().enumerate() {
                one(&ValueDefinition::LiteralValue(value.clone()), stroke)
                    .map_err(|error| format!("entry [{i}]: {error}"))?;
            }
            Ok(())
        }
        value => one(value, stroke),
    }
}

#[cfg(all(test, feature = "parsing"))]
mod tests {
    use super::*;
    use crate::{
        ComponentDefinition, ComponentTemplate, PaxManifest, TemplateNodeDefinition, Token, TypeId,
    };
    use std::collections::BTreeMap;

    fn parse(source: &str) -> ValueDefinition {
        let pair =
            pax_language::parse_pax_str(pax_language::Rule::any_template_value, source).unwrap();
        crate::parsing::parse_value_definition(pair.into_inner().next().unwrap())
    }

    #[test]
    fn mixed_layers_preserve_live_values_types_and_formatting() {
        let source =
            "[Fill {paint: {self.accent}, opacity: 50%}, @gradient {0%: RED, 100%: BLUE}, GREEN]";
        let value = parse(source);
        let ValueDefinition::List(entries) = &value else {
            panic!("list lost recursive values");
        };
        assert_eq!(entries.len(), 3);
        let ValueDefinition::Block(block) = &entries[0] else {
            panic!("layer lost type");
        };
        assert_eq!(
            block
                .explicit_type_pascal_identifier
                .as_ref()
                .unwrap()
                .token_value,
            "Fill"
        );
        validate_appearance(&value, false).unwrap();
        let formatted = value.to_string();
        validate_appearance(&parse(&formatted), false).unwrap();
        let template = format!("<Rectangle fill={source} />");
        let formatted_template = pax_language::formatting::format_pax_template(template).unwrap();
        assert_eq!(
            pax_language::formatting::format_pax_template(formatted_template.clone()).unwrap(),
            formatted_template
        );
    }

    #[test]
    fn layer_errors_retain_entry_context() {
        for source in [
            "[RED, Stroke {paint: BLUE}]",
            "[RED, {color: BLUE}]",
            "[RED, []]",
            "[RED, None]",
            "[RED, Fill {unknown: 1}]",
            "{[RED, Stroke {paint: BLUE}]}",
            "{[RED, Fill {unknown: self.live}]}",
        ] {
            let error = validate_appearance(&parse(source), false).unwrap_err();
            assert!(error.contains("[1]"), "{source}: {error}");
        }
    }

    #[test]
    fn recursive_values_survive_rich_and_baked_programs() {
        let value = parse("[Fill {paint: {self.accent}}, @gradient {0%: RED, 100%: BLUE}]");
        let ty = TypeId::build_singleton("crate::App", Some("App"));
        let mut template = ComponentTemplate::new(ty.clone(), Some("src/lib.pax".into()));
        template.add(TemplateNodeDefinition {
            type_id: ty.clone(),
            settings: Some(vec![SettingElement::Setting(
                Token::new_without_location("fill".into()),
                value.clone(),
            )]),
            ..Default::default()
        });
        let manifest = PaxManifest {
            components: BTreeMap::from([(
                ty.clone(),
                ComponentDefinition {
                    type_id: ty.clone(),
                    is_main_component: true,
                    is_primitive: false,
                    is_struct_only_component: false,
                    module_path: "crate".into(),
                    primitive_instance_import_path: None,
                    template: Some(template),
                    settings: None,
                    timelines: vec![],
                    route_branch: None,
                },
            )]),
            main_component_type_id: ty.clone(),
            type_table: Default::default(),
            assets_dirs: vec![],
            engine_import_path: "pax_engine".into(),
        };
        let rich = crate::binary::from_slice(&crate::binary::to_vec(&manifest).unwrap()).unwrap();
        let program = crate::program_ir::ProgramIR::from_manifest(&rich);
        let baked = crate::program_ir::binary::from_slice(
            &crate::program_ir::binary::to_vec(&program).unwrap(),
        )
        .unwrap();
        let template = baked.components[&ty].template.as_ref().unwrap();
        let node = &template.get_nodes()[0];
        let SettingElement::Setting(_, actual) = &node.settings.as_ref().unwrap()[0] else {
            panic!()
        };
        assert_eq!(actual.to_string(), value.to_string());
        #[cfg(feature = "compiler")]
        assert!(crate::rust_manifest::to_rust_expression(&rich).contains("ValueDefinition::List"));
    }
}
