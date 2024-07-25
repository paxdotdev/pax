#![allow(dead_code)]
/// LLM Setup constants

pub const GPT_4_TURBO_PREVIEW: &str = "gpt-4-turbo-preview";
pub const GPT_3_TURBO: &str = "gpt-3.5-turbo-0125";

pub const SEED: i64 = 1234i64;
pub const MAX_TOKENS: u16 = 1000u16;
pub const MODEL: &str = GPT_4_TURBO_PREVIEW;
pub const SYSTEM_PROMPT: &str = "src/llm/system_prompt.txt";
pub const TEMPERATURE: f32 = 1.0f32;

/// LLM function constants

/// ADD
pub const ADD_FUNCTION: &str = "add_a_node";
pub const ADD_DESCRIPTION: &str = "Add nodes to the template";

/// Other constants
pub const PREFIX: &str = "pax_designer::pax_reexports::pax_std::primitives";
pub const TRAINING_DATA_PATH: &str = "src/llm/future_training_data/";
pub const TRAINING_DATA_BEFORE_REQUEST: &str = "before.pax";
pub const TRAINING_DATA_AFTER_REQUEST: &str = "after.pax";
pub const TRAINING_DATA_REQUEST: &str = "request.txt";
