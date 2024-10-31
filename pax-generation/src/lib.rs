use pax_lang::parse_pax_err;
use pax_lang::Rule;
use pax_message::ScreenshotData;
use regex::Regex;
use reqwest;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use futures::channel::mpsc;

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

// Include the system prompt from the file in the cargo manifest root
const SYSTEM_PROMPT: &str = include_str!("../system_prompt_v3.txt");

macro_rules! project_root {
    () => {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    };
}

fn output_dir() -> PathBuf {
    project_root!().join("generated_project")
}

#[derive(Clone, Copy)]
pub enum AIModel {
    Claude3,
    GPT4o,
    GPT4oMini,
    O1,
    O1Mini,
}

impl AIModel {
    fn as_str(&self) -> &'static str {
        match self {
            AIModel::Claude3 => "claude-3-5-sonnet-20240620",
            AIModel::GPT4o => "gpt-4o",
            AIModel::GPT4oMini => "gpt-4o-mini",
            AIModel::O1 => "o1-preview",
            AIModel::O1Mini => "o1-mini",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct ImageUrl {
    url: String,
    detail: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum ContentItem {
    Text {
        #[serde(rename = "type")]
        content_type: String,
        text: String,
    },
    Image {
        #[serde(rename = "type")]
        content_type: String,
        image_url: ImageUrl,
    },
}

#[derive(Clone)]
struct Message {
    role: String,
    content: Vec<ContentItem>,
}
pub struct PaxAppGenerator {
    api_key: String,
    model: AIModel,
}

impl PaxAppGenerator {
    async fn send_prompt(&self, messages: &[Message]) -> Result<String, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let auth_header = format!("Bearer {}", self.api_key);

        let (url, headers, body) = match self.model {
            AIModel::Claude3 => {
                // Convert the new message format to Claude's expected format
                let (system_message, user_messages): (Option<&Message>, Vec<&Message>) =
                    if !messages.is_empty() && messages[0].role == "system" {
                        (Some(&messages[0]), messages[1..].iter().collect())
                    } else {
                        (None, messages.iter().collect())
                    };

                let api_messages: Vec<Value> = user_messages
                    .iter()
                    .map(|m| {
                        let content = m.content
                            .iter()
                            .map(|c| match c {
                                ContentItem::Text { text, .. } => text.clone(),
                                ContentItem::Image { .. } => String::new(), // Claude doesn't support images
                            })
                            .collect::<Vec<String>>()
                            .join("\n");
                        
                        json!({ "role": &m.role, "content": content })
                    })
                    .collect();

                let mut body = json!({
                    "model": self.model.as_str(),
                    "max_tokens": 8192,
                    "messages": api_messages,
                    "temperature": 0.5,
                });

                if let Some(sys_msg) = system_message {
                    let system_content = sys_msg.content
                        .iter()
                        .map(|c| match c {
                            ContentItem::Text { text, .. } => text.clone(),
                            ContentItem::Image { .. } => String::new(),
                        })
                        .collect::<Vec<String>>()
                        .join("\n");
                    body["system"] = json!(system_content);
                }

                (
                    CLAUDE_API_URL,
                    vec![
                        ("content-type", "application/json"),
                        ("x-api-key", &self.api_key),
                        ("anthropic-version", "2023-06-01"),
                    ],
                    body,
                )
            }
            AIModel::GPT4o | AIModel::GPT4oMini | AIModel::O1 | AIModel::O1Mini => {
                let api_messages: Vec<Value> = messages
                    .iter()
                    .map(|m| json!({
                        "role": &m.role,
                        "content": &m.content
                    }))
                    .collect();

                let body = json!({
                    "model": self.model.as_str(),
                    "messages": api_messages,
                    "max_tokens": 8192,
                    "temperature": 0.1,
                });

                (
                    OPENAI_API_URL,
                    vec![
                        ("content-type", "application/json"),
                        ("Authorization", &auth_header),
                    ],
                    body,
                )
            }
        };

        let mut request = client.post(url);
        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request.json(&body).send().await?.json::<Value>().await?;

        match self.model {
            AIModel::Claude3 => {
                if let Some(error) = response.get("error") {
                    Err(format!("API Error: {:?}", error).into())
                } else {
                    response["content"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|obj| obj["text"].as_str())
                        .ok_or_else(|| "Unexpected response format for Claude".into())
                        .map(String::from)
                }
            }
            AIModel::GPT4o | AIModel::O1 | AIModel::O1Mini | AIModel::GPT4oMini => {
                if let Some(error) = response.get("error") {
                    Err(format!("API Error: {:?}", error).into())
                } else {
                    response["choices"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|obj| obj["message"]["content"].as_str())
                        .ok_or_else(|| {
                            format!("Unexpected response format for GPT-4o. Response: {:?}", response).into()
                        })
                        .map(String::from)
                }
            }
        }
    }
    pub async fn update_pax_file(
        &self,
        prompt: &str,
        request_id: u64,
        tx: mpsc::UnboundedSender<(u64, String)>,
        screenshot: Option<ScreenshotData>,
        project_files: Vec<(String, String)>,
    ) -> Result<(Vec<(String, String)>,Vec<(String, String)>, String), Box<dyn Error>> {
        let project_files = project_files.iter().filter(|(f, _)| f.ends_with(".rs") || f.ends_with(".pax")).cloned().collect::<Vec<_>>();
        // Write the current project files to the generated_project directory
        self.write_files_to_directory(&output_dir(), &project_files)?;

        // Create the initial text content by including all project files
        let mut files_text = String::new();
        for (filename, content) in &project_files {
            let code_block = match filename.split('.').last() {
                Some("pax") => format!("```pax\n{}\n```", content),
                Some("rs") => format!("```rust\n{}\n```", content),
                _ => format!("```text\n{}\n```", content),
            };
            files_text.push_str(&format!("Filename: {}\n\n{}\n\n", filename, code_block));
        }

        let text_content = ContentItem::Text {
            content_type: "text".to_string(),
            text: format!(
                "Here are the current project files:\n\n{}\nPlease update the project files based on the following request:\n{}",
                files_text, prompt
            ),
        };
        // Create the initial message content vector with the text
        let mut message_content = vec![text_content];

        // Add screenshot if available
        if let Some(screenshot) = screenshot {
            // Create image buffer
            let img = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
                screenshot.width as u32,
                screenshot.height as u32,
                screenshot.data,
            )
            .expect("Failed to create image buffer");

            // Create a cursor-based buffer
            let mut buffer = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buffer, image::ImageFormat::Jpeg)?;

            // write the image to a file
            img.save("screenshot.jpeg")?;

            // Get the bytes from the cursor
            let bytes = buffer.into_inner();

            // Convert to base64
            let base64_image = base64::encode(&bytes);
            let url = format!("data:image/jpeg;base64,{}", base64_image);
            // Add image content
            message_content.push(ContentItem::Image {
                content_type: "image_url".to_string(),
                image_url: ImageUrl {
                    url,
                    detail: "high".to_string(),
                },
            });
        }

        let mut messages = vec![
            Message {
                role: "system".to_string(),
                content: vec![ContentItem::Text {
                    content_type: "text".to_string(),
                    text: SYSTEM_PROMPT.to_string(),
                }],
            },
            Message {
                role: "user".to_string(),
                content: message_content,
            },
        ];

        let mut retry_count = 0;
        const MAX_RETRIES: usize = 5;

        let mut total_files: HashMap<String, String> = project_files.clone().into_iter().collect();


        while retry_count < MAX_RETRIES {
            tx.unbounded_send((request_id, "--- Sent request to OpenAI ---".to_string()))?;
            let response = self.send_prompt(&messages).await?;
            tx.unbounded_send((request_id, "Validating Response from OpenAI.".to_string()))?;

            // Add assistant's response as text-only content
            messages.push(Message {
                role: "assistant".to_string(),
                content: vec![ContentItem::Text {
                    content_type: "text".to_string(),
                    text: response.clone(),
                }],
            });

            match self.parse_response(&response) {
                Ok(resp) => {
                    let (rust_files, pax_files, resp) = resp;

                    for (filename, content) in rust_files.iter().chain(pax_files.iter()) {
                        total_files.insert(filename.clone(), content.clone());
                    }

                    let parse_errors = self.pre_parse_pax_files(&pax_files);
                    if parse_errors.is_empty() {

                        let files : Vec<(String, String)> = total_files.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                        // Write the updated files to the generated_project directory
                        self.write_files_to_directory(&output_dir(),&files)?;

                        // Only proceed if there are Rust files
                        if !rust_files.is_empty() {
                            // Try compiling the project
                            tx.unbounded_send((request_id, "Attempting to compile the project.".to_string()))?;
                            match self.compile_and_run_project() {
                                Ok(_) => {
                                    // Compilation succeeded
                                    tx.unbounded_send((request_id, "Compilation succeeded.".to_string()))?;

                                    let rust_files = total_files.iter().filter(|(f, _)| f.ends_with(".rs")).map(|(f, c)| (f.clone(), c.clone())).collect();
                                    let pax_files = total_files.iter().filter(|(f, _)| f.ends_with(".pax")).map(|(f, c)| (f.clone(), c.clone())).collect();

                                    return Ok((rust_files, pax_files, resp));
                                }
                                Err(compile_errors) => {
                                    // Compilation failed, use the error message directly
                                    let error_message = format!(
                                        "The updated project failed to compile. Error: {}. Please fix the compilation errors and provide the corrected code.",
                                        compile_errors
                                    );
                                    messages.push(Message {
                                        role: "user".to_string(),
                                        content: vec![ContentItem::Text {
                                            content_type: "text".to_string(),
                                            text: error_message,
                                        }],
                                    });
                                    tx.unbounded_send((request_id, "Sending compilation errors to AI for correction.".to_string()))?;
                                    retry_count += 1;
                                    continue;
                                }
                            }
                        } else {
                            tx.unbounded_send((request_id, "Parse succeeded.".to_string()))?;
                            let pax_files = total_files.iter().filter(|(f, _)| f.ends_with(".pax")).map(|(f, c)| (f.clone(), c.clone())).collect();

                            // No Rust files, return the updated files
                            return Ok((rust_files, pax_files, resp));
                        }
                    } else {
                        tx.unbounded_send((request_id, "PAX parsing errors detected:".to_string()))?;
                        let error_message = format!(
                            "The updated PAX file failed to parse. Error: {}. Please fix the PAX syntax errors and provide the corrected code.",
                            parse_errors[0].1
                        );
                        println!("Error message: {}", error_message);
                        messages.push(Message {
                            role: "user".to_string(),
                            content: vec![ContentItem::Text {
                                content_type: "text".to_string(),
                                text: error_message,
                            }],
                        });
                        tx.unbounded_send((request_id,"Sending PAX parsing errors to AI for correction.".to_string()))?;
                        retry_count += 1;
                    }
                }
                Err(e) => {
                    let error_message = format!(
                        "The previous response could not be parsed correctly. Error: {}. Please provide the updated project files again, ensuring that they're properly formatted within code blocks with correct syntax highlighting and optional filenames.",
                        e
                    );
                    messages.push(Message {
                        role: "user".to_string(),
                        content: vec![ContentItem::Text {
                            content_type: "text".to_string(),
                            text: error_message,
                        }],
                    });
                    retry_count += 1;
                }
            }
        }

        Err(format!(
            "Maximum retries ({}) reached while updating project files.",
            MAX_RETRIES
        )
        .into())
    }


    pub fn new(api_key: String, model: AIModel) -> Self {
        PaxAppGenerator { api_key, model }
    }

    fn parse_response(
        &self,
        response: &str,
    ) -> Result<(Vec<(String, String)>, Vec<(String, String)>, String), Box<dyn Error>> {
        let rust_regex = Regex::new(r"(?s)```rust(?: filename=(.*?\.rs))?\n(.*?)```")?;
        let toml_regex = Regex::new(r"(?s)```toml(?: filename=(.*?\.toml))?\n(.*?)```")?;
        let pax_regex = Regex::new(r"(?s)```pax(?: filename=(.*?\.pax))?\n(.*?)```")?;

        let mut rust_files = Vec::new();

        let mut error_message = String::new();

        for cap in rust_regex.captures_iter(response) {
            let filename = cap.get(1);
            if let Some(filename) = filename {
                let filename = filename.as_str().to_string();
                let content = cap[2].trim().to_string();
                rust_files.push((filename, content));
            } else {
                error_message += "\nRust file missing filename, regex to match: ```rust(?: filename=(.*?\\.rs))?\\n(.*?)```";    
            }
        }

        for cap in toml_regex.captures_iter(response) {
            let filename = cap.get(1);
            if let Some(filename) = filename {
                let filename = filename.as_str().to_string();
                let content = cap[2].trim().to_string();
                rust_files.push((filename, content));
            } else {
                error_message += "\nTOML file missing filename, regex to match: ```toml(?: filename=(.*?\\.toml))?\\n(.*?)```";
            }
        }

        let mut pax_files = Vec::new();
        for cap in pax_regex.captures_iter(response) {
            let filename = cap.get(1);
            if let Some(filename) = filename {
                let filename = filename.as_str().to_string();
                let content = cap[2].trim().to_string();
                pax_files.push((filename, content));
            } else {
                error_message += "\nPAX file missing filename regex to match: ```pax(?: filename=(.*?\\.pax))?\\n(.*?)```";
            }
        }

        if !error_message.is_empty() {
            error_message = format!("Error parsing response: Please REDO WITH CORRECT FILENAMES\n {}", error_message);
            return Err(error_message.into());
        }

        if rust_files.is_empty() && pax_files.is_empty() {
            return Err("No Rust or PAX files found in response".into());
        }
        Ok((rust_files, pax_files, response.to_string()))
    }


    fn pre_parse_pax_files(&self, pax_files: &[(String, String)]) -> Vec<(String, String)> {
        let mut parse_errors = Vec::new();

        for (filename, content) in pax_files {
            match parse_pax_err(Rule::pax_component_definition, content) {
                Ok(_) => {},
                Err(e) => parse_errors.push((filename.clone(), e.to_string())),
            }
        }

        parse_errors
    }

    fn write_files_to_directory(&self, dir: &Path, files: &[(String, String)]) -> io::Result<()> {
        // Create the directory if it doesn't exist
        fs::create_dir_all(dir)?;

        // Write or update files
        for (filename, content) in files {
            let file_path = dir.join(filename);

            // Create parent directories if they don't exist
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&file_path, content)?;
            println!("Wrote file: {}", file_path.display());
        }

        Ok(())
    }

    fn compile_and_run_project(&self) -> Result<(), Box<dyn Error>> {
        let output = Command::new("./pax")
            .current_dir(output_dir())
            .arg("build")
            .output()?;

        if output.status.success() {
            println!("Project built successfully");
            Ok(())
        } else {
            println!(
                "Build failed. Error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Err(String::from_utf8_lossy(&output.stderr).into())
        }
    }

    fn read_directory_files(&self, dir: &Path) -> io::Result<String> {
        let mut files_content = String::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap().to_str().unwrap();
                let content = fs::read_to_string(&path)?;
                files_content.push_str(&format!("Filename: {}\n\n{}\n\n", filename, content));
            }
        }
        Ok(files_content)
    }

}
