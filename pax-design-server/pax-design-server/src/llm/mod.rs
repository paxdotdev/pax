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

use crate::llm::{
    constants::{ADD_FUNCTION, TEMPERATURE},
    simple::SimpleAddRequest,
};

use self::{
    constants::{ADD_DESCRIPTION, MAX_TOKENS, MODEL, SEED, SYSTEM_PROMPT},
    simple::{SimpleAdd, SimpleNodeAction},
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
                    println!("{}", args);
                    let add_operation = serde_json::from_str::<SimpleAddRequest>(&args).unwrap();
                    ret.push(SimpleNodeAction::Add(add_operation));
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
    let add_schema = gen.clone().into_root_schema_for::<SimpleAddRequest>();
    let add_schema_json = json!(add_schema);

    Ok(vec![ChatCompletionToolArgs::default()
        .r#type(ChatCompletionToolType::Function)
        .function(
            FunctionObjectArgs::default()
                .name(ADD_FUNCTION)
                .description(ADD_DESCRIPTION)
                .parameters(add_schema_json)
                .build()?,
        )
        .build()?])
}
