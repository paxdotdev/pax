use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
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
    SimpleColor, SimpleLocation, SimpleNodeAction, SimpleNodeInformation, SimpleNodeType,
    SimpleProperties, SimpleRotation, SimpleSize, SimpleSizeType, SimpleTemplate,
};

use super::constants::PREFIX;

pub fn convert_to_simple_node_info(
    id: &TemplateNodeId,
    node: &TemplateNodeDefinition,
) -> SimpleNodeInformation {
    let node_type = if let Some(p_i) = node.type_id.get_pascal_identifier() {
        match p_i.as_str() {
            "Rectangle" => SimpleNodeType::Rectangle,
            "Ellipse" => SimpleNodeType::Ellipse,
            "Text" => SimpleNodeType::Text,
            "Group" => SimpleNodeType::Group,
            _ => SimpleNodeType::Other,
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
    let t = match node_type {
        SimpleNodeType::Rectangle => {
            TypeId::build_singleton(&format!("{}::Rectangle", PREFIX), None)
        }
        SimpleNodeType::Ellipse => TypeId::build_singleton(&format!("{}::Ellipse", PREFIX), None),
        SimpleNodeType::Text => TypeId::build_singleton(&format!("{}::Text", PREFIX), None),
        SimpleNodeType::Group => TypeId::build_singleton(&format!("{}::Group", PREFIX), None),
        SimpleNodeType::Other => {
            return None;
        }
    };
    Some(t)
}

impl Display for SimpleSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.size_type {
            SimpleSizeType::Pixel => write!(f, "{}px", self.value),
            SimpleSizeType::Percent => write!(f, "{}%", self.value),
        }
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

impl From<SimpleSize> for ValueDefinition {
    fn from(value: SimpleSize) -> Self {
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
        if let Some(x) = value.x {
            settings.insert(
                Token::new_only_raw("x".to_string(), pax_manifest::TokenType::SettingKey),
                Some(x.into()),
            );
        }
        if let Some(y) = value.y {
            settings.insert(
                Token::new_only_raw("y".to_string(), pax_manifest::TokenType::SettingKey),
                Some(y.into()),
            );
        }
        if let Some(width) = value.width {
            settings.insert(
                Token::new_only_raw("width".to_string(), pax_manifest::TokenType::SettingKey),
                Some(width.into()),
            );
        }
        if let Some(height) = value.height {
            settings.insert(
                Token::new_only_raw("height".to_string(), pax_manifest::TokenType::SettingKey),
                Some(height.into()),
            );
        }

        if let Some(rotate) = value.rotate {
            settings.insert(
                Token::new_only_raw("rotate".to_string(), pax_manifest::TokenType::SettingKey),
                Some(rotate.into()),
            );
        }

        if let Some(fill) = value.fill {
            settings.insert(Token::new_only_raw("fill".to_string(), pax_manifest::TokenType::SettingKey), Some(fill.into()));
        }

        if let Some(stroke) = value.stroke {
            settings.insert(Token::new_only_raw("stroke".to_string(), pax_manifest::TokenType::SettingKey), Some(stroke.into()));
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

impl From<SimpleLocation> for TreeLocation {
    fn from(value: SimpleLocation) -> Self {
        match value {
            SimpleLocation::Root => TreeLocation::Root,
            SimpleLocation::Parent(parent) => TreeLocation::Parent(TemplateNodeId::build(parent)),
        }
    }
}

impl SimpleNodeAction {
    pub fn build(
        containing_component_type_id: TypeId,
        action: SimpleNodeAction,
    ) -> Option<NodeAction> {
        match action {
            SimpleNodeAction::Add(add) => {
                let type_id = simple_node_type_to_type_id(add.node_type)?;
                let node_data = if let Some(properties) = add.properties {
                    properties.into()
                } else {
                    NodeType::Template(vec![])
                };
                let tree_location: TreeLocation = add.parent_id.into();
                let node_location = NodeLocation::new(
                    containing_component_type_id.clone(),
                    tree_location,
                    TreeIndexPosition::Top,
                );

                let add = AddTemplateNodeRequest::new(
                    containing_component_type_id,
                    type_id,
                    node_data,
                    Some(node_location),
                );
                Some(NodeAction::Add(add))
            }
            SimpleNodeAction::Update(update) => {
                let template_node_id = TemplateNodeId::build(update.id);
                let uni = UniqueTemplateNodeIdentifier::build(
                    containing_component_type_id.clone(),
                    template_node_id,
                );
                let mut updated_property_map = HashMap::new();
                if let Some(properties) = update.properties {
                    updated_property_map = properties.into();
                }
                let new_location = update.new_parent.map(|new_parent| {
                    NodeLocation::new(
                        containing_component_type_id.clone(),
                        new_parent.into(),
                        TreeIndexPosition::Top,
                    )
                });
                let update =
                    UpdateTemplateNodeRequest::new(uni, updated_property_map, new_location);
                Some(NodeAction::Update(update))
            }
            SimpleNodeAction::Remove(remove) => {
                let template_node_id = TemplateNodeId::build(remove.id);
                let uni = UniqueTemplateNodeIdentifier::build(
                    containing_component_type_id,
                    template_node_id,
                );
                let remove = RemoveTemplateNodeRequest::new(uni);
                Some(NodeAction::Remove(remove))
            }
        }
    }
}
