use std::fs;

use async_openai::{
    error::OpenAIError,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        ChatCompletionTool, ChatCompletionToolArgs, ChatCompletionToolType,
        CreateChatCompletionRequestArgs, FunctionObjectArgs,
    },
    Client,
};
use serde_json::json;

pub mod helpers;
use schemars::gen::SchemaSettings;

use crate::llm::constants::{ADD_FUNCTION, REMOVE_FUNCTION, TEMPERATURE, UPDATE_FUNCTION};

use self::{
    constants::{
        ADD_DESCRIPTION, MAX_TOKENS, MODEL, REMOVE_DESCRIPTION, SEED, SYSTEM_PROMPT,
        UPDATE_DESCRIPTION,
    },
    simple::{SimpleAdd, SimpleNodeAction, SimpleRemove, SimpleUpdate},
};
pub mod constants;
pub mod simple;

/// Performs an OpenAI query with our built-in ORM operations and returns the SimpleNodeActions that need to be performed
pub async fn query_open_ai(request: &str) -> Result<Vec<SimpleNodeAction>, OpenAIError> {
    let client = Client::new();

    let system_prompt = fs::read_to_string(SYSTEM_PROMPT).unwrap();

    let request = CreateChatCompletionRequestArgs::default()
        .seed(SEED)
        .max_tokens(MAX_TOKENS)
        .model(MODEL)
        .temperature(TEMPERATURE)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(request)
                .build()?
                .into(),
        ])
        .tools(create_tools()?)
        .build()?;

    let response = client.chat().create(request).await?;

    let completion = response.choices.first().unwrap().message.clone();

    let mut ret: Vec<SimpleNodeAction> = vec![];
    if let Some(tool_calls) = completion.tool_calls {
        for tool_call in tool_calls {
            let name = tool_call.function.name.clone();
            let args = tool_call.function.arguments.clone();

            match name.as_str() {
                ADD_FUNCTION => {
                    let add_operation = serde_json::from_str::<SimpleAdd>(&args).unwrap();
                    ret.push(SimpleNodeAction::Add(add_operation));
                }
                UPDATE_FUNCTION => {
                    let update_operation = serde_json::from_str::<SimpleUpdate>(&args).unwrap();
                    ret.push(SimpleNodeAction::Update(update_operation));
                }
                REMOVE_FUNCTION => {
                    let remove_operation = serde_json::from_str::<SimpleRemove>(&args).unwrap();
                    ret.push(SimpleNodeAction::Remove(remove_operation));
                }
                _ => {}
            }
        }
    }

    Ok(ret)
}

/// Configures the SimpleNodeActions as ChatCompletionTools
pub fn create_tools() -> Result<Vec<ChatCompletionTool>, OpenAIError> {
    // Configure Schema settings
    let mut settings = SchemaSettings::openapi3();
    settings.inline_subschemas = true;
    let gen = settings.into_generator();

    // Generate the schema for the SimpleAddNode
    let add_schema = gen.clone().into_root_schema_for::<SimpleAdd>();
    let add_scehma_json = json!(add_schema);

    // Generate the schema for the SimpleUpdateNode
    let update_schema = gen.clone().into_root_schema_for::<SimpleUpdate>();
    let update_schema_json = json!(update_schema);

    // Generate the schema for the SimpleRemoveNode
    let remove_schema = gen.clone().into_root_schema_for::<SimpleRemove>();
    let remove_schema_json = json!(remove_schema);

    Ok(vec![
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                FunctionObjectArgs::default()
                    .name(ADD_FUNCTION)
                    .description(ADD_DESCRIPTION)
                    .parameters(add_scehma_json)
                    .build()?,
            )
            .build()?,
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                FunctionObjectArgs::default()
                    .name(UPDATE_FUNCTION)
                    .description(UPDATE_DESCRIPTION)
                    .parameters(update_schema_json)
                    .build()?,
            )
            .build()?,
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                FunctionObjectArgs::default()
                    .name(REMOVE_FUNCTION)
                    .description(REMOVE_DESCRIPTION)
                    .parameters(remove_schema_json)
                    .build()?,
            )
            .build()?,
    ])
}
