use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// All possible actions that the LLM can perform
#[derive(Debug)]
pub enum SimpleNodeAction {
    Add(SimpleAdd),
    Update(SimpleUpdate),
    Remove(SimpleRemove),
}

/// A node with the specified properties will be added to the scene at the specified location.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleAdd {
    pub node_type: SimpleNodeType,
    pub properties: Option<SimpleProperties>,
}

/// The node specified by id will be updated with the new properties or new parent
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleUpdate {
    pub id: usize,
    pub properties: Option<SimpleProperties>,
}

/// The node specified by id will be removed from the scene
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleRemove {
    pub id: usize,
}

// Information about the outer viewport
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ViewportInformation {
    pub height: SimpleSize,
    pub width: SimpleSize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimpleTemplate {
    pub root_nodes: Vec<SimpleNodeInformation>,
    pub children: HashMap<usize, Vec<SimpleNodeInformation>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimpleNodeInformation {
    pub id: usize,
    pub node_type: SimpleNodeType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimpleWorldInformation {
    pub template: SimpleTemplate,
}

/// The only possible node types that can be added to the scene. Only Group and Other can have children
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum SimpleNodeType {
    Rectangle,
    Ellipse,
    Text,
    Group,
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleNode {
    pub id: usize,
    pub node_type: SimpleNodeType,
    pub properties: Option<SimpleProperties>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleProperties {
    pub x: Option<SimpleSize>,
    pub y: Option<SimpleSize>,
    pub width: Option<SimpleSize>,
    pub height: Option<SimpleSize>,
    pub fill: Option<SimpleColor>,
    pub stroke: Option<SimpleColor>,
    pub rotate: Option<SimpleRotation>,
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum SimpleSizeType {
    Pixel,
    Percent,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleSize {
    pub value: f32,
    pub size_type: SimpleSizeType,
}

/// The rotation of a node in degrees
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleRotation {
    pub degrees: f32,
}
