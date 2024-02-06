use std::collections::HashMap;

use anyhow::Result;
use pax_manifest::{
    ControlFlowRepeatPredicateDefinition, ControlFlowRepeatSourceDefinition,
    ControlFlowSettingsDefinition, PropertyDefinition, SettingElement, TemplateNodeDefinition,
    Token, TokenType, ValueDefinition,
};

use super::{
    AddTemplateNodeRequest, GetAllTemplateNodeRequest, NodeType, UpdateTemplateNodeRequest,
};
use crate::orm::PaxManifestORM;

pub static TYPE_ID_IF: &str = "IF";
pub static TYPE_ID_REPEAT: &str = "REPEAT";
pub static TYPE_ID_SLOT: &str = "SLOT";
pub static TYPE_ID_COMMENT: &str = "COMMENT";
pub static PASCAL_IDENTIFIER_SLOT: &str = "Slot";
pub static PASCAL_IDENTIFIER_COMMENT: &str = "Comment";
pub static PASCAL_IDENTIFIER_IF: &str = "If";
pub static PASCAL_IDENTIFIER_REPEAT: &str = "Repeat";

type AllProperties = Option<(Vec<(Option<ValueDefinition>, String, String)>, String)>;
/// Builder for creating and modifying template nodes in the PaxManifest.
pub struct NodeBuilder<'a> {
    orm: &'a mut PaxManifestORM,
    component_type_id: String,
    template_node: TemplateNodeDefinition,
    property_map: HashMap<String, usize>,
    parent_node_id: Option<usize>,
    is_new: bool,
}

impl<'a> NodeBuilder<'a> {
    pub fn new(
        orm: &'a mut PaxManifestORM,
        component_type_id: String,
        type_id: String,
        pascal_identifier: String,
        parent_node_id: Option<usize>,
    ) -> Self {
        let template_node = TemplateNodeDefinition {
            id: 0,
            child_ids: Vec::new(),
            type_id: type_id.clone(),
            control_flow_settings: None,
            settings: None,
            pascal_identifier: pascal_identifier.clone(),
            raw_comment_string: None,
        };
        NodeBuilder {
            orm,
            component_type_id,
            template_node,
            property_map: HashMap::new(),
            parent_node_id,
            is_new: true,
        }
    }

    pub fn retrieve_node(
        orm: &'a mut PaxManifestORM,
        component_type_id: &str,
        node_id: usize,
    ) -> Self {
        let request = GetAllTemplateNodeRequest {
            component_type_id: component_type_id.to_owned(),
        };

        let command = request;
        let response = orm
            .execute_command::<GetAllTemplateNodeRequest, _>(command)
            .unwrap();

        if let Some(nodes) = response.nodes {
            let result = nodes.iter().find(|node| node.id == node_id);

            if let Some(node) = result {
                let parent_node_id = nodes
                    .iter()
                    .find(|node| node.child_ids.contains(&node_id))
                    .map(|node| node.id);

                let mut property_map = HashMap::new();

                if let Some(settings) = &node.settings {
                    for (index, setting) in settings.iter().enumerate() {
                        if let SettingElement::Setting(token, _) = setting {
                            property_map.insert(token.raw_value.clone(), index);
                        }
                    }
                }
                NodeBuilder {
                    orm,
                    component_type_id: component_type_id.to_owned(),
                    template_node: node.clone(),
                    parent_node_id,
                    property_map,
                    is_new: false,
                }
            } else {
                panic!("No template node found with id {}", node_id);
            }
        } else {
            panic!("No template nodes found");
        }
    }

    pub fn get_id(&self) -> usize {
        self.template_node.id
    }

    pub fn get_type_id(&self) -> &str {
        &self.template_node.type_id
    }

    pub fn get_property_definitions(&self) -> AllProperties {
        let template_props = self.template_node.settings.clone().unwrap_or_default();

        let template_node_type_id = self.get_type_id().to_owned();
        let mut available_props = self
            .orm
            .manifest
            .type_table
            .get(&template_node_type_id)?
            .property_definitions
            .to_owned();

        //Manually add common_props for now
        available_props.extend(
            [
                ("x", "Size"),
                ("y", "Size"),
                ("scale_x", "Size"),
                ("scale_y", "Size"),
                ("skew_x", "Numeric"),
                ("skew_y", "Numeric"),
                ("rotate", "Rotation"),
                ("anchor_x", "Size"),
                ("anchor_y", "Size"),
                ("transform", "Transform2D"),
                ("width", "Size"),
                ("height", "Size"),
            ]
            .into_iter()
            .map(|(name, type_id)| PropertyDefinition {
                name: name.to_owned(),
                type_id: type_id.to_owned(),
                flags: Default::default(),
                type_id_escaped: type_id.to_owned(),
            }),
        );
        let props: Vec<_> = available_props
            .into_iter()
            .map(|p| {
                (
                    template_props
                        .iter()
                        .find_map(|settings_elem| match settings_elem {
                            SettingElement::Setting(Token { token_value, .. }, value)
                                if token_value == &p.name =>
                            {
                                Some(value)
                            }
                            _ => None,
                        })
                        .cloned(),
                    p.name,
                    p.type_id,
                )
            })
            .collect();
        Some((props, template_node_type_id))
    }

    pub fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        if value.is_empty() {
            self.remove_property(key);
            return Ok(());
        }
        let value = pax_manifest::utils::parse_value(value)?;
        let token = Token::new_from_raw_value(key.to_owned(), TokenType::SettingKey);
        if let Some(index) = self.property_map.get(key) {
            self.template_node.settings.as_mut().unwrap()[*index] =
                SettingElement::Setting(token, value);
        } else {
            if let Some(settings) = &mut self.template_node.settings {
                settings.push(SettingElement::Setting(token, value));
            } else {
                self.template_node.settings = Some(vec![SettingElement::Setting(token, value)]);
            }
            self.property_map.insert(
                key.to_string(),
                self.template_node.settings.as_ref().unwrap().len() - 1,
            );
        };
        Ok(())
    }

    pub fn remove_property(&mut self, key: &str) {
        if let Some(index) = self.property_map.get(key) {
            self.template_node.settings.as_mut().unwrap().remove(*index);

            let keys_to_update: Vec<String> = self
                .property_map
                .iter()
                .filter(|(_, &index_elem)| index_elem > *index)
                .map(|(key, _)| key.clone())
                .collect();

            for key in keys_to_update {
                if let Some(index_elem) = self.property_map.get_mut(&key) {
                    *index_elem -= 1;
                }
            }

            self.property_map.remove(key);
        }
    }

    pub fn set_condition(&mut self, condition: String) {
        self.template_node.control_flow_settings = Some(ControlFlowSettingsDefinition {
            condition_expression_paxel: Some(Token::new_from_raw_value(
                condition,
                TokenType::IfExpression,
            )),
            repeat_predicate_definition: None,
            repeat_source_definition: None,
            slot_index_expression_paxel: None,
            condition_expression_vtable_id: None,
            slot_index_expression_vtable_id: None,
        });
        self.template_node.type_id = TYPE_ID_IF.to_string();
        self.template_node.pascal_identifier = PASCAL_IDENTIFIER_IF.to_string();
    }

    pub fn set_slot_index(&mut self, slot: String) {
        self.template_node.control_flow_settings = Some(ControlFlowSettingsDefinition {
            condition_expression_paxel: None,
            repeat_predicate_definition: None,
            repeat_source_definition: None,
            slot_index_expression_paxel: Some(Token::new_from_raw_value(
                slot,
                TokenType::SlotExpression,
            )),
            condition_expression_vtable_id: None,
            slot_index_expression_vtable_id: None,
        });
        self.template_node.type_id = TYPE_ID_SLOT.to_string();
        self.template_node.pascal_identifier = PASCAL_IDENTIFIER_SLOT.to_string();
    }

    pub fn set_repeat_expression(
        &mut self,
        pred: ControlFlowRepeatPredicateDefinition,
        source: ControlFlowRepeatSourceDefinition,
    ) {
        self.template_node.control_flow_settings = Some(ControlFlowSettingsDefinition {
            condition_expression_paxel: None,
            repeat_predicate_definition: Some(pred),
            repeat_source_definition: Some(source),
            slot_index_expression_paxel: None,
            condition_expression_vtable_id: None,
            slot_index_expression_vtable_id: None,
        });
        self.template_node.type_id = TYPE_ID_REPEAT.to_string();
        self.template_node.pascal_identifier = PASCAL_IDENTIFIER_REPEAT.to_string();
    }

    pub fn set_comment(&mut self, comment: String) {
        self.template_node.raw_comment_string = Some(comment);
        self.template_node.type_id = TYPE_ID_COMMENT.to_string();
        self.template_node.pascal_identifier = PASCAL_IDENTIFIER_COMMENT.to_string();
    }

    pub fn add_child(&mut self, child: NodeBuilder<'a>) {
        self.template_node.child_ids.push(child.template_node.id);
    }

    pub fn remove_child(&mut self, child_id: usize) {
        self.template_node.child_ids.retain(|id| *id != child_id);
    }

    pub fn insert_child_at(&mut self, index: usize, child: NodeBuilder<'a>) {
        self.template_node
            .child_ids
            .insert(index, child.template_node.id);
    }

    pub fn save(mut self) -> Result<(), String> {
        if self.is_new {
            let command = AddTemplateNodeRequest {
                component_type_id: self.component_type_id,
                parent_node_id: self.parent_node_id,
                node_id: None,
                child_ids: self.template_node.child_ids,
                type_id: self.template_node.type_id,
                node_type: NodeType::Template(self.template_node.settings.unwrap_or_default()),
                pascal_identifier: self.template_node.pascal_identifier,
                cached_node: None,
            };
            self.orm.execute_command(command)?;
            self.is_new = false;
        } else {
            let command = UpdateTemplateNodeRequest {
                component_type_id: self.component_type_id,
                new_parent: None,
                updated_node: self.template_node,
                cached_prev_state: None,
                cached_prev_parent: None,
                cached_prev_position: None,
            };

            self.orm.execute_command(command)?;
        };
        Ok(())
    }
}
