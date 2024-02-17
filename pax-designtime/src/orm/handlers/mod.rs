use pax_manifest::{HandlersBlockElement, PaxManifest, Token, TokenType};
use serde_derive::{Deserialize, Serialize};

use super::{Command, Request, Response, UndoRedo, UndoRedoCommand};

pub mod builder;
#[cfg(test)]
mod tests;

#[derive(Serialize, Deserialize, Clone)]
pub struct AddHandlerRequest {
    component_type_id: String,
    handler_index: Option<usize>,
    key: String,
    value: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AddHandlerResponse {
    command_id: Option<usize>,
    handler: HandlersBlockElement,
}

impl Request for AddHandlerRequest {
    type Response = AddHandlerResponse;
}

impl Response for AddHandlerResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

fn create_handler_element(key: String, value: Vec<String>) -> HandlersBlockElement {
    HandlersBlockElement::Handler(
        Token::new_from_raw_value(key, TokenType::EventId),
        value
            .iter()
            .map(|value| Token::new_from_raw_value(value.clone(), TokenType::Handler))
            .collect(),
    )
}

impl Command<AddHandlerRequest> for AddHandlerRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<AddHandlerResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept handlers.");
        }

        let id = component.handlers.clone().unwrap().len();

        let handler = create_handler_element(self.key.clone(), self.value.clone());

        if component.handlers.is_none() {
            component.handlers = Some(vec![]);
        }

        component.handlers.as_mut().unwrap().push(handler.clone());

        self.handler_index = Some(id);

        Ok(AddHandlerResponse {
            command_id: None,
            handler: handler.clone(),
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::AddHandlerRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl UndoRedo for AddHandlerRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if let Some(handlers) = &mut component.handlers {
            handlers.remove(self.handler_index.unwrap());
            if component.handlers.is_some() && component.handlers.as_ref().unwrap().is_empty() {
                component.handlers = None;
            }
        }

        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        let handler = create_handler_element(self.key.clone(), self.value.clone());

        if let Some(id) = self.handler_index {
            if component.handlers.is_none() {
                component.handlers = Some(vec![]);
            }
            component.handlers.as_mut().unwrap().insert(id, handler);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateHandlerRequest {
    component_type_id: String,
    new_index: Option<usize>,
    key: String,
    value: Vec<String>,
    // Filled in on execute in order to undo
    cached_prev_state: Option<HandlersBlockElement>,
    cached_prev_position: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateHandlerResponse {
    command_id: Option<usize>,
    handler: HandlersBlockElement,
}

impl Request for UpdateHandlerRequest {
    type Response = UpdateHandlerResponse;
}

impl Response for UpdateHandlerResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<UpdateHandlerRequest> for UpdateHandlerRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<UpdateHandlerResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept handlers.");
        }

        let current_id = component
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .position(|handler| {
                if let HandlersBlockElement::Handler(t1, _) = handler {
                    return t1.raw_value == self.key;
                }
                false
            });

        let handler = create_handler_element(self.key.clone(), self.value.clone());

        if let Some(id) = current_id {
            self.cached_prev_state = Some(component.handlers.as_ref().unwrap()[id].clone());
            self.cached_prev_position = Some(id);

            if let Some(new_id) = self.new_index {
                component.handlers.as_mut().unwrap().remove(id);
                component
                    .handlers
                    .as_mut()
                    .unwrap()
                    .insert(new_id, handler.clone());
            } else {
                component.handlers.as_mut().unwrap()[id] = handler.clone();
            }
        } else {
            panic!("Handler not found");
        }
        Ok(UpdateHandlerResponse {
            command_id: None,
            handler,
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::UpdateHandlerRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl UndoRedo for UpdateHandlerRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        let old_id = self.cached_prev_position.unwrap();
        if let Some(new_id) = self.new_index {
            component.handlers.as_mut().unwrap().remove(new_id);
            component
                .handlers
                .as_mut()
                .unwrap()
                .insert(old_id, self.cached_prev_state.as_ref().unwrap().clone());
        } else {
            component.handlers.as_mut().unwrap()[old_id] =
                self.cached_prev_state.as_ref().unwrap().clone();
        }
        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        let current_id = self.cached_prev_position.unwrap();

        let handler = create_handler_element(self.key.clone(), self.value.clone());

        if let Some(new_id) = self.new_index {
            component.handlers.as_mut().unwrap().remove(current_id);
            component.handlers.as_mut().unwrap().insert(new_id, handler);
        } else {
            component.handlers.as_mut().unwrap()[current_id] = handler;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemoveHandlerRequest {
    component_type_id: String,
    key: String,
    // Filled in on execute in order to undo
    cached_prev_state: Option<HandlersBlockElement>,
    cached_prev_position: Option<usize>,
}

impl RemoveHandlerRequest {
    pub fn new(component_type_id: String, handler_key: String) -> Self {
        RemoveHandlerRequest {
            component_type_id,
            key: handler_key,
            cached_prev_state: None,
            cached_prev_position: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemoveHandlerResponse {
    command_id: Option<usize>,
}

impl Request for RemoveHandlerRequest {
    type Response = RemoveHandlerResponse;
}

impl Response for RemoveHandlerResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<RemoveHandlerRequest> for RemoveHandlerRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<RemoveHandlerResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept handlers.");
        }

        let current_id = component
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .position(|handler| {
                if let HandlersBlockElement::Handler(t1, _) = handler {
                    return t1.raw_value == self.key;
                }
                false
            });

        if let Some(id) = current_id {
            self.cached_prev_state = Some(component.handlers.as_ref().unwrap()[id].clone());
            self.cached_prev_position = Some(id);

            if let Some(handlers) = &mut component.handlers {
                handlers.remove(id);
                if component.handlers.is_some() && component.handlers.as_ref().unwrap().is_empty() {
                    component.handlers = None;
                }
            }
        } else {
            panic!("Handler not present");
        }
        Ok(RemoveHandlerResponse { command_id: None })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::RemoveHandlerRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl UndoRedo for RemoveHandlerRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        let old_id = self.cached_prev_position.unwrap();
        if component.handlers.is_none() {
            component.handlers = Some(vec![]);
        }
        component
            .handlers
            .as_mut()
            .unwrap()
            .insert(old_id, self.cached_prev_state.as_ref().unwrap().clone());
        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        let current_id = self.cached_prev_position.unwrap();
        component.handlers.as_mut().unwrap().remove(current_id);
        if component.handlers.is_some() && component.handlers.as_ref().unwrap().is_empty() {
            component.handlers = None;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetHandlerRequest {
    component_type_id: String,
    key: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetHandlerResponse {
    command_id: Option<usize>,
    handler: Option<HandlersBlockElement>,
}

impl Request for GetHandlerRequest {
    type Response = GetHandlerResponse;
}

impl Response for GetHandlerResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<GetHandlerRequest> for GetHandlerRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<GetHandlerResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept handlers.");
        }

        let current_id = component
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .position(|handler| {
                if let HandlersBlockElement::Handler(t1, _) = handler {
                    return t1.raw_value == self.key;
                }
                false
            });

        if let Some(id) = current_id {
            Ok(GetHandlerResponse {
                command_id: None,
                handler: Some(component.handlers.as_ref().unwrap()[id].clone()),
            })
        } else {
            Ok(GetHandlerResponse {
                command_id: None,
                handler: None,
            })
        }
    }
    fn is_mutative(&self) -> bool {
        false
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetAllHandlersRequest {
    component_type_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetAllHandlersResponse {
    command_id: Option<usize>,
    handlers: Option<Vec<HandlersBlockElement>>,
}

impl Request for GetAllHandlersRequest {
    type Response = GetAllHandlersResponse;
}

impl Response for GetAllHandlersResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<GetAllHandlersRequest> for GetAllHandlersRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<GetAllHandlersResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept handlers.");
        }

        Ok(GetAllHandlersResponse {
            command_id: None,
            handlers: component.handlers.clone(),
        })
    }

    fn is_mutative(&self) -> bool {
        false
    }
}
