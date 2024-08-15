use core::panic;
use std::{collections::HashMap, path::PathBuf};

use pax_manifest::{
    pax_runtime_api::ToPaxValue, ComponentDefinition, ComponentTemplate, NodeLocation, NodeType,
    PaxManifest, SettingElement, TemplateNodeDefinition, TemplateNodeId, Token, TreeIndexPosition,
    TreeLocation, TypeId, UniqueTemplateNodeIdentifier, ValueDefinition,
};
use serde_derive::{Deserialize, Serialize};

use super::{
    Command, MoveToComponentEntry, ReloadType, Request, Response, SubTrees, Undo, UndoRedoCommand,
};

pub mod builder;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct NodeData {
    pub unique_node_identifier: UniqueTemplateNodeIdentifier,
    pub cached_node: TemplateNodeDefinition,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddTemplateNodeRequest {
    containing_component_type_id: TypeId,
    template_node_type_id: TypeId,
    node_data: NodeType,
    location: Option<NodeLocation>,

    // Used for Undo/Redo
    _cached_node_data: Option<NodeData>,
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
    fn get_affected_component_type_id(&self) -> Option<TypeId> {
        Some(self.uni.get_containing_component_type_id().clone())
    }
    fn get_reload_type(&self) -> Option<ReloadType> {
        Some(ReloadType::FullEdit)
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

        let mut template_node = TemplateNodeDefinition {
            type_id: self.template_node_type_id.clone(),
            ..Default::default()
        };

        match &self.node_data {
            NodeType::Template(settings) => {
                template_node.settings = Some(settings.clone());
            }
            NodeType::ControlFlow(control_flow_settings) => {
                template_node.control_flow_settings = Some(*control_flow_settings.clone());
            }
            NodeType::Comment(raw_comment_string) => {
                template_node.raw_comment_string = Some(raw_comment_string.clone());
            }
        }

        let mut node_data = NodeData {
            cached_node: template_node.clone(),
            ..Default::default()
        };

        if let Some(template) = &mut component.template {
            node_data.unique_node_identifier = if let Some(location) = &self.location {
                template.add_at(template_node, location.clone())
            } else {
                template.add(template_node)
            };
        } else {
            let mut template =
                ComponentTemplate::new(self.containing_component_type_id.clone(), None);
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
            uni: node_data.unique_node_identifier.clone(),
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::AddTemplateNodeRequest(Box::new(
            self.clone(),
        )))
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
            template.set_next_id(
                cached_data
                    .unique_node_identifier
                    .get_template_node_id()
                    .as_usize(),
            );
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateTemplateNodeRequest {
    uni: UniqueTemplateNodeIdentifier,
    updated_properties: HashMap<Token, Option<ValueDefinition>>,
    new_type_id: Option<TypeId>,
    new_location: Option<NodeLocation>,
    // Used for Undo/Redo
    _cached_node_data: Option<NodeData>,
    _cached_move: Option<MoveTemplateNodeRequest>,
}

impl UpdateTemplateNodeRequest {
    pub fn new(
        uni: UniqueTemplateNodeIdentifier,
        new_type_id: Option<TypeId>,
        updated_properties: HashMap<Token, Option<ValueDefinition>>,
        new_location: Option<NodeLocation>,
    ) -> Self {
        Self {
            uni,
            updated_properties,
            new_location,
            new_type_id,
            _cached_node_data: None,
            _cached_move: None,
        }
    }
}

pub struct UpdateTemplateNodeResponse {
    command_id: Option<usize>,
    _affected_component_type_id: TypeId,
    _affected_unique_node_identifier: UniqueTemplateNodeIdentifier,
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
    fn get_affected_component_type_id(&self) -> Option<TypeId> {
        Some(self._affected_component_type_id.clone())
    }
    fn get_reload_type(&self) -> Option<ReloadType> {
        Some(ReloadType::Partial(
            self._affected_unique_node_identifier.clone(),
        ))
    }
}

impl Command<UpdateTemplateNodeRequest> for UpdateTemplateNodeRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<UpdateTemplateNodeResponse, String> {
        let uni = self.uni.clone();
        let containing_component = uni.get_containing_component_type_id().clone();
        let component = manifest.components.get_mut(&containing_component).unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }

        if let Some(template) = &mut component.template {
            self._cached_node_data = Some(NodeData {
                unique_node_identifier: uni.clone(),
                cached_node: template
                    .get_node(&uni.get_template_node_id())
                    .expect("Cannot update node that doesn't exist")
                    .clone(),
            });

            template.update_node_properties(
                &uni.get_template_node_id(),
                &mut self.updated_properties.clone(),
            );
            if let Some(new_type) = &self.new_type_id {
                template.update_node_type_id(&uni.get_template_node_id(), new_type);
            }

            if let Some(location) = &self.new_location {
                let mut move_request = MoveTemplateNodeRequest::new(uni.clone(), location.clone());
                move_request.execute(manifest).unwrap();
                self._cached_move = Some(move_request.clone());
            }
        }

        Ok(UpdateTemplateNodeResponse {
            command_id: None,
            _affected_component_type_id: uni.get_containing_component_type_id(),
            _affected_unique_node_identifier: uni,
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::UpdateTemplateNodeRequest(Box::new(
            self.clone(),
        )))
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    _affected_component_type_id: TypeId,
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
    fn get_affected_component_type_id(&self) -> Option<TypeId> {
        Some(self._affected_component_type_id.clone())
    }
    fn get_reload_type(&self) -> Option<ReloadType> {
        Some(ReloadType::FullEdit)
    }
}

impl Command<MoveTemplateNodeRequest> for MoveTemplateNodeRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<MoveTemplateNodeResponse, String> {
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
            _affected_component_type_id: uni.get_containing_component_type_id(),
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::MoveTemplateNodeRequest(Box::new(
            self.clone(),
        )))
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PasteSubTreeRequest {
    new_location: NodeLocation,
    subtrees: SubTrees,
    _cached_template: Option<ComponentTemplate>,
}

impl PasteSubTreeRequest {
    pub fn new(new_location: NodeLocation, subtrees: SubTrees) -> Self {
        Self {
            new_location,
            subtrees,
            _cached_template: None,
        }
    }
}

pub struct PasteSubTreeResponse {
    command_id: Option<usize>,
    root_ids: Vec<TemplateNodeId>,
    _affected_component_type_id: TypeId,
}

impl PasteSubTreeResponse {
    pub fn get_created(&self) -> &[TemplateNodeId] {
        &self.root_ids
    }
}

impl Request for PasteSubTreeRequest {
    type Response = PasteSubTreeResponse;
}

impl Response for PasteSubTreeResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }

    fn get_affected_component_type_id(&self) -> Option<TypeId> {
        Some(self._affected_component_type_id.clone())
    }
    fn get_reload_type(&self) -> Option<ReloadType> {
        Some(ReloadType::FullEdit)
    }
}

impl Command<PasteSubTreeRequest> for PasteSubTreeRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<PasteSubTreeResponse, String> {
        let type_id = self.new_location.get_type_id();
        let component = manifest.components.get_mut(type_id).unwrap();

        if component.is_primitive || component.is_struct_only_component {
            unreachable!("Component doesn't accept template nodes.");
        }
        if component.template.is_none() {
            unreachable!("Component doesn't have a template.");
        }

        let template = component.template.as_mut().unwrap();
        self._cached_template = Some(template.clone());

        let mut root_ids = vec![];
        for r in self.subtrees.roots.iter().rev() {
            let def = self.subtrees.nodes.get(r).unwrap();
            let id = template
                .add_at(def.clone(), self.new_location.clone())
                .get_template_node_id();
            root_ids.push(id.clone());
            let mut to_visit = vec![];
            if let Some(children) = self.subtrees.children.get(r) {
                to_visit.push((id, children.clone()));
            }
            while let Some((id, children)) = to_visit.pop() {
                for c in children {
                    let c_def = self.subtrees.nodes.get(&c).unwrap();
                    let c_id = template
                        .add_child_back(id.clone(), c_def.clone())
                        .get_template_node_id();
                    if let Some(c_children) = self.subtrees.children.get(&c) {
                        to_visit.push((c_id, c_children.clone()));
                    }
                }
            }
        }

        Ok(PasteSubTreeResponse {
            command_id: None,
            root_ids,
            _affected_component_type_id: type_id.clone(),
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::PasteSubTreeRequest(Box::new(self.clone())))
    }
}

impl Undo for PasteSubTreeRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.new_location.type_id)
            .unwrap();
        component.template.clone_from(&self._cached_template);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveTemplateNodeRequest {
    uni: UniqueTemplateNodeIdentifier,
    // Used for Undo/Redo
    _cached_template: Option<ComponentTemplate>,
}

impl RemoveTemplateNodeRequest {
    pub fn new(uni: UniqueTemplateNodeIdentifier) -> Self {
        RemoveTemplateNodeRequest {
            uni,
            _cached_template: None,
        }
    }
}

pub struct RemoveTemplateNodeResponse {
    command_id: Option<usize>,
    _affected_component_type_id: TypeId,
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
    fn get_affected_component_type_id(&self) -> Option<TypeId> {
        Some(self._affected_component_type_id.clone())
    }
    fn get_reload_type(&self) -> Option<ReloadType> {
        Some(ReloadType::FullEdit)
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

        Ok(RemoveTemplateNodeResponse {
            command_id: None,
            _affected_component_type_id: self.uni.get_containing_component_type_id(),
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::RemoveTemplateNodeRequest(Box::new(
            self.clone(),
        )))
    }
}

impl Undo for RemoveTemplateNodeRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.uni.get_containing_component_type_id())
            .unwrap();
        component.template.clone_from(&self._cached_template);
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
            if let Some(n) = template.get_node(&self.uni.get_template_node_id()) {
                node = Some(n.clone());
            }
        }

        Ok(GetTemplateNodeResponse {
            command_id: None,
            node,
        })
    }
}

pub struct GetChildrenRequest {
    pub uni: UniqueTemplateNodeIdentifier,
}

pub struct GetChildrenResponse {
    command_id: Option<usize>,
    #[allow(unused)]
    pub children: Vec<UniqueTemplateNodeIdentifier>,
}

impl Request for GetChildrenRequest {
    type Response = GetChildrenResponse;
}

impl Response for GetChildrenResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }
}

impl Command<GetChildrenRequest> for GetChildrenRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<<GetChildrenRequest as Request>::Response, String> {
        let component = manifest
            .components
            .get_mut(&self.uni.get_containing_component_type_id())
            .unwrap();

        let mut children = vec![];
        if let Some(template) = &component.template {
            if let Some(n) = template.get_children(&self.uni.get_template_node_id()) {
                children = n
                    .iter()
                    .map(|tnid| {
                        UniqueTemplateNodeIdentifier::build(
                            self.uni.get_containing_component_type_id(),
                            tnid.clone(),
                        )
                    })
                    .collect();
            }
        }

        Ok(GetChildrenResponse {
            command_id: None,
            children,
        })
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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReplaceTemplateRequest {
    component_type_id: TypeId,
    new_template: ComponentTemplate,
    _cached_prev_template: Option<ComponentTemplate>,
}

impl ReplaceTemplateRequest {
    pub fn new(component_type_id: TypeId, new_template: ComponentTemplate) -> Self {
        Self {
            component_type_id,
            new_template,
            _cached_prev_template: None,
        }
    }
}

pub struct ReplaceTemplateResponse {
    command_id: Option<usize>,
    _affected_component_type_id: TypeId,
}

impl Response for ReplaceTemplateResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }

    fn get_affected_component_type_id(&self) -> Option<TypeId> {
        Some(self._affected_component_type_id.clone())
    }
    fn get_reload_type(&self) -> Option<ReloadType> {
        Some(ReloadType::FullEdit)
    }
}

impl Request for ReplaceTemplateRequest {
    type Response = ReplaceTemplateResponse;
}

impl Command<ReplaceTemplateRequest> for ReplaceTemplateRequest {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<ReplaceTemplateResponse, String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();

        self._cached_prev_template.clone_from(&component.template);

        component.template = Some(self.new_template.clone());

        Ok(ReplaceTemplateResponse {
            command_id: None,
            _affected_component_type_id: self.component_type_id.clone(),
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::ReplaceTemplateRequest(Box::new(
            self.clone(),
        )))
    }
}

impl Undo for ReplaceTemplateRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        let component = manifest
            .components
            .get_mut(&self.component_type_id)
            .unwrap();
        component.template.clone_from(&self._cached_prev_template);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConvertToComponentRequest {
    // These subtrees (roots) must be at the same TreeLocation
    subtrees_roots: Vec<MoveToComponentEntry>,
    new_component_number: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,

    // Used for Undo/Redo
    _cached_template: Option<ComponentTemplate>,
    _cached_add: Option<AddTemplateNodeRequest>,
    _cached_new_component_type_id: Option<TypeId>,
}

impl ConvertToComponentRequest {
    pub fn new(
        subtrees_roots: Vec<MoveToComponentEntry>,
        new_component_number: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Self {
        Self {
            subtrees_roots,
            new_component_number,
            x,
            y,
            width,
            height,
            _cached_template: None,
            _cached_add: None,
            _cached_new_component_type_id: None,
        }
    }
}

impl Request for ConvertToComponentRequest {
    type Response = ConvertToComponentResponse;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConvertToComponentResponse {
    command_id: Option<usize>,
    pub uni: UniqueTemplateNodeIdentifier,
    pub new_component_type_id: TypeId,
}

impl Response for ConvertToComponentResponse {
    fn set_id(&mut self, id: usize) {
        self.command_id = Some(id);
    }
    fn get_id(&self) -> usize {
        self.command_id.unwrap()
    }
    fn get_affected_component_type_id(&self) -> Option<TypeId> {
        Some(self.uni.get_containing_component_type_id().clone())
    }
    fn get_reload_type(&self) -> Option<ReloadType> {
        Some(ReloadType::FullEdit)
    }
}

impl Command<ConvertToComponentRequest> for ConvertToComponentRequest {
    fn execute(
        &mut self,
        manifest: &mut PaxManifest,
    ) -> Result<ConvertToComponentResponse, String> {
        if self.subtrees_roots.is_empty() {
            return Err("No subtrees provided".to_string());
        }

        let new_component_identifier = format!("NewComponent{}", self.new_component_number);
        let new_component_file_name = format!("new_component_{}.pax", self.new_component_number);
        let new_component_type_id = TypeId::build_blank_component(&new_component_identifier);

        let (module_path, ul_path) = {
            let userland_project_type_id =
                TypeId::build_singleton("designer_project::Example", None);
            let ul_bind = manifest.components.get(&userland_project_type_id);
            let ul = ul_bind.expect("Main component not found").clone();
            (
                ul.module_path.clone(),
                ul.template
                    .as_ref()
                    .expect("Main component template not found")
                    .get_file_path()
                    .expect("Main component file path not found")
                    .clone(),
            )
        };

        let new_component_path = PathBuf::from(ul_path)
            .parent()
            .expect("Main component path has no parent")
            .join(new_component_file_name)
            .to_str()
            .map(|s| s.to_string());

        let current_component_type_id = self.subtrees_roots[0]
            .id
            .get_containing_component_type_id()
            .clone();
        let binding = manifest.components.get_mut(&current_component_type_id);
        let current_component = binding.expect("Component not found");
        let current_component_template = current_component
            .template
            .as_mut()
            .expect("Component template not found");

        self._cached_template = Some(current_component_template.clone());

        let mut new_template =
            ComponentTemplate::new(new_component_type_id.clone(), new_component_path);

        fn add_subtree_to_new_template(
            current_template: &ComponentTemplate,
            new_template: &mut ComponentTemplate,
            node_id: TemplateNodeId,
            node_location: NodeLocation,
            root_bounds: Vec<MoveToComponentEntry>,
        ) {
            let mut node = current_template
                .get_node(&node_id)
                .expect("Node not found")
                .clone();

            let relevant_bounds = root_bounds
                .iter()
                .filter(|entry| entry.id.get_template_node_id() == node_id)
                .collect::<Vec<_>>();

            if let Some(e) = relevant_bounds.first() {
                if let Some(settings) = &mut node.settings {
                    update_position_if_exists(e.x, e.y, settings);
                }
            }

            let new_node_id = new_template.add_at(node, node_location.clone());
            for child in current_template.get_children(&node_id).unwrap_or_default() {
                let location = NodeLocation::new(
                    node_location.type_id.clone(),
                    TreeLocation::Parent(new_node_id.get_template_node_id().clone()),
                    TreeIndexPosition::Bottom,
                );
                add_subtree_to_new_template(
                    current_template,
                    new_template,
                    child,
                    location,
                    root_bounds.clone(),
                );
            }
        }

        // sort ids by TreeIndexPosition
        let mut ids_with_location = self
            .subtrees_roots
            .iter()
            .map(|entry| {
                let template_node_id = entry.id.get_template_node_id();
                let location = current_component_template
                    .get_location(&template_node_id)
                    .expect("Location not found")
                    .clone();
                (template_node_id, location)
            })
            .collect::<Vec<_>>();

        ids_with_location.sort_by(|a, b| a.1.cmp(&b.1));

        let new_component_location = ids_with_location.first().unwrap().1.clone();

        let mut processed_ids: Vec<TemplateNodeId> = vec![];
        let new_bounds = self
            .subtrees_roots
            .iter()
            .map(|e| {
                let mut new_entry = e.clone();
                new_entry.x -= self.x;
                new_entry.y -= self.y;
                new_entry
            })
            .collect::<Vec<_>>();

        for (id, nl) in ids_with_location {
            let new_location = NodeLocation::new(
                new_component_type_id.clone(),
                nl.tree_location.clone(),
                TreeIndexPosition::Bottom,
            );
            add_subtree_to_new_template(
                current_component_template,
                &mut new_template,
                id.clone(),
                new_location,
                new_bounds.clone(),
            );
            current_component_template.remove_node(id.clone());
            processed_ids.push(id);
        }

        let new_component = ComponentDefinition {
            type_id: new_component_type_id.clone(),
            is_main_component: false,
            is_primitive: false,
            is_struct_only_component: false,
            module_path,
            primitive_instance_import_path: None,
            template: Some(new_template),
            settings: None,
        };

        manifest
            .components
            .insert(new_component_type_id.clone(), new_component);

        self._cached_new_component_type_id = Some(new_component_type_id.clone());

        let settings = {
            let mut settings: Vec<SettingElement> = Vec::new();
            settings.push(SettingElement::Setting(
                Token::new_without_location("x".to_string()),
                ValueDefinition::LiteralValue(self.x.to_pax_value()),
            ));
            settings.push(SettingElement::Setting(
                Token::new_without_location("y".to_string()),
                ValueDefinition::LiteralValue(self.y.to_pax_value()),
            ));
            settings.push(SettingElement::Setting(
                Token::new_without_location("width".to_string()),
                ValueDefinition::LiteralValue(self.width.to_pax_value()),
            ));
            settings.push(SettingElement::Setting(
                Token::new_without_location("height".to_string()),
                ValueDefinition::LiteralValue(self.height.to_pax_value()),
            ));
            settings
        };

        let mut add_request = AddTemplateNodeRequest::new(
            current_component_type_id.clone(),
            new_component_type_id.clone(),
            NodeType::Template(settings),
            Some(new_component_location),
        );
        let response = add_request.execute(manifest)?;

        self._cached_add = Some(add_request);

        Ok(ConvertToComponentResponse {
            command_id: None,
            uni: response.uni,
            new_component_type_id,
        })
    }

    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        Some(UndoRedoCommand::ConvertToComponentRequest(Box::new(
            self.clone(),
        )))
    }
}

impl Undo for ConvertToComponentRequest {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        if let Some(add_request) = &mut self._cached_add {
            add_request.undo(manifest).unwrap();
        }

        if let Some(new_component_type_id) = &self._cached_new_component_type_id {
            manifest.components.remove(new_component_type_id);
        }

        if let Some(template) = &self._cached_template {
            let binding = manifest
                .components
                .get_mut(&self.subtrees_roots[0].id.get_containing_component_type_id());
            let current_component = binding.expect("Component not found");
            current_component.template = Some(template.clone());
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeAction {
    Add(AddTemplateNodeRequest),
    Update(UpdateTemplateNodeRequest),
    Remove(RemoveTemplateNodeRequest),
    Move(PasteSubTreeRequest),
}

pub fn update_position_if_exists(new_x: f64, new_y: f64, settings: &mut [SettingElement]) {
    for setting in settings.iter_mut() {
        if let SettingElement::Setting(key, value) = setting {
            if key.token_value == "x" {
                *value = ValueDefinition::LiteralValue(new_x.to_pax_value())
            }
            if key.token_value == "y" {
                *value = ValueDefinition::LiteralValue(new_y.to_pax_value())
            }
        }
    }
}
