use std::collections::HashMap;

use pax_manifest::{pax_runtime_api::ToPaxValue, TypeId, ValueDefinition};

use crate::orm::PaxManifestORM;
use anyhow::{anyhow, Result};

use super::UpdateClassRequest;

/// Builder for creating and modifying template nodes in the PaxManifest.
pub struct ClassBuilder<'a> {
    orm: &'a mut PaxManifestORM,
    containing_component_type_id: TypeId,
    class_name: String,
    updated_class_properties: HashMap<String, Option<ValueDefinition>>,
}

pub struct ClassSaveData {
    pub undo_id: Option<usize>,
}

impl<'a> ClassBuilder<'a> {
    pub fn new(
        orm: &'a mut PaxManifestORM,
        component_type_id: TypeId,
        name: &'a str,
    ) -> Result<Self> {
        if !pax_manifest::utils::valid_class_or_id(&name) {
            return Err(anyhow!("not a valid class identifier"));
        }
        Ok(Self {
            orm,
            containing_component_type_id: component_type_id,
            class_name: name.to_string(),
            updated_class_properties: HashMap::new(),
        })
    }

    pub fn set_property_from_value_definition(
        &mut self,
        key: &str,
        value: Option<ValueDefinition>,
    ) -> Result<()> {
        self.updated_class_properties.insert(key.to_string(), value);
        Ok(())
    }

    pub fn set_property_from_typed<T: ToPaxValue>(
        &mut self,
        key: &str,
        value: Option<T>,
    ) -> Result<()> {
        self.set_property_from_value_definition(
            key,
            value.map(|v| ValueDefinition::LiteralValue(v.to_pax_value())),
        )
    }

    pub fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        self.set_property_from_value_definition(
            key,
            pax_manifest::utils::parse_value(value).map_err(|e| anyhow!(e.to_owned()))?,
        )
    }

    pub fn remove_property(&mut self, key: &str) {
        self.updated_class_properties.insert(key.to_string(), None);
    }

    pub fn save(self) -> Result<ClassSaveData, String> {
        let resp = self.orm.execute_command(UpdateClassRequest::new(
            self.containing_component_type_id,
            self.class_name,
            self.updated_class_properties,
        ))?;

        Ok(ClassSaveData {
            undo_id: resp.command_id,
        })
    }
}
