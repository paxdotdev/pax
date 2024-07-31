use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// All possible actions that the LLM can perform
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum SimpleNodeAction {
    Add(SimpleAddRequest),
}

/// All the nodes to be added to the scene
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleAddRequest {
    pub nodes_to_add: Vec<SimpleAdd>,
}

/// A node with the specified properties will be added to the scene at the specified location.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleAdd {
    pub node_type: SimpleNodeType,
    pub properties: SimpleProperties,
}

// Information about the outer viewport
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ViewportInformation {
    pub height: SimplePixel,
    pub width: SimplePixel,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimpleTemplate {
    pub root_nodes: Vec<SimpleNodeInformation>,
    pub children: HashMap<usize, Vec<SimpleNodeInformation>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimpleNodeInformation {
    pub id: usize,
    pub node_info: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimpleWorldInformation {
    pub template: SimpleTemplate,
}

/// The only possible node types that can be added to the scene
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum SimpleNodeType {
    Rectangle,
    Ellipse,
    Text,
    Navbar,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleProperties {
    pub x: SimplePixel,
    pub y: SimplePixel,
    pub width: SimplePixel,
    pub height: SimplePixel,
    pub fill: SimpleColor,
    pub rotate: Option<SimpleRotation>,
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimplePercent {
    pub value: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimplePixel {
    pub value: f32,
}

/// The rotation of a node in degrees
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleRotation {
    pub degrees: f32,
}
