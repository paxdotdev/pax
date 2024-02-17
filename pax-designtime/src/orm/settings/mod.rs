use pax_manifest::{LiteralBlockDefinition, PaxManifest, SettingsBlockElement, Token, TokenType};
use serde_derive::{Deserialize, Serialize};

use super::{Command, Request, Response, UndoRedo, UndoRedoCommand};

pub mod builder;
#[cfg(test)]
mod tests;

#[derive(Serialize, Deserialize, Clone)]
pub struct AddSelectorRequest {
    component_type_id: String,
    selector_index: Option<usize>,
    key: String,
    value: LiteralBlockDefinition,
    cached_selector: Option<SettingsBlockElement>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AddSelectorResponse {
    command_id: Option<usize>,
    selector: SettingsBlockElement,
}

impl Request for AddSelectorRequest {
    type Response = AddSelectorResponse;
}

impl Response for AddSelectorResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<AddSelectorRequest> for AddSelectorRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<AddSelectorResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept settings.");
        }

        let selector: SettingsBlockElement = SettingsBlockElement::SelectorBlock(
            Token::new_from_raw_value(self.key.clone(), TokenType::SettingKey),
            self.value.clone(),
        );

        let id = component.settings.clone().unwrap().len();

        if component.settings.is_none() {
            component.settings = Some(vec![]);
        }

        component.settings.as_mut().unwrap().push(selector.clone());

        self.selector_index = Some(id);
        self.cached_selector = Some(selector.clone());

        Ok(AddSelectorResponse {
            command_id: None,
            selector,
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::AddSelectorRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl UndoRedo for AddSelectorRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        if let Some(id) = self.selector_index {
            component.settings.as_mut().unwrap().remove(id);
            if component.settings.is_some() && component.settings.as_ref().unwrap().is_empty() {
                component.settings = None;
            }
        }
        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        if let Some(id) = self.selector_index {
            if component.settings.is_none() {
                component.settings = Some(vec![]);
            }
            component
                .settings
                .as_mut()
                .unwrap()
                .insert(id, self.cached_selector.clone().unwrap());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateSelectorRequest {
    component_type_id: String,
    new_index: Option<usize>,
    key: String,
    value: LiteralBlockDefinition,
    // Filled in on execute in order to undo
    cached_prev_state: Option<SettingsBlockElement>,
    cached_prev_position: Option<usize>,
}

pub struct UpdateSelectorResponse {
    command_id: Option<usize>,
    #[allow(unused)]
    selector: SettingsBlockElement,
}

impl Request for UpdateSelectorRequest {
    type Response = UpdateSelectorResponse;
}

impl Response for UpdateSelectorResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<UpdateSelectorRequest> for UpdateSelectorRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<UpdateSelectorResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't have settings.");
        }

        self.cached_prev_position = component
            .settings
            .clone()
            .unwrap()
            .iter_mut()
            .position(|s| {
                if let SettingsBlockElement::SelectorBlock(key, _) = s {
                    key.raw_value == self.key
                } else {
                    false
                }
            });

        let new_setting = SettingsBlockElement::SelectorBlock(
            Token::new_from_raw_value(self.key.clone(), TokenType::SettingKey),
            self.value.clone(),
        );

        if let Some(old_index) = self.cached_prev_position {
            let setting = component
                .settings
                .as_mut()
                .unwrap()
                .get_mut(old_index)
                .unwrap();
            self.cached_prev_state = Some(setting.clone());

            if let Some(new_index) = self.new_index {
                component.settings.as_mut().unwrap().remove(old_index);
                component
                    .settings
                    .as_mut()
                    .unwrap()
                    .insert(new_index, new_setting.clone());
            } else {
                *setting = new_setting.clone();
            }
        } else {
            panic!("No setting found with key {}", self.key);
        }

        Ok(UpdateSelectorResponse {
            command_id: None,
            selector: new_setting,
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::UpdateSelectorRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl UndoRedo for UpdateSelectorRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        if let Some(new_index) = self.new_index {
            component.settings.as_mut().unwrap().remove(new_index);
        } else {
            component
                .settings
                .as_mut()
                .unwrap()
                .remove(self.cached_prev_position.unwrap());
        }
        component.settings.as_mut().unwrap().insert(
            self.cached_prev_position.unwrap(),
            self.cached_prev_state.clone().unwrap(),
        );
        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        component
            .settings
            .as_mut()
            .unwrap()
            .remove(self.cached_prev_position.unwrap());
        let new_setting = SettingsBlockElement::SelectorBlock(
            Token::new_from_raw_value(self.key.clone(), TokenType::SettingKey),
            self.value.clone(),
        );

        if let Some(new_index) = self.new_index {
            component
                .settings
                .as_mut()
                .unwrap()
                .insert(new_index, new_setting);
        } else {
            component
                .settings
                .as_mut()
                .unwrap()
                .insert(self.cached_prev_position.unwrap(), new_setting);
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemoveSelectorRequest {
    component_type_id: String,
    key: String,
    // Filled in on execute in order to undo
    cached_prev_state: Option<SettingsBlockElement>,
    cached_prev_position: Option<usize>,
}

impl RemoveSelectorRequest {
    pub fn new(component_type_id: String, key: String) -> Self {
        RemoveSelectorRequest {
            component_type_id,
            key,
            cached_prev_state: None,
            cached_prev_position: None,
        }
    }
}

pub struct RemoveSelectorResponse {
    command_id: Option<usize>,
}

impl Request for RemoveSelectorRequest {
    type Response = RemoveSelectorResponse;
}

impl Response for RemoveSelectorResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<RemoveSelectorRequest> for RemoveSelectorRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<RemoveSelectorResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }

        self.cached_prev_position = component
            .settings
            .clone()
            .unwrap()
            .iter_mut()
            .position(|s| {
                if let SettingsBlockElement::SelectorBlock(key, _) = s {
                    key.raw_value == self.key
                } else {
                    false
                }
            });

        if let Some(index) = self.cached_prev_position {
            let setting = component.settings.as_mut().unwrap().get_mut(index);
            self.cached_prev_state = Some(setting.unwrap().clone());
            component.settings.as_mut().unwrap().remove(index);
        } else {
            panic!("No setting found with key {}", self.key);
        }

        if component.settings.is_some() && component.settings.as_ref().unwrap().is_empty() {
            component.settings = None;
        }

        Ok(RemoveSelectorResponse { command_id: None })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::RemoveSelectorRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl UndoRedo for RemoveSelectorRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if let Some(index) = self.cached_prev_position {
            if component.settings.is_none() {
                component.settings = Some(vec![]);
            }
            component
                .settings
                .as_mut()
                .unwrap()
                .insert(index, self.cached_prev_state.clone().unwrap());
        }
        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        if let Some(index) = self.cached_prev_position {
            component.settings.as_mut().unwrap().remove(index);
        }
        if component.settings.is_some() && component.settings.as_ref().unwrap().is_empty() {
            component.settings = None;
        }
        Ok(())
    }
}

pub struct GetSelectorRequest {
    component_type_id: String,
    key: String,
}

pub struct GetSelectorResponse {
    command_id: Option<usize>,
    #[allow(unused)]
    selector: Option<SettingsBlockElement>,
}

impl Request for GetSelectorRequest {
    type Response = GetSelectorResponse;
}

impl Response for GetSelectorResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<GetSelectorRequest> for GetSelectorRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<<GetSelectorRequest as Request>::Response, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        let selector = component.settings.as_mut().unwrap().iter_mut().find(|s| {
            if let SettingsBlockElement::SelectorBlock(key, _) = s {
                key.raw_value == self.key
            } else {
                false
            }
        });

        let ret = selector.map(|s| s.clone());

        Ok(GetSelectorResponse {
            command_id: None,
            selector: ret,
        })
    }

    fn is_mutative(&self) -> bool {
        false
    }
}

pub struct GetAllSelectorsRequest {
    component_type_id: String,
}

pub struct GetAllSelectorsResponse {
    command_id: Option<usize>,
    #[allow(unused)]
    selectors: Option<Vec<SettingsBlockElement>>,
}

impl Request for GetAllSelectorsRequest {
    type Response = GetAllSelectorsResponse;
}

impl Response for GetAllSelectorsResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<GetAllSelectorsRequest> for GetAllSelectorsRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<<GetAllSelectorsRequest as Request>::Response, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        let selectors = component.settings.as_ref().map(|s| s.to_vec());

        Ok(GetAllSelectorsResponse {
            command_id: None,
            selectors,
        })
    }

    fn is_mutative(&self) -> bool {
        false
    }
}
