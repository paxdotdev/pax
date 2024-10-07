use crate::{parsing::parse_value_definition, ValueDefinition};
use pax_lang::{Parser, PaxParser, Rule};

pub fn parse_value(raw_value: &str) -> Result<ValueDefinition, &str> {
    if raw_value.is_empty() {
        return Err("raw value cannot be empty");
    }
    let mut values =
        PaxParser::parse(Rule::any_template_value, raw_value).map_err(|_| "couldn't parse")?;
    if values.as_str() != raw_value {
        return Err("no rule matched entire raw value");
    }
    let value = values.next().unwrap().into_inner().next().unwrap();
    let res = parse_value_definition(value);
    Ok(res)
}

pub fn valid_class_or_id(class_ident: &str) -> bool {
    if !(class_ident.starts_with('.') || class_ident.starts_with('#')) {
        return false;
    }
    let class_ident = class_ident.trim_start_matches('.').trim_start_matches('#');
    if class_ident.is_empty() {
        return false;
    }
    let Ok(parse) = PaxParser::parse(Rule::identifier, class_ident) else {
        return false;
    };
    parse.as_str() == class_ident
}
