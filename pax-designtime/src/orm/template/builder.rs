use std::collections::HashMap;

use anyhow::{anyhow, Result};
use pax_manifest::{
    NodeLocation, PropertyDefinition, SettingElement, TemplateNodeDefinition, Token, TokenType,
    TypeId, UniqueTemplateNodeIdentifier, ValueDefinition,
};
use serde::Serialize;

use super::{AddTemplateNodeRequest, GetTemplateNodeRequest, NodeType, UpdateTemplateNodeRequest};
use crate::{orm::PaxManifestORM, serde_pax};

/// Builder for creating and modifying template nodes in the PaxManifest.
pub struct NodeBuilder<'a> {
    orm: &'a mut PaxManifestORM,
    containing_component_type_id: TypeId,
    node_type_id: TypeId,
    property_map: HashMap<String, usize>,
    settings: Option<Vec<SettingElement>>,
    unique_node_identifier: Option<UniqueTemplateNodeIdentifier>,
    location: Option<NodeLocation>,
}

impl<'a> NodeBuilder<'a> {
    pub fn new(
        orm: &'a mut PaxManifestORM,
        containing_component_type_id: TypeId,
        node_type_id: TypeId,
    ) -> Self {
        NodeBuilder {
            orm,
            containing_component_type_id,
            node_type_id,
            property_map: HashMap::new(),
            settings: None,
            unique_node_identifier: None,
            location: None,
        }
    }

    pub fn retrieve_node(
        orm: &'a mut PaxManifestORM,
        uni: UniqueTemplateNodeIdentifier,
    ) -> Option<Self> {
        let resp = orm
            .execute_command(GetTemplateNodeRequest { uni: uni.clone() })
            .unwrap();
        if let Some(node) = resp.node {
            let mut property_map = HashMap::new();
            if let NodeType::Template(settings) = node.get_node_type() {
                for (index, setting) in settings.iter().enumerate() {
                    if let SettingElement::Setting(Token { token_value, .. }, _) = setting {
                        property_map.insert(token_value.clone(), index);
                    }
                }
            }
            let location = orm.manifest.get_node_location(&uni);
            Some(NodeBuilder {
                orm,
                containing_component_type_id: uni.get_containing_component_type_id(),
                node_type_id: node.type_id,
                property_map,
                settings: node.settings.clone(),
                unique_node_identifier: Some(uni),
                location,
            })
        } else {
            None
        }
    }

    pub fn get_unique_identifier(&self) -> Option<UniqueTemplateNodeIdentifier> {
        self.unique_node_identifier.clone()
    }

    pub fn get_all_properties(&self) -> Vec<(PropertyDefinition, Option<ValueDefinition>)> {
        let properties = self
            .orm
            .manifest
            .get_all_component_properties(&self.node_type_id);
        let values = properties
            .iter()
            .map(|prop| {
                if let Some(index) = self.property_map.get(&prop.name) {
                    if let Some(SettingElement::Setting(_, value)) =
                        self.settings.as_ref().unwrap().get(*index)
                    {
                        Some(value.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<Option<ValueDefinition>>>();

        properties.into_iter().zip(values).collect()
    }

    pub fn set_typed_property<T: Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        let value = serde_pax::se::to_pax::<T>(&value)?;
        self.set_property(key, &value)
    }

    pub fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        if value.is_empty() {
            self.remove_property(key);
            return Ok(());
        }
        let value = pax_manifest::utils::parse_value(value).map_err(|e| anyhow!(e.to_owned()))?;
        let token = Token::new_from_raw_value(key.to_owned(), TokenType::SettingKey);
        if let Some(index) = self.property_map.get(key) {
            self.settings.as_mut().unwrap()[*index] = SettingElement::Setting(token, value);
        } else {
            if let Some(settings) = &mut self.settings {
                settings.push(SettingElement::Setting(token, value));
            } else {
                self.settings = Some(vec![SettingElement::Setting(token, value)]);
            }
            self.property_map
                .insert(key.to_string(), self.settings.as_ref().unwrap().len() - 1);
        };
        Ok(())
    }

    pub fn remove_property(&mut self, key: &str) {
        if let Some(index) = self.property_map.get(key) {
            self.settings.as_mut().unwrap().remove(*index);

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

    pub fn set_location(&mut self, location: NodeLocation) {
        self.location = Some(location);
    }

    pub fn save(mut self) -> Result<usize, String> {
        let id = if let Some(uni) = self.unique_node_identifier {
            // Node already exists
            let location = self
                .location
                .unwrap_or_else(|| self.orm.manifest.get_node_location(&uni).unwrap());

            let updated_node = TemplateNodeDefinition {
                type_id: self.node_type_id,
                control_flow_settings: None,
                settings: self.settings,
                raw_comment_string: None,
            };

            let resp = self.orm.execute_command(UpdateTemplateNodeRequest::new(
                uni,
                updated_node,
                Some(location),
            ))?;
            resp.command_id
        } else {
            // Node does not exist
            let resp = self.orm.execute_command(AddTemplateNodeRequest::new(
                self.containing_component_type_id,
                self.node_type_id,
                NodeType::Template(self.settings.unwrap_or_default()),
                self.location,
            ))?;
            self.location = self.orm.manifest.get_node_location(&resp.uni);
            self.unique_node_identifier = Some(resp.uni);
            resp.command_id
        };

        Ok(id.unwrap())
    }
}
