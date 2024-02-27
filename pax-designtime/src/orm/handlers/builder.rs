use crate::orm::PaxManifestORM;

use super::{AddHandlerRequest, RemoveHandlerRequest, UpdateHandlerRequest};
use pax_manifest::HandlerBindingElement;

/// Builder for creating and modifying handlers in the PaxManifest.
pub struct HandlerBuilder<'a> {
    orm: &'a mut PaxManifestORM,
    component_type_id: String,
    key: String,
    value: Vec<String>,
    handler_index: Option<usize>,
    is_new: bool,
}

impl<'a> HandlerBuilder<'a> {
    pub fn new(orm: &'a mut PaxManifestORM, component_type_id: String, key: String) -> Self {
        HandlerBuilder {
            orm,
            component_type_id,
            key,
            value: Vec::new(),
            handler_index: None,
            is_new: true,
        }
    }

    pub fn retrieve_handler(
        orm: &'a mut PaxManifestORM,
        component_type_id: String,
        key: String,
    ) -> Self {
        if let Some(handlers) = &orm
            .get_manifest()
            .components
            .get(&component_type_id)
            .unwrap()
            .handlers
        {
            if let Some((index, handler)) = handlers.iter().enumerate().find(|(_, elem)| {
                if let HandlerBindingElement::Handler(token, _) = elem {
                    return token.raw_value == key;
                }
                false
            }) {
                let (key, value) = match handler {
                    HandlerBindingElement::Handler(key, value) => {
                        (key.raw_value.clone(), value.clone())
                    }
                    _ => panic!("Invalid handler type"),
                };

                let value = value.iter().map(|token| token.raw_value.clone()).collect();

                return HandlerBuilder {
                    orm,
                    component_type_id,
                    key,
                    value,
                    handler_index: Some(index),
                    is_new: false,
                };
            }
        }
        panic!("Handler with key {} not found", key);
    }

    pub fn set_handler_value(&mut self, value: Vec<String>) {
        self.value = value;
    }

    pub fn set_handler_index(&mut self, index: usize) {
        self.handler_index = Some(index);
    }

    pub fn save(mut self) -> Result<(), String> {
        if self.is_new {
            let add_request = AddHandlerRequest {
                component_type_id: self.component_type_id.clone(),
                handler_index: None,
                key: self.key,
                value: self.value,
            };
            self.orm
                .execute_command(add_request)
                .map_err(|e| format!("Failed to add handler: {}", e))?;

            self.is_new = false;
        } else {
            let update_request = UpdateHandlerRequest {
                component_type_id: self.component_type_id.clone(),
                new_index: self.handler_index,
                key: self.key,
                value: self.value,
                cached_prev_state: None,
                cached_prev_position: None,
            };
            self.orm
                .execute_command(update_request)
                .map_err(|e| format!("Failed to update handler: {}", e))?;
        }

        Ok(())
    }

    pub fn remove(self) -> Result<(), String> {
        let remove_request = RemoveHandlerRequest {
            component_type_id: self.component_type_id.clone(),
            key: self.key,
            cached_prev_state: None,
            cached_prev_position: None,
        };
        self.orm
            .execute_command(remove_request)
            .map_err(|e| format!("Failed to remove handler: {}", e))?;
        Ok(())
    }
}
