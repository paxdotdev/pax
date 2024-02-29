use core::panic;

use pax_manifest::{
    ComponentTemplate, NodeLocation, NodeType, PaxManifest, TemplateNodeDefinition, TypeId, UniqueTemplateNodeIdentifier
};
use serde_derive::{Deserialize, Serialize};

use super::{Command, Request, Response, Undo, UndoRedoCommand};

pub mod builder;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct NodeData {
    pub unique_node_identifier: UniqueTemplateNodeIdentifier,
    pub cached_node: TemplateNodeDefinition,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct AddTemplateNodeRequest {
    containing_component_type_id: TypeId,
    template_node_type_id: TypeId,
    node_data: NodeType,
    location: Option<NodeLocation>,
    
    // Used for Undo/Redo
    _cached_node_data: Option<NodeData>
}

impl AddTemplateNodeRequest {
    pub fn new(
        containing_component_type_id: TypeId,
        template_node_type_id: TypeId,
        node_data: NodeType,
        location: Option<NodeLocation>,
    ) -> Self {
        Self {
            containing_component_type_id,
            template_node_type_id,
            node_data,
            location,
            _cached_node_data: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AddTemplateNodeResponse {
    command_id: Option<usize>,
    uni: UniqueTemplateNodeIdentifier,
}

impl Request for AddTemplateNodeRequest {
    type Response = AddTemplateNodeResponse;
}

impl Response for AddTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }
}

impl Command<AddTemplateNodeRequest> for AddTemplateNodeRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<AddTemplateNodeResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.containing_component_type_id)
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }

        let mut template_node = TemplateNodeDefinition::default();
        template_node.type_id = self.template_node_type_id.clone();

        match &self.node_data {
            NodeType::Template(settings) => {
                template_node.settings = Some(settings.clone());
            },
            NodeType::ControlFlow(control_flow_settings) => {
                template_node.control_flow_settings = Some(*control_flow_settings.clone());
            },
            NodeType::Comment(raw_comment_string) => {
                template_node.raw_comment_string = Some(raw_comment_string.clone());
            },
        }

        let mut node_data = NodeData::default();
        node_data.cached_node = template_node.clone();

        if let Some(template) = &mut component.template {
            node_data.unique_node_identifier = if let Some(location) = &self.location {
                template.add_at(template_node, location.clone())
            } else {
                template.add(template_node)
            };
        } else {
            let mut template = ComponentTemplate::new(self.containing_component_type_id.clone(), None);
            node_data.unique_node_identifier = if let Some(location) = &self.location {
                template.add_at(template_node, location.clone())
            } else {
                template.add(template_node)
            };
            component.template = Some(template);
        }

        self._cached_node_data = Some(node_data.clone());

        Ok(AddTemplateNodeResponse {
            command_id: None,
            uni: node_data.unique_node_identifier.clone()
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::AddTemplateNodeRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl Undo for AddTemplateNodeRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.containing_component_type_id)
            .unwrap();

        let cached_data = self._cached_node_data.clone().unwrap();
        if let Some(template) = &mut component.template {
            template.remove_node(cached_data.unique_node_identifier.get_template_node_id());
            template.set_next_id(cached_data.unique_node_identifier.get_template_node_id().as_usize());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateTemplateNodeRequest {
    uni: UniqueTemplateNodeIdentifier,
    updated_node: TemplateNodeDefinition,
    new_location: Option<NodeLocation>,
    // Used for Undo/Redo
    _cached_node_data: Option<NodeData>,
    _cached_move: Option<MoveTemplateNodeRequest>,
}

impl UpdateTemplateNodeRequest {
    pub fn new(uni: UniqueTemplateNodeIdentifier, updated_node: TemplateNodeDefinition, new_location: Option<NodeLocation>) -> Self {
        Self {
            uni,
            updated_node,
            new_location,
            _cached_node_data: None,
            _cached_move: None,
        }
    }
}

pub struct UpdateTemplateNodeResponse {
    command_id: Option<usize>,
}

impl Request for UpdateTemplateNodeRequest {
    type Response = UpdateTemplateNodeResponse;
}

impl Response for UpdateTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }
}

impl Command<UpdateTemplateNodeRequest> for UpdateTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<UpdateTemplateNodeResponse, String> {
        let uni = self.uni.clone();
        let component = manifest
            .components
            .get_mut(&uni.get_containing_component_type_id())
            .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }
       
       if let Some(template) = &mut component.template {
            let mut node_data = NodeData::default();
            node_data.unique_node_identifier = uni.clone();
            node_data.cached_node = template.get_node(&uni.get_template_node_id()).expect("Cannot update node that doesn't exist").clone();

            template.set_node(uni.get_template_node_id(), self.updated_node.clone());

            if let Some(location) = &self.new_location {
                let mut move_request = MoveTemplateNodeRequest::new(uni, location.clone());
                move_request.execute(manifest).unwrap();
                self._cached_move = Some(move_request.clone());
            }
       }

        Ok(UpdateTemplateNodeResponse {
            command_id: None,
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::UpdateTemplateNodeRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl Undo for UpdateTemplateNodeRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        if let Some(move_request) = &mut self._cached_move {
            move_request.undo(manifest).unwrap();
        }       
        
        
        let component = manifest
        .components
        .get_mut(&self.uni.get_containing_component_type_id())
        .unwrap();

        if let Some(template) = &mut component.template {
            if let Some(data) = &self._cached_node_data {
                template.set_node(self.uni.get_template_node_id(), data.cached_node.clone());
            }
        }
        Ok(())
    }
}


#[derive(Serialize, Deserialize, Clone)]
pub struct MoveTemplateNodeRequest {
    uni: UniqueTemplateNodeIdentifier,
    new_location: NodeLocation,
    // Used for Undo/Redo
    _cached_old_position: Option<NodeLocation>,
}

impl MoveTemplateNodeRequest {
    pub fn new(uni: UniqueTemplateNodeIdentifier, new_location: NodeLocation) -> Self {
        Self {
            uni,
            new_location,
            _cached_old_position: None,
        }
    }
}

pub struct MoveTemplateNodeResponse {
    command_id: Option<usize>,
}

impl Request for MoveTemplateNodeRequest {
    type Response = MoveTemplateNodeResponse;
}

impl Response for MoveTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }
}

impl Command<MoveTemplateNodeRequest> for MoveTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<MoveTemplateNodeResponse, String> {
        let uni = self.uni.clone();
        let requested_component = self.new_location.get_type_id();

        let current_component = manifest
            .components
            .get_mut(&uni.get_containing_component_type_id())
            .unwrap();

        if current_component.is_primitive || current_component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }
        if current_component.template.is_none() {
            unreachable!("Component doesn't have a template.");
        }

        let template = current_component.template.as_mut().unwrap();

        if *requested_component != uni.get_containing_component_type_id() {
            panic!("Cannot move node to a different component.");
        }

        self._cached_old_position = template.get_location(&self.uni.get_template_node_id());
        template.move_node(&uni.get_template_node_id(), self.new_location.clone());

        Ok(MoveTemplateNodeResponse {
            command_id: None,
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::MoveTemplateNodeRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl Undo for MoveTemplateNodeRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
        .components
        .get_mut(&self.uni.get_containing_component_type_id())
        .unwrap();

        if let Some(template) = &mut component.template {
            if let Some(location) = &self._cached_old_position {
                template.move_node(&self.uni.get_template_node_id(), location.clone());
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemoveTemplateNodeRequest {
    uni: UniqueTemplateNodeIdentifier,
    // Used for Undo/Redo
    _cached_template: Option<ComponentTemplate>
    
}

impl RemoveTemplateNodeRequest {
    pub fn new(uni: UniqueTemplateNodeIdentifier) -> Self {
        RemoveTemplateNodeRequest {
            uni,
            _cached_template: None
        }
    }
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
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }
}

impl Command<RemoveTemplateNodeRequest> for RemoveTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<RemoveTemplateNodeResponse, String> {
        let component = manifest
        .components
        .get_mut(&self.uni.get_containing_component_type_id())
        .unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }

        if let Some(template) = &mut component.template {
            self._cached_template = Some(template.clone());
            template.remove_node(self.uni.get_template_node_id());
        };

        Ok(RemoveTemplateNodeResponse { command_id: None })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::RemoveTemplateNodeRequest(self.clone()))
    }

    fn is_mutative(&self) -> bool {
        true
    }
}

impl Undo for RemoveTemplateNodeRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
        .components
        .get_mut(&self.uni.get_containing_component_type_id())
        .unwrap();
       component.template = self._cached_template.clone();
        Ok(())
    }
}

pub struct GetTemplateNodeRequest {
    uni: UniqueTemplateNodeIdentifier,
}

pub struct GetTemplateNodeResponse {
    command_id: Option<usize>,
    #[allow(unused)]
    node: Option<TemplateNodeDefinition>,
}

impl Request for GetTemplateNodeRequest {
    type Response = GetTemplateNodeResponse;
}

impl Response for GetTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }
}

impl Command<GetTemplateNodeRequest> for GetTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<<GetTemplateNodeRequest as Request>::Response, String> {
        let component = manifest
        .components
        .get_mut(&self.uni.get_containing_component_type_id())
        .unwrap();

        let mut node = None;
        if let Some(template) = &component.template {
            node = Some(template.get_node(&self.uni.get_template_node_id()).unwrap().clone());
        }

        Ok(GetTemplateNodeResponse {
            command_id: None,
            node,
        })
    }

    fn is_mutative(&self) -> bool {
        false
    }
}

pub struct GetAllTemplateNodeRequest {
    component_type_id: TypeId,
}

pub struct GetAllTemplateNodeResponse {
    command_id: Option<usize>,
    #[allow(unused)]
    nodes: Option<Vec<TemplateNodeDefinition>>,
}

impl Request for GetAllTemplateNodeRequest {
    type Response = GetAllTemplateNodeResponse;
}

impl Response for GetAllTemplateNodeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
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

        let mut nodes = None;
        if let Some(template) = &component.template {
            nodes = Some(template.get_nodes_owned());
        }

        Ok(GetAllTemplateNodeResponse {
            command_id: None,
            nodes,
        })
    }

    fn is_mutative(&self) -> bool {
        false
    }
}
