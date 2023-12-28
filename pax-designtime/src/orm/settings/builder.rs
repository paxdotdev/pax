use pax_manifest::{LiteralBlockDefinition, SettingsBlockElement};

use crate::orm::{
    settings::{AddSelectorRequest, RemoveSelectorRequest, UpdateSelectorRequest},
    PaxManifestORM,
};

/// Builder for creating and modifying selectors in the PaxManifest.
pub struct SelectorBuilder<'a> {
    orm: &'a mut PaxManifestORM,
    component_type_id: String,
    key: String,
    value: LiteralBlockDefinition,
    selector_index: Option<usize>,
    is_new: bool,
}

impl<'a> SelectorBuilder<'a> {
    pub fn new(
        orm: &'a mut PaxManifestORM,
        component_type_id: String,
        key: String,
        value: LiteralBlockDefinition,
    ) -> Self {
        SelectorBuilder {
            orm,
            component_type_id,
            key,
            value,
            selector_index: None,
            is_new: true,
        }
    }

    pub fn retreive_selector(
        orm: &'a mut PaxManifestORM,
        component_type_id: String,
        key: String,
    ) -> Self {
        if let Some(settings) = &orm
            .get_manifest()
            .components
            .get(&component_type_id)
            .unwrap()
            .settings
        {
            if let Some((index, selector)) = settings.iter().enumerate().find(|(_, elem)| {
                if let SettingsBlockElement::SelectorBlock(token, _) = elem {
                    return token.raw_value == key;
                }
                false
            }) {
                let (key, value) = match selector {
                    SettingsBlockElement::SelectorBlock(key, value) => {
                        (key.raw_value.clone(), value.clone())
                    }
                    _ => panic!("Invalid selector type"),
                };
                return SelectorBuilder {
                    orm,
                    component_type_id,
                    key,
                    value,
                    selector_index: Some(index),
                    is_new: false,
                };
            }
        }
        panic!("Selector with key {} not found", key);
    }

    pub fn set_value(&mut self, value: LiteralBlockDefinition) {
        self.value = value;
    }

    pub fn set_index(&mut self, index: usize) {
        self.selector_index = Some(index);
    }

    pub fn save(mut self) -> Result<(), String> {
        if self.is_new {
            let command = AddSelectorRequest {
                component_type_id: self.component_type_id.clone(),
                selector_index: self.selector_index,
                key: self.key,
                value: self.value,
                cached_selector: None,
            };
            self.orm.execute_command(command)?;
            self.is_new = false;
        } else {
            let command = UpdateSelectorRequest {
                component_type_id: self.component_type_id.clone(),
                new_index: self.selector_index,
                key: self.key,
                value: self.value,
                cached_prev_state: None,
                cached_prev_position: None,
            };
            self.orm.execute_command(command)?;
        }
        Ok(())
    }

    pub fn remove(self) -> Result<(), String> {
        let command = RemoveSelectorRequest::new(self.component_type_id.clone(), self.key);
        self.orm.execute_command(command)?;
        Ok(())
    }
}
