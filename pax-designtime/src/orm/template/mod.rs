use pax_manifest::{
    ControlFlowSettingsDefinition, PaxManifest, SettingElement, TemplateNodeDefinition,
};

use super::{Command, Request, Response};

#[cfg(test)]
mod tests;

pub enum NodeType {
    Template(Vec<SettingElement>),
    ControlFlow(ControlFlowSettingsDefinition),
    Comment(String),
}

pub struct AddTemplateNodeRequest {
    component_type_id: String,
    parent_node_id: usize,
    node_id: Option<usize>,
    child_ids: Vec<usize>,
    type_id: String,
    node_type: NodeType,
    pascal_identifier: String,
    cached_node: Option<TemplateNodeDefinition>,
}

pub struct AddTemplateNodeResponse {
    command_id: Option<usize>,
    template_node: TemplateNodeDefinition,
}

impl Request for AddTemplateNodeRequest {
    type Response = AddTemplateNodeResponse;
}

impl Response for AddTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<AddTemplateNodeRequest> for AddTemplateNodeRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<AddTemplateNodeResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }
        let next_id = if let Some(next_id) = component.next_template_id {
            next_id
        } else {
            1
        };
        self.node_id = Some(next_id);

        let template = if let Some(t) = &mut component.template {
            t
        } else {
            unreachable!("No available template.")
        };

        let control_flow_settings = if let NodeType::ControlFlow(c) = &self.node_type {
            Some(c.clone())
        } else {
            None
        };

        let settings = if let NodeType::Template(s) = &self.node_type {
            Some(s.clone())
        } else {
            None
        };

        let raw_comment_string = if let NodeType::Comment(c) = &self.node_type {
            Some(c.clone())
        } else {
            None
        };

        let new_node = TemplateNodeDefinition {
            id: next_id,
            child_ids: self.child_ids.clone(),
            type_id: self.type_id.clone(),
            control_flow_settings,
            settings,
            pascal_identifier: self.pascal_identifier.clone(),
            raw_comment_string,
        };

        template.insert(next_id, new_node.clone());

        if let Some(parent) = template.get_mut(&self.parent_node_id) {
            parent.child_ids.push(next_id);
        }

        component.next_template_id = Some(next_id + 1);

        self.cached_node = Some(new_node.clone());

        Ok(AddTemplateNodeResponse {
            command_id: None,
            template_node: new_node,
        })
    }

    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        let id = self.node_id.unwrap();
        if let Some(template) = &mut component.template {
            template.remove(&id);
            let parent = template.get_mut(&self.parent_node_id).unwrap();
            parent.child_ids.retain(|id| *id != self.node_id.unwrap());
        }
        component.next_template_id = Some(id);
        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        let id = self.node_id.unwrap();
        if let Some(template) = &mut component.template {
            let node = &self.cached_node.clone().unwrap();
            template.insert(id, node.clone());

            if let Some(parent) = template.get_mut(&self.parent_node_id) {
                parent.child_ids.push(id);
            }

            component.next_template_id = Some(id + 1);
        }
        Ok(())
    }
}

pub struct UpdateTemplateNodeRequest {
    component_type_id: String,
    new_parent: Option<usize>,
    updated_node: TemplateNodeDefinition,
    // Filled in on execute in order to undo
    cached_prev_state: Option<TemplateNodeDefinition>,
    cached_prev_parent: Option<usize>,
    cached_prev_position: Option<usize>,
}

pub struct UpdateTemplateNodeResponse {
    command_id: Option<usize>,
    template_node: TemplateNodeDefinition,
}

impl Request for UpdateTemplateNodeRequest {
    type Response = UpdateTemplateNodeResponse;
}

impl Response for UpdateTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<UpdateTemplateNodeRequest> for UpdateTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<UpdateTemplateNodeResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }
        let id = self.updated_node.id;

        let template = if let Some(t) = &mut component.template {
            t
        } else {
            unreachable!("No available template.")
        };
        self.cached_prev_state = Some(template.get(&id).unwrap().clone());
        template.insert(id, self.updated_node.clone());

        if let Some(new_parent) = self.new_parent {
            let parent_node_ids: Vec<usize> = template
                .iter()
                .filter(|(_, node)| node.child_ids.contains(&id))
                .map(|(&node_id, _)| node_id)
                .collect();

            if let Some(&old_parent_id) = parent_node_ids.first() {
                let old_parent = template.get_mut(&old_parent_id).unwrap();
                if let Some((index, _)) = old_parent
                    .child_ids
                    .iter()
                    .enumerate()
                    .find(|&(_, &val)| val == id)
                {
                    self.cached_prev_position = Some(index);
                } else {
                    unreachable!("Previous parent must contain node.")
                }
                old_parent.child_ids.retain(|&child_id| child_id != id);
                self.cached_prev_parent = Some(old_parent.id);
            } else {
                unreachable!("Requested node is not in the template.");
            }
            let new_parent = template.get_mut(&new_parent).unwrap();
            new_parent.child_ids.push(id);
        }

        Ok(UpdateTemplateNodeResponse {
            command_id: None,
            template_node: self.updated_node.clone(),
        })
    }

    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        let id = self.updated_node.id;
        if let Some(template) = &mut component.template {
            template.insert(id, self.cached_prev_state.clone().unwrap());

            if let Some(new_parent) = self.new_parent {
                let new_parent_to_revert = template.get_mut(&new_parent).unwrap();
                new_parent_to_revert.child_ids.retain(|c| *c != id);
                let old_parent_to_revert =
                    template.get_mut(&self.cached_prev_parent.unwrap()).unwrap();
                old_parent_to_revert
                    .child_ids
                    .insert(self.cached_prev_position.unwrap(), id);
            }
        }
        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        let id = self.updated_node.id;
        if let Some(template) = &mut component.template {
            template.insert(id, self.updated_node.clone());

            if let Some(new_parent) = self.new_parent {
                let new_parent_to_redo = template.get_mut(&new_parent).unwrap();
                new_parent_to_redo.child_ids.push(id);
                let old_parent_to_redo =
                    template.get_mut(&self.cached_prev_parent.unwrap()).unwrap();
                old_parent_to_redo.child_ids.retain(|c| *c != id);
            }
        }
        Ok(())
    }
}

pub struct RemoveTemplateNodeRequest {
    component_type_id: String,
    node_id: usize,
    // Filled in on execute in order to undo
    cached_prev_state: Option<TemplateNodeDefinition>,
    cached_prev_parent: Option<usize>,
    cached_prev_position: Option<usize>,
}

pub struct RemoveTemplateNodeResponse {
    command_id: Option<usize>,
}

impl Request for RemoveTemplateNodeRequest {
    type Response = RemoveTemplateNodeResponse;
}

impl Response for RemoveTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<RemoveTemplateNodeRequest> for RemoveTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<RemoveTemplateNodeResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }
        let id = self.node_id;

        let template = if let Some(t) = &mut component.template {
            t
        } else {
            unreachable!("No available template.")
        };
        self.cached_prev_state = Some(template.get(&id).unwrap().clone());
        template.remove(&id);

        let parent_node_ids: Vec<usize> = template
            .iter()
            .filter(|(_, node)| node.child_ids.contains(&id))
            .map(|(&node_id, _)| node_id)
            .collect();

        if let Some(parent_id) = parent_node_ids.first() {
            let parent = template.get_mut(parent_id).unwrap();
            if let Some((index, _)) = parent
                .child_ids
                .iter()
                .enumerate()
                .find(|&(_, &val)| val == id)
            {
                self.cached_prev_position = Some(index);
            } else {
                unreachable!("Previous parent must contain node.")
            }
            self.cached_prev_parent = Some(*parent_id);
            parent.child_ids.retain(|c| *c != id);
        }

        Ok(RemoveTemplateNodeResponse { command_id: None })
    }

    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        let id = self.node_id;
        if let Some(template) = &mut component.template {
            template.insert(id, self.cached_prev_state.clone().unwrap());

            let parent = template.get_mut(&self.cached_prev_parent.unwrap()).unwrap();
            parent
                .child_ids
                .insert(self.cached_prev_position.unwrap(), id);
        }
        Ok(())
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        let id = self.node_id;
        if let Some(template) = &mut component.template {
            template.remove(&id);
            let parent = template.get_mut(&self.cached_prev_parent.unwrap()).unwrap();
            parent.child_ids.retain(|c| *c != id);
        }
        Ok(())
    }
}

pub struct GetTemplateNodeRequest {
    component_type_id: String,
    node_id: usize,
}

pub struct GetTemplateNodeResponse {
    command_id: Option<usize>,
    node: Option<TemplateNodeDefinition>,
}

impl Request for GetTemplateNodeRequest {
    type Response = GetTemplateNodeResponse;
}

impl Response for GetTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<GetTemplateNodeRequest> for GetTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<<GetTemplateNodeRequest as Request>::Response, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        let id = self.node_id;
        let mut node = None;
        if let Some(template) = &component.template {
            node = if let Some(n) = template.get(&id) {
                Some(n.clone())
            } else {
                None
            };
        }
        Ok(GetTemplateNodeResponse {
            command_id: None,
            node,
        })
    }

    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        unreachable!("Non-mutative command does not support undo.")
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        unreachable!("Non-mutative command does not support redo.")
    }

    fn is_mutative(&mut self) -> bool {
        return false;
    }
}

pub struct GetAllTemplateNodeRequest {
    component_type_id: String,
}

pub struct GetAllTemplateNodeResponse {
    command_id: Option<usize>,
    nodes: Option<Vec<TemplateNodeDefinition>>,
}

impl Request for GetAllTemplateNodeRequest {
    type Response = GetAllTemplateNodeResponse;
}

impl Response for GetAllTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
}

impl Command<GetAllTemplateNodeRequest> for GetAllTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<<GetAllTemplateNodeRequest as Request>::Response, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        let nodes = if let Some(template) = &component.template {
            Some(template.values().into_iter().map(|x| x.clone()).collect())
        } else {
            None
        };

        Ok(GetAllTemplateNodeResponse {
            command_id: None,
            nodes,
        })
    }

    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        unreachable!("Non-mutative command does not support undo.")
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        unreachable!("Non-mutative command does not support redo.")
    }

    fn is_mutative(&mut self) -> bool {
        return false;
    }
}
