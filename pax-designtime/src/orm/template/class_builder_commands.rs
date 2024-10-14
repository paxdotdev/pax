use std::collections::HashMap;

use pax_manifest::{
    LiteralBlockDefinition, PaxManifest, SettingElement, SettingsBlockElement, Token, TypeId,
    ValueDefinition,
};
use serde::{Deserialize, Serialize};

use crate::orm::{Command, ReloadType, Request, Response, Undo, UndoRedoCommand};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateClassRequest {
    containing_component_type_id: TypeId,
    // TODO field definitions
    class_name: String,
    // Used for Undo/Redo
    updated_class_properties: HashMap<String, Option<ValueDefinition>>,
    block_before_changes: Option<SettingsBlockElement>,
}

impl UpdateClassRequest {
    pub fn new(
        containing_component_type_id: TypeId,
        name: String,
        updated_class_properties: HashMap<String, Option<ValueDefinition>>,
    ) -> Self {
        Self {
            containing_component_type_id,
            class_name: name.to_string(),
            updated_class_properties,
            block_before_changes: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateClassResponse {
    pub command_id: Option<usize>,
}

impl Request for UpdateClassRequest {
    type Response = UpdateClassResponse;
}

impl Response for UpdateClassResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }
    fn get_reload_type(&self) -> Option<ReloadType> {
        Some(ReloadType::Full)
    }
}

impl Command<UpdateClassRequest> for UpdateClassRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<UpdateClassResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.containing_component_type_id)
            .ok_or_else(|| {
                format!(
                    "no component with type_id: {:?}",
                    self.containing_component_type_id
                )
            })?;
        let settings = component.settings.get_or_insert_with(Default::default);
        self.block_before_changes = settings
            .iter()
            .find(|v| {
                if let SettingsBlockElement::SelectorBlock(token, _) = v {
                    token.token_value == self.class_name
                } else {
                    false
                }
            })
            .cloned();
        let class_block = match settings.iter_mut().find_map(|v| {
            if let SettingsBlockElement::SelectorBlock(token, block_def) = v {
                (token.token_value == self.class_name).then_some(block_def)
            } else {
                None
            }
        }) {
            Some(block_def) => block_def,
            None => {
                settings.push(SettingsBlockElement::SelectorBlock(
                    Token::new_without_location(self.class_name.clone()),
                    LiteralBlockDefinition::new(vec![]),
                ));
                match settings.last_mut().unwrap() {
                    SettingsBlockElement::SelectorBlock(_, block_def) => block_def,
                    _ => unreachable!("Just inserted a SelectorBlock"),
                }
            }
        };
        for (name, value_definition) in &self.updated_class_properties {
            if let Some(value_definition) = value_definition {
                if let Some(value_def) = class_block.elements.iter_mut().find_map(|v| match v {
                    SettingElement::Setting(token, value_definition)
                        if &token.token_value == name =>
                    {
                        Some(value_definition)
                    }
                    _ => None,
                }) {
                    *value_def = value_definition.clone();
                } else {
                    class_block.elements.push(SettingElement::Setting(
                        Token::new_without_location(name.to_string()),
                        value_definition.clone(),
                    ));
                }
            } else {
                class_block.elements.retain(|v| match v {
                    SettingElement::Setting(token, _) if &token.token_value == name => false,
                    _ => true,
                })
            }
        }
        Ok(UpdateClassResponse { command_id: None })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::UpdateClassRequest(Box::new(self.clone())))
    }
}

impl Undo for UpdateClassRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.containing_component_type_id)
            .ok_or_else(|| {
                format!(
                    "no component with type_id: {:?}",
                    self.containing_component_type_id
                )
            })?;
        let settings = component.settings.get_or_insert_with(Default::default);
        if let Some(before_changes) = &self.block_before_changes {
            let class = settings.iter_mut().find(|v| match &v {
                SettingsBlockElement::SelectorBlock(token, _) => {
                    token.token_value == self.class_name
                }
                _ => false,
            });
            if let Some(class) = class {
                *class = before_changes.clone();
            }
        } else {
            settings.retain(|v| match v {
                SettingsBlockElement::SelectorBlock(token, _) => {
                    token.token_value != self.class_name
                }
                _ => true,
            });
        }
        Ok(())
    }
}
