use std::{
    collections::HashMap, fmt::{format, Display, Formatter}
};

use pax_designtime::orm::template::{
    AddTemplateNodeRequest, NodeAction, RemoveTemplateNodeRequest, UpdateTemplateNodeRequest,
};
use pax_manifest::{
    ComponentTemplate, NodeLocation, NodeType, SettingElement, TemplateNodeDefinition,
    TemplateNodeId, Token, TreeIndexPosition, TreeLocation, TypeId, UniqueTemplateNodeIdentifier,
    ValueDefinition,
};

use super::simple::{
    SimpleColor, SimpleNodeAction, SimpleNodeInformation, SimpleNodeType, SimpleProperties,
    SimpleRotation, SimplePixel, SimplePercent, SimpleTemplate,
};

use super::constants::PREFIX;

pub fn convert_to_simple_node_info(
    id: &TemplateNodeId,
    node: &TemplateNodeDefinition,
) -> SimpleNodeInformation {
    SimpleNodeInformation {
        id: id.as_usize(),
        node_info: format!("{:?}", node.type_id.get_pascal_identifier()),
    }
}

impl From<ComponentTemplate> for SimpleTemplate {
    fn from(value: ComponentTemplate) -> Self {
        let mut root_nodes = vec![];
        for id in value.get_root() {
            if let Some(node) = value.get_node(&id) {
                root_nodes.push(convert_to_simple_node_info(&id, node));
            }
        }
        let mut children = std::collections::HashMap::new();

        let known_relationships = value.get_all_children_relationships();
        for (parent, node_children) in known_relationships.iter() {
            let mut current_children = vec![];
            for child in node_children {
                if let Some(node) = value.get_node(child) {
                    current_children.push(convert_to_simple_node_info(child, node));
                }
            }
            children.insert(parent.as_usize(), current_children);
        }
        SimpleTemplate {
            root_nodes,
            children,
        }
    }
}

pub fn simple_node_type_to_type_id(node_type: SimpleNodeType) -> Option<TypeId> {
    let t = match node_type {
        SimpleNodeType::Rectangle => {
            TypeId::build_singleton(&format!("{}::Rectangle", PREFIX), None)
        }
        SimpleNodeType::Ellipse => TypeId::build_singleton(&format!("{}::Ellipse", PREFIX), None),
        SimpleNodeType::Text => TypeId::build_singleton(&format!("{}::Text", PREFIX), None),
        SimpleNodeType::Navbar => TypeId::build_singleton(&format!("pax_designer::pax_reexports::designer_project::menu_bar::MenuBar"), None),
    };
    Some(t)
}

impl Display for SimplePercent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.value)
    }
}

impl Display for SimplePixel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
         write!(f, "{}px", self.value)
    }
}

impl Display for SimpleRotation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}deg", self.degrees)
    }
}

impl Display for SimpleColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

impl From<SimplePercent> for ValueDefinition {
    fn from(value: SimplePercent) -> Self {
        ValueDefinition::LiteralValue(Token::new_only_raw(
            value.to_string(),
            pax_manifest::TokenType::LiteralValue,
        ))
    }
}

impl From<SimplePixel> for ValueDefinition {
    fn from(value: SimplePixel) -> Self {
        ValueDefinition::LiteralValue(Token::new_only_raw(
            value.to_string(),
            pax_manifest::TokenType::LiteralValue,
        ))
    }
}

impl From<SimpleRotation> for ValueDefinition {
    fn from(value: SimpleRotation) -> Self {
        ValueDefinition::LiteralValue(Token::new_only_raw(
            value.to_string(),
            pax_manifest::TokenType::LiteralValue,
        ))
    }
}

impl From<SimpleColor> for ValueDefinition {
    fn from(value: SimpleColor) -> Self {
        ValueDefinition::LiteralValue(Token::new_only_raw(
            value.to_string(),
            pax_manifest::TokenType::LiteralValue,
        ))
    }
}

impl From<SimpleProperties> for NodeType {
    fn from(value: SimpleProperties) -> Self {
        let settings: HashMap<Token, Option<ValueDefinition>> = value.into();
        let template_settings = settings
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| SettingElement::Setting(k, v)))
            .collect();
        NodeType::Template(template_settings)
    }
}

impl From<SimpleProperties> for HashMap<Token, Option<ValueDefinition>> {
    fn from(value: SimpleProperties) -> Self {
        let mut settings: HashMap<Token, Option<ValueDefinition>> = HashMap::new();
        settings.insert(
            Token::new_only_raw("x".to_string(), pax_manifest::TokenType::SettingKey),
            Some(value.x.into()),
        );
        settings.insert(
            Token::new_only_raw("y".to_string(), pax_manifest::TokenType::SettingKey),
            Some(value.y.into()),
        );
        settings.insert(
            Token::new_only_raw("width".to_string(), pax_manifest::TokenType::SettingKey),
            Some(value.width.into()),
        );
        settings.insert(
            Token::new_only_raw("height".to_string(), pax_manifest::TokenType::SettingKey),
            Some(value.height.into()),
        );

        settings.insert(
            Token::new_only_raw("fill".to_string(), pax_manifest::TokenType::SettingKey),
            Some(value.fill.into()),
        );

        if let Some(rotation) = value.rotate {
            settings.insert(
                Token::new_only_raw("rotate".to_string(), pax_manifest::TokenType::SettingKey),
                Some(rotation.into()),
            );
        }


        if let Some(text) = value.text {
            // Pest grammar expects a literal value to be an escaped string
            let escaped_text = format!("\"{}\"", text);
            settings.insert(
                Token::new_only_raw("text".to_string(), pax_manifest::TokenType::SettingKey),
                Some(ValueDefinition::LiteralValue(Token::new_only_raw(
                    escaped_text,
                    pax_manifest::TokenType::LiteralValue,
                ))),
            );
        }
        settings
    }
}

impl SimpleNodeAction {
    pub fn build(
        containing_component_type_id: TypeId,
        action: SimpleNodeAction,
    ) -> Vec<NodeAction> {
        match action {
            SimpleNodeAction::Add(adds) => {
                let mut add_actions : Vec<NodeAction> = Vec::new();
                for add in adds.nodes_to_add {
                    let type_id = simple_node_type_to_type_id(add.node_type);
                    if let Some(type_id) = &type_id {
                        let mut node_data : NodeType = add.properties.into();
                        if let Some(pc) = type_id.get_pascal_identifier() {
                            if pc == "MenuBar" {
                                if let NodeType::Template(node_data) = &mut node_data {
                                    for element in node_data.iter_mut() {
                                        if let SettingElement::Setting(key, value) = element {
                                            if key.raw_value == "height" {
                                                *value = SimplePixel{ value: 50.0 }.into();
                                            }
                                            if key.raw_value == "width" {
                                                *value = SimplePercent{ value: 100.0 }.into();
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let tree_location: TreeLocation = TreeLocation::Root;
                        let node_location = NodeLocation::new(
                            containing_component_type_id.clone(),
                            tree_location,
                            TreeIndexPosition::Top,
                        );
    
                        let add = AddTemplateNodeRequest::new(
                            containing_component_type_id.clone(),
                            type_id.clone(),
                            node_data,
                            Some(node_location),
                        );
                        add_actions.push(NodeAction::Add(add));
                    }
                }
                add_actions
            }
        }
    }
}
