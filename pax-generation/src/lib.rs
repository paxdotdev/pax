use pax_lang::parse_pax_err;
use pax_lang::Rule;
use pax_message::ScreenshotData;
use regex::Regex;
use reqwest;
use serde::Serialize;
use serde_json::{json, Value};
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
struct ClaudeImageUrl {
    #[serde(rename = "type")]
    content_type: String,
    media_type: String,
    data: String,
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
    ClaudeImage {
        #[serde(rename = "type")]
        content_type: String,
        source: ClaudeImageUrl,
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
                                ContentItem::Image { .. } => String::new(),
                                ContentItem::ClaudeImage { .. } => String::new(),//json!(c).to_string(),
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
                            ContentItem::ClaudeImage { .. } => String::new(),//json!(c).to_string(),
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
        pax_content: &str,
        prompt: &str,
        request_id: u64,
        tx: mpsc::UnboundedSender<(u64, String)>,
        screenshot: Option<ScreenshotData>,
        model: &AIModel,
    ) -> Result<(String, String), Box<dyn Error>> {
        // Create the initial text content
        let text_content = ContentItem::Text {
            content_type: "text".to_string(),
            text: format!(
                "Here's the current PAX file content:\n\n```pax\n{}\n```\n\nPlease update this PAX file based on the following request:\n{}",
                pax_content, prompt
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
            screenshot.data
        ).expect("Failed to create image buffer");

        // Calculate new dimensions maintaining aspect ratio
        let max_dimension = 1440;
        let (width, height) = (img.width(), img.height());
        let ratio = width as f32 / height as f32;
        let (new_width, new_height) = if width > height {
            if width > max_dimension {
                (max_dimension, (max_dimension as f32 / ratio) as u32)
            } else {
                (width, height)
            }
        } else {
            if height > max_dimension {
                ((max_dimension as f32 * ratio) as u32, max_dimension)
            } else {
                (width, height)
            }
        };

        // Resize the image
        let resized = image::imageops::resize(
            &img,
            new_width,
            new_height,
            image::imageops::FilterType::Lanczos3
        );

        // Create a cursor-based buffer
        let mut buffer = std::io::Cursor::new(Vec::new());
        resized.write_to(&mut buffer, image::ImageFormat::Jpeg)?;

        // write the image to a file
        resized.save("screenshot.jpeg")?;

        // Get the bytes from the cursor
        let bytes = buffer.into_inner();
        
        // Convert to base64 but utf8
        let base64_image = base64::encode(&bytes);
        let url = format!("data:image/jpeg;base64,{}", base64_image.clone());
        // Add image content

        match model {
            AIModel::Claude3 => {
                message_content.push(ContentItem::ClaudeImage {
                    content_type: "image".to_string(),
                    source: ClaudeImageUrl {
                        content_type: "base64".to_string(),
                        media_type: "image/jpeg".to_string(),
                        data: base64_image,
                    }
                });
            },
            _ => {
                message_content.push(ContentItem::Image {
                    content_type: "image_url".to_string(),
                    image_url: ImageUrl {
                        url,
                        detail: "high".to_string(),
                    },
                });
            }
        }
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

        while retry_count < MAX_RETRIES {
            tx.unbounded_send((request_id, format!("--- Sent request to {} ---", self.model.as_str())))?;
            let response = self.send_prompt(&messages).await?;
            tx.unbounded_send((request_id, "Received response.".to_string()))?;

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
                    let (_, pax_files, resp) = resp;
                    let parse_errors = self.pre_parse_pax_files(&pax_files);
                    if parse_errors.is_empty() {
                        return Ok((pax_files[0].1.clone(), resp));
                    } else {
                        tx.unbounded_send((request_id,"PAX parsing errors detected:".to_string()))?;
                        let error_message = format!(
                            "The updated PAX file failed to parse. Error: {}. Please fix the PAX syntax errors and provide the corrected code.",
                            parse_errors[0].1
                        );
                        messages.push(Message {
                            role: "user".to_string(),
                            content: vec![ContentItem::Text {
                                content_type: "text".to_string(),
                                text: error_message,
                            }],
                        });
                        tx.unbounded_send((request_id,"Sending error message to AI for correction.".to_string()))?;
                        retry_count += 1;
                    }
                }
                Err(e) => {
                    let error_message = format!(
                        "The previous response could not be parsed correctly. Error: {}. Please provide the updated PAX file again, ensuring that it's properly formatted within a PAX code block.",
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
            "Maximum retries ({}) reached while updating PAX file.",
            MAX_RETRIES
        )
        .into())
    }

    pub fn new(api_key: String, model: AIModel) -> Self {
        PaxAppGenerator { api_key, model }
    }

    // New method to copy all contents from one directory to another
    fn copy_directory_contents(&self, from: &Path, to: &Path) -> io::Result<()> {
        // Convert both paths to absolute paths
        let abs_from = fs::canonicalize(from)?;
        let abs_to = fs::canonicalize(to)?;

        if abs_from == abs_to {
            println!("Source and destination are the same. Nothing to copy.");
            return Ok(());
        }

        fs::create_dir_all(&abs_to)?;
        for entry in fs::read_dir(&abs_from)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let dest_path = abs_to.join(path.file_name().unwrap());
                fs::copy(&path, &dest_path)?;
                println!("Copied: {}", dest_path.display());
            }
        }
        Ok(())
    }

    // New method to read all files in a directory as a Vec<(String, String)>
    fn read_directory_files_as_vec(&self, dir: &Path) -> io::Result<Vec<(String, String)>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                let content = fs::read_to_string(&path)?;
                files.push((filename, content));
            }
        }
        Ok(files)
    }

    fn replace_main_struct_name_in_file(&self, content: &str) -> String {
        let main_struct_re =
            Regex::new(r"(?m)^#\[main\]\s*(?:#\[(?:pax|file\([^\)]+\))\]\s*)*pub struct (\w+)")
                .unwrap();

        if let Some(captures) = main_struct_re.captures(content) {
            if let Some(struct_name) = captures.get(1) {
                let struct_name = struct_name.as_str();
                let struct_name_re =
                    Regex::new(&format!(r"\b{}\b", regex::escape(struct_name))).unwrap();
                return struct_name_re.replace_all(content, "Example").to_string();
            }
        }

        content.to_string()
    }

    // async fn send_prompt(&self, messages: &[Message]) -> Result<String, Box<dyn Error>> {
    //     let client = reqwest::Client::new();

    //     let auth_header = format!("Bearer {}", self.api_key);

    //     let (url, headers, body) = match self.model {
    //         AIModel::Claude3 => {
    //             let (system_message, user_messages): (Option<&Message>, Vec<&Message>) =
    //                 if !messages.is_empty() && messages[0].role == "system" {
    //                     (Some(&messages[0]), messages[1..].iter().collect())
    //                 } else {
    //                     (None, messages.iter().collect())
    //                 };

    //             let api_messages: Vec<Value> = user_messages
    //                 .iter()
    //                 .map(|m| json!({ "role": &m.role, "content": &m.content }))
    //                 .collect();

    //             let mut body = json!({
    //                 "model": self.model.as_str(),
    //                 "max_tokens": 8192,
    //                 "messages": api_messages,
    //                 "temperature": 0.5,
    //             });

    //             if let Some(sys_msg) = system_message {
    //                 body["system"] = json!(&sys_msg.content);
    //             }

    //             (
    //                 CLAUDE_API_URL,
    //                 vec![
    //                     ("content-type", "application/json"),
    //                     ("x-api-key", &self.api_key),
    //                     ("anthropic-version", "2023-06-01"),
    //                 ],
    //                 body,
    //             )
    //         }
    //         AIModel::GPT4o | AIModel::GPT4oMini | AIModel::O1 | AIModel::O1Mini => {
    //             let api_messages: Vec<Value> = messages
    //                 .iter()
    //                 .map(|m| json!({ "role": &m.role, "content": &m.content }))
    //                 .collect();

    //             let body = json!({
    //                 "model": self.model.as_str(),
    //                 "messages": api_messages,
    //                 "max_tokens": 8192,
    //                 "temperature": 0.1,
    //             });

    //             (
    //                 OPENAI_API_URL,
    //                 vec![
    //                     ("content-type", "application/json"),
    //                     ("Authorization", &auth_header),
    //                 ],
    //                 body,
    //             )
    //         }
    //     };

    //     let mut request = client.post(url);
    //     for (key, value) in headers {
    //         request = request.header(key, value);
    //     }

    //     let response = request.json(&body).send().await?.json::<Value>().await?;

    //     match self.model {
    //         AIModel::Claude3 => {
    //             if let Some(error) = response.get("error") {
    //                 Err(format!("API Error: {:?}", error).into())
    //             } else {
    //                 response["content"]
    //                     .as_array()
    //                     .and_then(|arr| arr.first())
    //                     .and_then(|obj| obj["text"].as_str())
    //                     .ok_or_else(|| "Unexpected response format for Claude".into())
    //                     .map(String::from)
    //             }
    //         }
    //         AIModel::GPT4o | AIModel::O1 | AIModel::O1Mini | AIModel::GPT4oMini => {
    //             if let Some(error) = response.get("error") {
    //                 Err(format!("API Error: {:?}", error).into())
    //             } else {
    //                 response["choices"]
    //                     .as_array()
    //                     .and_then(|arr| arr.first())
    //                     .and_then(|obj| obj["message"]["content"].as_str())
    //                     .ok_or_else(|| {
    //                         let error_msg = format!(
    //                             "Unexpected response format for GPT-4o. Response: {:?}",
    //                             response
    //                         );
    //                         error_msg.into()
    //                     })
    //                     .map(String::from)
    //             }
    //         }
    //     }
    // }

    fn parse_response(
        &self,
        response: &str,
    ) -> Result<(Vec<(String, String)>, Vec<(String, String)>, String), Box<dyn Error>> {
        let rust_regex = Regex::new(r"(?s)```rust(?: filename=(.*?\.rs))?\n(.*?)```")?;
        let pax_regex = Regex::new(r"(?s)```pax(?: filename=(.*?\.pax))?\n(.*?)```")?;

        let mut rust_files = Vec::new();
        for cap in rust_regex.captures_iter(response) {
            let filename = cap.get(1).map_or("default.rs", |m| m.as_str()).to_string();
            let content = cap[2].trim().to_string();
            rust_files.push((filename, content));
        }

        let mut pax_files = Vec::new();
        for cap in pax_regex.captures_iter(response) {
            let filename = cap.get(1).map_or("default.pax", |m| m.as_str()).to_string();
            let content = cap[2].trim().to_string();
            pax_files.push((filename, content));
        }

        if rust_files.is_empty() && pax_files.is_empty() {
            return Err("No Rust or PAX files found in response".into());
        }

        // Return the parsed files along with the original LLM response text
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

    fn compile_and_run_project(&self) -> Result<bool, Box<dyn Error>> {
        let output = Command::new("./pax")
            .current_dir(output_dir())
            .arg("build")
            .output()?;

        if output.status.success() {
            println!("Project built successfully");
            Ok(true)
        } else {
            println!(
                "Build failed. Error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(false)
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




   
// pub async fn update_pax_file(
//     &self,
//     pax_content: &str,
//     prompt: &str,
//     request_id: u64,
//     tx: mpsc::UnboundedSender<(u64, String)>,
//     screenshot: Option<ScreenshotData>
// ) -> Result<(String, String), Box<dyn Error>> {

// if let Some(screenshot) = screenshot {
//     let screenshot_path = project_root!().join("screenshot.png");
//     let img = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
//         screenshot.width as u32,
//         screenshot.height as u32,
//         screenshot.data
//     ).expect("Failed to create image buffer");
    
//     img.save(&screenshot_path)?;
// }


//     let mut messages = vec![
//         Message {
//             role: "system".to_string(),
//             content: SYSTEM_PROMPT.to_string(),
//         },
//         Message {
//             role: "user".to_string(),
//             content: format!(
//                 "Here's the current PAX file content:\n\n```pax\n{}\n```\n\nPlease update this PAX file based on the following request:\n{}",
//                 pax_content, prompt
//             ),
//         },
//     ];

//     let mut retry_count = 0;
//     const MAX_RETRIES: usize = 5;

//     while retry_count < MAX_RETRIES {
//         tx.unbounded_send((request_id, "--- Sent request to OpenAI ---".to_string()))?;
//         let response = self.send_prompt(&messages).await?;
//         tx.unbounded_send((request_id, "Received response from OpenAI.".to_string()))?;

//         messages.push(Message {
//             role: "assistant".to_string(),
//             content: response.clone(),
//         });
//         match self.parse_response(&response) {
//             Ok(resp) => {
//                 let (_, pax_files, resp) = resp;
//                 let parse_errors = self.pre_parse_pax_files(&pax_files);
//                 if parse_errors.is_empty() {
//                     return Ok((pax_files[0].1.clone(), resp));
//                 } else {
//                     tx.unbounded_send((request_id,"PAX parsing errors detected:".to_string()))?;
//                     let error_message = format!(
//                         "The updated PAX file failed to parse. Error: {}. Please fix the PAX syntax errors and provide the corrected code.",
//                         parse_errors[0].1
//                     );
//                     messages.push(Message {
//                         role: "user".to_string(),
//                         content: error_message,
//                     });
//                     tx.unbounded_send((request_id,"Sending error message to AI for correction.".to_string()))?;
//                     retry_count += 1;
//                 }
//             }
//             Err(e) => {
//                 let error_message = format!(
//                     "The previous response could not be parsed correctly. Error: {}. Please provide the updated PAX file again, ensuring that it's properly formatted within a PAX code block.",
//                     e
//                 );
//                 messages.push(Message {
//                     role: "user".to_string(),
//                     content: error_message,
//                 });
//                 retry_count += 1;
//             }
//         }
//     }

//     Err(format!(
//         "Maximum retries ({}) reached while updating PAX file.",
//         MAX_RETRIES
//     )
//     .into())
// }

}
