use std::fmt::{Display, Formatter};

use pax_designtime::orm::template::{AddTemplateNodeRequest, NodeAction};
use pax_manifest::{ComponentTemplate, NodeLocation, NodeType, SettingElement, TemplateNodeDefinition, TemplateNodeId, Token, TreeIndexPosition, TreeLocation, TypeId, ValueDefinition};
use serde::{Deserialize, Serialize};

use crate::llm::SimpleNodeType;

use super::{SimpleAddNode, SimpleColor, SimpleLocation, SimpleMoveNode, SimpleNodeInformation, SimpleProperties, SimpleRemoveNode, SimpleSize, SimpleTemplate, SimpleUpdateNode, SizeType};

pub fn convert_to_simple_node_info(id: &TemplateNodeId, node: &TemplateNodeDefinition) -> SimpleNodeInformation {
   let node_type = if let Some(p_i) = node.type_id.get_pascal_identifier() {
        match p_i.as_str() {
            "Rectangle" => {
                SimpleNodeType::Rectangle
            },
            "Ellipse" => {
                SimpleNodeType::Ellipse
            },
            "Text" => {
                SimpleNodeType::Text
            },
            "Group" => {
                SimpleNodeType::Group
            },
            _ => {
                SimpleNodeType::Other
            },
        }
   } else {
         SimpleNodeType::Other
   };

    SimpleNodeInformation {
         id: id.as_usize(),
         node_type,
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
    const PREFIX : &str= "pax_designer::pax_reexports::pax_std::primitives";
    let t = match node_type {
        SimpleNodeType::Rectangle => {
            TypeId::build_singleton(&format!("{}::Rectangle", PREFIX) , None)
        },
        SimpleNodeType::Ellipse => {
            TypeId::build_singleton(&format!("{}::Ellipse", PREFIX) , None)
        },
        SimpleNodeType::Text => {
            TypeId::build_singleton(&format!("{}::Text", PREFIX) , None)
        },
        SimpleNodeType::Group => {
            TypeId::build_singleton(&format!("{}::Group", PREFIX) , None)
        },
        SimpleNodeType::Other => {
            return None;
        },
    };
    Some(t)
}


impl Display for SimpleSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.size_type {
            SizeType::Pixel => write!(f, "{}px", self.value),
            SizeType::Percent => write!(f, "{}%", self.value),
        }
    }
}

impl Display for SimpleColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

impl From<SimpleSize> for ValueDefinition {
    fn from(value: SimpleSize) -> Self {
        ValueDefinition::LiteralValue(Token::new_only_raw(value.to_string(), pax_manifest::TokenType::LiteralValue))
    }
}

impl From<SimpleColor> for ValueDefinition {
    fn from(value: SimpleColor) -> Self {
        ValueDefinition::LiteralValue(Token::new_only_raw(value.to_string(), pax_manifest::TokenType::LiteralValue))
    }
}

impl From<SimpleProperties> for NodeType {
    fn from(value: SimpleProperties) -> Self {
        let mut settings: Vec<SettingElement> = vec![
        ];
        if let Some(x) = value.x {
            settings.push(SettingElement::Setting(Token::new_only_raw("x".to_string(), pax_manifest::TokenType::SettingKey), x.into()));
        }
        if let Some(y) = value.y {
            settings.push(SettingElement::Setting(Token::new_only_raw("y".to_string(), pax_manifest::TokenType::SettingKey), y.into()));
        }
        if let Some(width) = value.width {
            settings.push(SettingElement::Setting(Token::new_only_raw("width".to_string(), pax_manifest::TokenType::SettingKey), width.into()));
        }
        if let Some(height) = value.height {
            settings.push(SettingElement::Setting(Token::new_only_raw("height".to_string(), pax_manifest::TokenType::SettingKey), height.into()));
        }
        if let Some(fill) = value.fill {
            //settings.push(SettingElement::Setting(Token::new_only_raw("fill".to_string(), pax_manifest::TokenType::SettingKey), fill.into()));
        }
        if let Some(text) = value.text {
            settings.push(SettingElement::Setting(Token::new_only_raw("text".to_string(), pax_manifest::TokenType::SettingKey),
                ValueDefinition::LiteralValue(Token::new_only_raw(text, pax_manifest::TokenType::LiteralValue))));
        }
        NodeType::Template(settings)
    }
}

impl From<SimpleLocation> for TreeLocation {
    fn from(value: SimpleLocation) -> Self {
        match value {
            SimpleLocation::Root => TreeLocation::Root,
            SimpleLocation::Parent(parent) => TreeLocation::Parent(TemplateNodeId::build(parent)),
        }
    }

}


pub enum SimpleNodeAction {
    Add(SimpleAddNode),
    Update(SimpleUpdateNode),
    Remove(SimpleRemoveNode),
    Move(SimpleMoveNode),
}

impl SimpleNodeAction {
    pub fn build(containing_component_type_id: TypeId, action: SimpleNodeAction) -> Option<NodeAction> {
        match action {
            SimpleNodeAction::Add(add) => {
                let type_id = simple_node_type_to_type_id(add.node.node_type)?;
                let node_data = if let Some(properties) = add.node.properties {
                    properties.into()
                } else {
                    NodeType::Template(vec![])
                };
                let tree_location : TreeLocation = add.parent_id.into();
                let node_location = NodeLocation::new(containing_component_type_id.clone(), tree_location, TreeIndexPosition::Top);

                let add = AddTemplateNodeRequest::new(containing_component_type_id, type_id, node_data, Some(node_location));
                Some(NodeAction::Add(add))
            },
            SimpleNodeAction::Update(_) => {
               None
            },
            SimpleNodeAction::Remove(_) => {
                None
            },
            SimpleNodeAction::Move(_) => {
                None
            },
        }
    }
}


