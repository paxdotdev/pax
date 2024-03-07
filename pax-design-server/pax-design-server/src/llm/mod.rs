use std::{collections::HashMap, fmt::{Display, Formatter}, fs, sync::{Arc, Mutex}};

use async_openai::{config::OpenAIConfig, error::OpenAIError, types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, ChatCompletionToolArgs, ChatCompletionToolType, CreateChatCompletionRequestArgs, FunctionObjectArgs}, Client};
use pax_manifest::{ComponentTemplate, NodeType, SettingElement, TemplateNodeDefinition, TemplateNodeId, Token, TypeId, ValueDefinition};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub mod helpers;
use schemars::{schema_for, JsonSchema, gen::SchemaSettings};

use self::helpers::SimpleNodeAction;

pub async fn query_open_ai(request: &str) -> Result<Vec<SimpleNodeAction>, OpenAIError> {
    let mut settings = SchemaSettings::openapi3();
    settings.inline_subschemas = true;
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<SimpleAddNode>();


    
    // let schema = schema_for!(SimpleAddNode);
    let json = json!(schema);

    let string_schema = serde_json::to_string_pretty(&schema).unwrap();
    //println!("Schema: {}", string_schema);

    let client = Client::new();

    let system_prompt = fs::read_to_string("src/llm/system_prompt.txt").unwrap();

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(2000u16)
        .model("gpt-4-turbo-preview")
        .messages(
            [ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()?
            .into(),
            ChatCompletionRequestUserMessageArgs::default()
            .content(request)
            .build()?
            .into()])
        .tools(vec![ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                FunctionObjectArgs::default()
                    .name("add_a_node")
                    .description("Add a node to the scene")
                    .parameters(json)
                    .build()?,
            )
            .build()?])
        .build()?;

    println!("Making request!");
    let response_message = client
    .chat()
    .create(request)
    .await.unwrap();

    println!("Response: {:?}", response_message);


    let response_message = response_message
        .choices
        .get(0)
        .unwrap()
        .message
        .clone();

    let mut ret : Vec<SimpleNodeAction> = vec![];
    if let Some(tool_calls) = response_message.tool_calls {
        for tool_call in tool_calls {
            let name = tool_call.function.name.clone();
            let args = tool_call.function.arguments.clone();
            let tool_call_clone = tool_call.clone();
            println!("Tool Call: {}", args);
            let add_operation = serde_json::from_str::<SimpleAddNode>(&args).unwrap();
            println!("Tool Call: {:?}", add_operation);
            ret.push(SimpleNodeAction::Add(add_operation));
        }
    }

    Ok(ret)

}





/// API ACTIONS 
/// 
/// Add

/// Nodes will be added at the specified location, above all the existing nodes at that level (i.e. Nodes added to the front).
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleAddNode {
    pub parent_id: SimpleLocation,
    pub node: SimpleNode,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleUpdateNode {
    pub id: usize,
    pub properties: SimpleProperties,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleRemoveNode {
    pub id: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleMoveNode {
    pub id: usize,
    pub new_parent: SimpleLocation,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum SizeType {
    Pixel,
    Percent,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SimpleSize {
    pub value: f32,
    pub size_type: SizeType,
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
    pub viewport: ViewportInformation,
    pub template: SimpleTemplate,
}


/// The only possible node types that can be added to the scene
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum SimpleNodeType {
    Rectangle,
    Ellipse,
    Text,
    Group,
    Other,
}



/// Information needed for Additions, Updates, and Removals

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
    pub text: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum SimpleLocation {
    Root,
    Parent(usize),
}