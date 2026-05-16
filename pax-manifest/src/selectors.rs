use pax_message::serde::{Deserialize, Serialize};

use crate::{LocationInfo, SettingElement, Token, TypeId, ValueDefinition};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub enum SelectorExpr {
    Type(String),
    Class(String),
    Id(String),
}

impl SelectorExpr {
    pub fn parse(selector: &str) -> Result<Self, String> {
        let trimmed = selector.trim();
        if trimmed.is_empty() {
            return Err("selector cannot be empty".to_string());
        }

        if let Some(stripped) = trimmed.strip_prefix('#') {
            validate_selector_identifier(stripped)?;
            return Ok(Self::Id(stripped.to_string()));
        }

        if let Some(stripped) = trimmed.strip_prefix('.') {
            validate_selector_identifier(stripped)?;
            return Ok(Self::Class(stripped.to_string()));
        }

        if trimmed
            .split("::")
            .all(|segment| is_selector_identifier(segment))
        {
            return Ok(Self::Type(trimmed.to_string()));
        }

        Err(
            "selector must be an id (`#id`), class (`.class`), or element type (`Ellipse` or `crate::Example`)"
                .to_string(),
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub struct TemplateNodeSelectorInfo {
    pub source_location: Option<LocationInfo>,
    pub id: Option<Token>,
    pub classes: Vec<Token>,
}

impl TemplateNodeSelectorInfo {
    pub fn from_inline_settings(
        source_location: Option<LocationInfo>,
        settings: &Option<Vec<SettingElement>>,
    ) -> Self {
        let mut id = None;
        let mut classes = Vec::new();

        if let Some(settings) = settings {
            for setting in settings {
                let SettingElement::Setting(token, value) = setting else {
                    continue;
                };
                let ValueDefinition::Identifier(identifier) = value else {
                    continue;
                };

                match token.token_value.as_str() {
                    "id" => {
                        if id.is_some() {
                            panic!("Specified more than one id inline!");
                        }
                        id = Some(Token {
                            token_value: identifier.name.clone(),
                            token_location: token.token_location.clone(),
                        });
                    }
                    "class" => classes.push(Token {
                        token_value: identifier.name.clone(),
                        token_location: token.token_location.clone(),
                    }),
                    _ => {}
                }
            }
        }

        Self {
            source_location,
            id,
            classes,
        }
    }

    pub fn matches(&self, type_id: &TypeId, selector: &SelectorExpr) -> bool {
        match selector {
            SelectorExpr::Id(id) => self
                .id
                .as_ref()
                .map(|token| token.token_value.as_str() == id.as_str())
                .unwrap_or(false),
            SelectorExpr::Class(class_name) => self
                .classes
                .iter()
                .any(|token| token.token_value.as_str() == class_name.as_str()),
            SelectorExpr::Type(type_name) => {
                type_id.to_string() == *type_name
                    || type_id.get_pascal_identifier().as_deref() == Some(type_name.as_str())
            }
        }
    }
}

fn validate_selector_identifier(identifier: &str) -> Result<(), String> {
    if is_selector_identifier(identifier) {
        Ok(())
    } else {
        Err(format!("invalid selector identifier `{identifier}`"))
    }
}

fn is_selector_identifier(identifier: &str) -> bool {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

#[cfg(test)]
mod tests {
    use crate::{PaxIdentifier, SettingElement, Token, TypeId, ValueDefinition};

    use super::{SelectorExpr, TemplateNodeSelectorInfo};

    #[test]
    fn parses_current_selector_forms() {
        assert_eq!(
            SelectorExpr::parse("#hero").unwrap(),
            SelectorExpr::Id("hero".to_string())
        );
        assert_eq!(
            SelectorExpr::parse(".card").unwrap(),
            SelectorExpr::Class("card".to_string())
        );
        assert_eq!(
            SelectorExpr::parse("crate::Thing").unwrap(),
            SelectorExpr::Type("crate::Thing".to_string())
        );
    }

    #[test]
    fn selector_info_matches_id_class_and_type() {
        let settings = Some(vec![
            SettingElement::Setting(
                Token::new_without_location("id".to_string()),
                ValueDefinition::Identifier(PaxIdentifier::new("hero")),
            ),
            SettingElement::Setting(
                Token::new_without_location("class".to_string()),
                ValueDefinition::Identifier(PaxIdentifier::new("card")),
            ),
        ]);

        let info = TemplateNodeSelectorInfo::from_inline_settings(None, &settings);
        let type_id = TypeId::build_singleton("crate::Thing", Some("Thing"));

        assert!(info.matches(&type_id, &SelectorExpr::Id("hero".to_string())));
        assert!(info.matches(&type_id, &SelectorExpr::Class("card".to_string())));
        assert!(info.matches(&type_id, &SelectorExpr::Type("crate::Thing".to_string())));
        assert!(info.matches(&type_id, &SelectorExpr::Type("Thing".to_string())));
    }
}
