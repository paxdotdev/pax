use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    constants::{COMMON_PROPERTIES, COMMON_PROPERTIES_TYPE},
    HandlersBlockElement, PaxManifest, PropertyDefinition, SettingElement, SettingsBlockElement,
    TemplateNodeDefinition, Token, ValueDefinition,
};

#[derive(Serialize, Debug)]
pub struct ComponentInfo {
    pub type_id: String,
    pub pascal_identifier: String,
    pub primitive_instance_import_path: Option<String>,
    pub properties: Vec<PropertyInfo>,
    pub handlers: Vec<HandlerInfo>,
}

#[derive(Serialize, Debug)]
pub struct PropertyInfo {
    pub name: String,
    pub property_type: PropertyDefinition,
}

#[derive(Serialize, Debug)]
pub struct HandlerInfo {
    pub name: String,
    pub args_type: Option<String>,
}

impl PaxManifest {
    pub fn get_component_handlers(&self, type_id: &str) -> Vec<(String, Vec<String>)> {
        let mut handlers = Vec::new();
        if let Some(component) = self.components.get(type_id) {
            if let Some(component_handlers) = &component.handlers {
                for handler in component_handlers {
                    if let HandlersBlockElement::Handler(key, values) = handler {
                        handlers.push((
                            key.token_value.clone(),
                            values
                                .iter()
                                .map(|v| self.clean_handler(v.raw_value.clone()))
                                .collect(),
                        ));
                    }
                }
            }
        }
        handlers
    }

    pub fn event_to_args_map(&self) -> HashMap<String, Option<String>> {
        let mut map = HashMap::new();
        map.insert("scroll".to_string(), Some("ArgsScroll".to_string()));
        map.insert("clap".to_string(), Some("ArgsClap".to_string()));
        map.insert(
            "touch_start".to_string(),
            Some("ArgsTouchStart".to_string()),
        );
        map.insert("touch_move".to_string(), Some("ArgsTouchMove".to_string()));
        map.insert("touch_end".to_string(), Some("ArgsTouchEnd".to_string()));
        map.insert("key_down".to_string(), Some("ArgsKeyDown".to_string()));
        map.insert("key_up".to_string(), Some("ArgsKeyUp".to_string()));
        map.insert("key_press".to_string(), Some("ArgsKeyPress".to_string()));
        map.insert(
            "checkbox_change".to_string(),
            Some("ArgsCheckboxChange".to_string()),
        );
        map.insert(
            "button_click".to_string(),
            Some("ArgsButtonClick".to_string()),
        );
        map.insert(
            "textbox_change".to_string(),
            Some("ArgsTextboxChange".to_string()),
        );
        map.insert("click".to_string(), Some("ArgsClick".to_string()));
        map.insert("mouse_down".to_string(), Some("ArgsMouseDown".to_string()));
        map.insert("mouse_up".to_string(), Some("ArgsMouseUp".to_string()));
        map.insert("mouse_move".to_string(), Some("ArgsMouseMove".to_string()));
        map.insert("mouse_over".to_string(), Some("ArgsMouseOver".to_string()));
        map.insert("mouse_out".to_string(), Some("ArgsMouseOut".to_string()));
        map.insert(
            "double_click".to_string(),
            Some("ArgsDoubleClick".to_string()),
        );
        map.insert(
            "context_menu".to_string(),
            Some("ArgsContextMenu".to_string()),
        );
        map.insert("wheel".to_string(), Some("ArgsWheel".to_string()));
        map.insert("pre_render".to_string(), None);
        map.insert("mount".to_string(), None);
        map
    }

    fn clean_handler(&self, handler: String) -> String {
        handler.replace("self.", "")
    }

    pub fn generate_codegen_component_info(&self) -> Vec<ComponentInfo> {
        let mut component_infos = Vec::new();
        let event_map = self.event_to_args_map();

        // get all the properties for this commonent type
        for (type_id, component) in &self.components {
            // No need to generate helpers on simple types
            if component.is_struct_only_component {
                continue;
            }

            let mut properties = Vec::new();
            for property in &self.type_table[type_id].property_definitions {
                properties.push(PropertyInfo {
                    name: property.name.clone(),
                    property_type: property.clone(),
                });
            }

            let mut handler_data = Vec::new();

            // pull all handlers from the component settings
            if let Some(handlers) = &component.handlers {
                for handler in handlers {
                    if let HandlersBlockElement::Handler(key, values) = handler {
                        for value in values {
                            let args_type = event_map.get(key.token_value.as_str()).unwrap();
                            handler_data.push(HandlerInfo {
                                name: self.clean_handler(value.raw_value.clone()),
                                args_type: args_type.clone(),
                            });
                        }
                    }
                }
            }

            // pull all handlers from the template inline settings
            if let Some(template) = &component.template {
                for (id, tnd) in template {
                    if id > &0 {
                        if let Some(settings) = &tnd.settings {
                            for setting in settings {
                                if let SettingElement::Setting(key, value) = setting {
                                    if let ValueDefinition::EventBindingTarget(e) = value {
                                        let args_type = event_map
                                            .get(key.token_value.as_str())
                                            .expect("Unsupported event");
                                        handler_data.push(HandlerInfo {
                                            name: self.clean_handler(e.raw_value.clone()),
                                            args_type: args_type.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            component_infos.push(ComponentInfo {
                type_id: component.type_id.clone(),
                pascal_identifier: component.pascal_identifier.clone(),
                primitive_instance_import_path: component.primitive_instance_import_path.clone(),
                properties,
                handlers: handler_data,
            });
        }
        component_infos
    }

    pub fn get_inline_properties(
        &self,
        containing_component_type_id: &str,
        tnd: &TemplateNodeDefinition,
    ) -> HashMap<String, ValueDefinition> {
        let component = self.components.get(containing_component_type_id).unwrap();
        let settings =
            Self::merge_inline_settings_with_settings_block(&tnd.settings, &component.settings);
        let mut map = HashMap::new();
        if let Some(settings) = &settings {
            for setting in settings {
                if let SettingElement::Setting(key, value) = setting {
                    match value {
                        ValueDefinition::LiteralValue(_)
                        | ValueDefinition::Block(_)
                        | ValueDefinition::Expression(_, _)
                        | ValueDefinition::Identifier(_, _) => {
                            map.insert(key.token_value.clone(), value.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
        map
    }

    pub fn get_inline_common_properties(
        &self,
        containing_component_type_id: &str,
        tnd: &TemplateNodeDefinition,
    ) -> HashMap<String, ValueDefinition> {
        let component = self.components.get(containing_component_type_id).unwrap();
        let settings =
            Self::merge_inline_settings_with_settings_block(&tnd.settings, &component.settings);
        let mut map = HashMap::new();
        if let Some(settings) = &settings {
            for setting in settings {
                if let SettingElement::Setting(key, value) = setting {
                    match value {
                        ValueDefinition::LiteralValue(_)
                        | ValueDefinition::Block(_)
                        | ValueDefinition::Expression(_, _)
                        | ValueDefinition::Identifier(_, _) => {
                            if CommonProperty::get_common_properties().contains(&key.token_value) {
                                map.insert(key.token_value.clone(), value.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        map
    }

    pub fn get_inline_event_handlers(&self, tnd: &TemplateNodeDefinition) -> Vec<(String, String)> {
        let mut handlers = Vec::new();
        if let Some(settings) = &tnd.settings {
            for setting in settings {
                if let SettingElement::Setting(key, value) = setting {
                    match value {
                        ValueDefinition::EventBindingTarget(e) => {
                            handlers.push((
                                key.token_value.clone(),
                                self.clean_handler(e.raw_value.clone()),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
        handlers
    }

    fn pull_matched_identifiers_from_inline(
        inline_settings: &Option<Vec<SettingElement>>,
        s: String,
    ) -> Vec<Token> {
        let mut ret = Vec::new();
        if let Some(val) = inline_settings {
            let matched_settings = val.iter().filter(|e| match e {
                SettingElement::Setting(token, _) => token.token_value == s.as_str(),
                _ => false,
            });
            for e in matched_settings {
                if let SettingElement::Setting(_, value) = e {
                    match value {
                        ValueDefinition::Identifier(s, _) => ret.push(s.clone()),
                        _ => {}
                    };
                }
            }
        }
        ret
    }

    fn pull_settings_with_selector(
        settings: &Option<Vec<SettingsBlockElement>>,
        selector: String,
    ) -> Option<Vec<SettingElement>> {
        settings.as_ref().and_then(|val| {
            let mut merged_setting = Vec::new();
            for settings_value in val.iter() {
                match settings_value {
                    SettingsBlockElement::SelectorBlock(token, value) => {
                        if token.token_value == selector {
                            merged_setting.extend(value.elements.clone());
                        }
                    }
                    _ => {}
                }
            }
            (!merged_setting.is_empty()).then(|| merged_setting)
        })
    }

    pub fn merge_inline_settings_with_settings_block(
        inline_settings: &Option<Vec<SettingElement>>,
        settings_block: &Option<Vec<SettingsBlockElement>>,
    ) -> Option<Vec<SettingElement>> {
        // collect id settings
        let ids = Self::pull_matched_identifiers_from_inline(&inline_settings, "id".to_string());

        let mut id_settings = Vec::new();
        if ids.len() == 1 {
            if let Some(settings) = Self::pull_settings_with_selector(
                &settings_block,
                format!("#{}", ids[0].token_value),
            ) {
                id_settings.extend(settings.clone());
            }
        } else if ids.len() > 1 {
            panic!("Specified more than one id inline!");
        }

        // collect all class settings
        let classes =
            Self::pull_matched_identifiers_from_inline(&inline_settings, "class".to_string());

        let mut class_settings = Vec::new();
        for class in classes {
            if let Some(settings) = Self::pull_settings_with_selector(
                &settings_block,
                format!(".{}", class.token_value),
            ) {
                class_settings.extend(settings.clone());
            }
        }

        let mut map = HashMap::new();

        // Iterate in reverse order of priority (class, then id, then inline)
        for e in class_settings.into_iter() {
            if let SettingElement::Setting(key, _) = e.clone() {
                map.insert(key, e);
            }
        }

        for e in id_settings.into_iter() {
            if let SettingElement::Setting(key, _) = e.clone() {
                map.insert(key, e);
            }
        }

        if let Some(inline) = inline_settings.clone() {
            for e in inline.into_iter() {
                if let SettingElement::Setting(key, _) = e.clone() {
                    map.insert(key, e);
                }
            }
        }
        let merged: Vec<SettingElement> = map.values().cloned().collect();

        if merged.len() > 0 {
            Some(merged)
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CommonProperty {
    name: String,
    property_type: String,
    is_optional: bool,
}

impl CommonProperty {
    pub fn get_common_properties() -> Vec<String> {
        COMMON_PROPERTIES.iter().map(|e| e.to_string()).collect()
    }

    pub fn get_property_types() -> Vec<(String, String)> {
        COMMON_PROPERTIES_TYPE
            .iter()
            .map(|(c, t)| (c.to_string(), t.to_string()))
            .collect()
    }

    pub fn get_as_common_property() -> Vec<CommonProperty> {
        let mut common_properties = Vec::new();
        for (name, property_type) in CommonProperty::get_property_types() {
            common_properties.push(CommonProperty {
                name: name.clone(),
                property_type,
                is_optional: (name != "transform" && name != "width" && name != "height"),
            });
        }
        common_properties
    }
}
