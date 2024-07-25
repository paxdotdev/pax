use std::collections::HashMap;

use anyhow::{anyhow, Result};
use pax_manifest::{
    NodeLocation, PropertyDefinition, SettingElement, Token, TokenType, TypeId,
    UniqueTemplateNodeIdentifier, ValueDefinition,
};
use serde::Serialize;

use super::{AddTemplateNodeRequest, GetTemplateNodeRequest, NodeType, UpdateTemplateNodeRequest};
use crate::{orm::PaxManifestORM, serde_pax};

/// Builder for creating and modifying template nodes in the PaxManifest.
pub struct NodeBuilder<'a> {
    orm: &'a mut PaxManifestORM,
    containing_component_type_id: TypeId,
    node_type_id: TypeId,
    updated_property_map: HashMap<Token, Option<ValueDefinition>>,
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
            updated_property_map: HashMap::new(),
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
            let location = orm.manifest.get_node_location(&uni);
            Some(NodeBuilder {
                orm,
                containing_component_type_id: uni.get_containing_component_type_id(),
                node_type_id: node.type_id,
                updated_property_map: HashMap::new(),
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

    pub fn get_all_properties(&mut self) -> Vec<(PropertyDefinition, Option<ValueDefinition>)> {
        let properties = self
            .orm
            .manifest
            .get_all_component_properties(&self.node_type_id);

        let mut full_settings: HashMap<Token, ValueDefinition> = HashMap::new();
        if let Some(uni) = &self.unique_node_identifier {
            let resp = self
                .orm
                .execute_command(GetTemplateNodeRequest { uni: uni.clone() })
                .unwrap();
            if let Some(node) = resp.node {
                if let Some(settings) = node.settings {
                    for setting in settings {
                        if let SettingElement::Setting(token, value) = setting {
                            full_settings.insert(token, value);
                        }
                    }
                }
            }
        }

        let values: Vec<Option<ValueDefinition>> = properties
            .iter()
            .map(|prop| {
                let key = &Token::new_only_raw(prop.name.clone(), TokenType::SettingKey);
                full_settings.get(key).cloned()
            })
            .collect();

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
        self.updated_property_map.insert(token, Some(value));
        Ok(())
    }

    pub fn remove_property(&mut self, key: &str) {
        let key = Token::new_from_raw_value(key.to_owned(), TokenType::SettingKey);
        self.updated_property_map.insert(key, None);
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

            let resp = self.orm.execute_command(UpdateTemplateNodeRequest::new(
                uni,
                self.updated_property_map,
                Some(location),
            ))?;
            resp.command_id
        } else {
            // Node does not exist
            let settings = self
                .updated_property_map
                .iter()
                .filter_map(|(k, v)| {
                    v.as_ref()
                        .map(|value| SettingElement::Setting(k.clone(), value.clone()))
                })
                .collect::<Vec<SettingElement>>();

            let resp = self.orm.execute_command(AddTemplateNodeRequest::new(
                self.containing_component_type_id,
                self.node_type_id,
                NodeType::Template(settings),
                self.location,
            ))?;
            self.location = self.orm.manifest.get_node_location(&resp.uni);
            self.unique_node_identifier = Some(resp.uni);
            resp.command_id
        };

        Ok(id.unwrap())
    }
}
