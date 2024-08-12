use crate::{LiteralBlockDefinition, SettingElement, Token, TokenType, ValueDefinition};
use pax_lang::{Pair, Parser, PaxParser, Rule};

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
    let res = match value.as_rule() {
        Rule::literal_value => {
            let literal_value_token =
                Token::new_only_raw(value.as_str().to_string(), TokenType::LiteralValue);
            ValueDefinition::LiteralValue(literal_value_token)
        }
        Rule::literal_object => {
            ValueDefinition::Block(derive_value_definition_from_literal_object_pair(value))
        }
        Rule::expression_body => {
            let expression_token = Token::new_value_raw(
                raw_value
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .to_string(),
                raw_value.to_string(),
                TokenType::Expression,
            );
            ValueDefinition::Expression(expression_token)
        }
        Rule::identifier => {
            let identifier_token =
                Token::new_only_raw(value.as_str().to_string(), TokenType::Identifier);
            ValueDefinition::Identifier(identifier_token)
        }
        _ => {
            return Err(
                "couldn't parse value: didn't match literal, expression, or identifier rules",
            );
        }
    };
    Ok(res)
}

//--------------------------------------------------------------------------------------------------------
// Everything below is very similar to functions in pax-compiler, and should be consolidated at some point
//--------------------------------------------------------------------------------------------------------

fn derive_value_definition_from_literal_object_pair(
    literal_object: Pair<Rule>,
) -> LiteralBlockDefinition {
    let mut literal_object_pairs = literal_object.into_inner();

    if let None = literal_object_pairs.peek() {
        return LiteralBlockDefinition {
            explicit_type_pascal_identifier: None,
            elements: vec![],
        };
    }

    let explicit_type_pascal_identifier = match literal_object_pairs.peek().unwrap().as_rule() {
        Rule::pascal_identifier => {
            let raw_value = literal_object_pairs.next().unwrap();
            let token =
                Token::new_only_raw(raw_value.as_str().to_string(), TokenType::PascalIdentifier);
            Some(token)
        }
        _ => None,
    };

    LiteralBlockDefinition {
        explicit_type_pascal_identifier,
        elements: literal_object_pairs
            .map(|settings_key_value_pair| {
                match settings_key_value_pair.as_rule() {
                    Rule::settings_key_value_pair => {
                        let mut pairs = settings_key_value_pair.into_inner();
                        let setting_key = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_key_token = Token::new_only_raw(
                            setting_key.as_str().to_string(),
                            TokenType::SettingKey,
                        );

                        let raw_value = pairs.peek().unwrap().as_str();
                        let value = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_value_definition = match value.as_rule() {
                            Rule::literal_value => {
                                let token = Token::new_only_raw(
                                    raw_value.to_string(),
                                    TokenType::LiteralValue,
                                );
                                ValueDefinition::LiteralValue(token)
                            }
                            Rule::literal_object => {
                                ValueDefinition::Block(
                                    //Recurse
                                    derive_value_definition_from_literal_object_pair(value),
                                )
                            }
                            // Rule::literal_enum_value => {ValueDefinition::Enum(raw_value.as_str().to_string())},
                            Rule::expression_body => {
                                let token = Token::new_only_raw(
                                    raw_value.to_string(),
                                    TokenType::Expression,
                                );
                                ValueDefinition::Expression(token)
                            }
                            _ => {
                                unreachable!("Parsing error 231453468: {:?}", value.as_rule());
                            }
                        };

                        SettingElement::Setting(setting_key_token, setting_value_definition)
                    }
                    Rule::comment => {
                        let comment = settings_key_value_pair.as_str().to_string();
                        SettingElement::Comment(comment)
                    }
                    _ => {
                        unreachable!(
                            "Parsing error 2314314145: {:?}",
                            settings_key_value_pair.as_rule()
                        );
                    }
                }
            })
            .collect(),
    }
}
