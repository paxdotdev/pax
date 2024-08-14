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
