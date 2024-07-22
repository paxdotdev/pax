use dotenv::dotenv;
use regex::Regex;
use reqwest;
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const OUTPUT_DIR: &str = "generated_project";

#[derive(Clone, Copy)]
pub enum AIModel {
    Claude3,
    GPT4,
}

impl AIModel {
    fn as_str(&self) -> &'static str {
        match self {
            AIModel::Claude3 => "claude-3-5-sonnet-20240620",
            AIModel::GPT4 => "gpt-4o",
        }
    }
}

#[derive(Clone)]
struct Message {
    role: String,
    content: String,
}

pub struct PaxAppGenerator {
    api_key: String,
    system_prompt: String,
    model: AIModel,
}

impl PaxAppGenerator {
    pub fn new(api_key: String, system_prompt: String, model: AIModel) -> Self {
        PaxAppGenerator {
            api_key,
            system_prompt,
            model,
        }
    }

    pub async fn generate_app(
        &self,
        prompt: &str,
        input_dir: Option<&Path>,
    ) -> Result<Vec<(String, String)>, Box<dyn Error>> {
        println!("\n--- Starting App Generation ---");
        println!("Prompt: {}", prompt);
    
        let mut user_content = prompt.to_string();
    
        if let Some(dir) = input_dir {
            let files_content = self.read_directory_files(dir)?;
            user_content.push_str(&format!("\n\nHere are the current files in the project:\n\n{}", files_content));
            println!("\nExisting files found in directory:");
            println!("{}", files_content);
        }
    
        let mut messages = vec![
            Message {
                role: "system".to_string(),
                content: self.system_prompt.clone(),
            },
            Message {
                role: "user".to_string(),
                content: user_content,
            },
        ];
    
        loop {
            println!("\n--- Sending Prompt to AI ---");
            let response = self.send_prompt(&messages).await?;
            println!("Received response from AI.");
            println!("AI's response:\n{}", response);
    
            messages.push(Message {
                role: "assistant".to_string(),
                content: response.clone(),
            });
    
            println!("\n--- Parsing Response ---");
            match self.parse_response(&response) {
                Ok((rust_files, pax_files)) => {
                    let all_files: Vec<(String, String)> = rust_files.into_iter().chain(pax_files).collect();
    
                    println!("\n--- Writing Files to Directory ---");
                    self.write_files_to_directory(&all_files)?;
                    println!("Files written to directory:");
                    for (filename, _) in &all_files {
                        println!("- {}", filename);
                    }
    
                    println!("\n--- Compiling and Running Project ---");
                    if self.compile_and_run_project()? {
                        println!("Project compiled and ran successfully.");
                        return Ok(all_files);
                    }
    
                    println!("\n--- Compilation or Runtime Error Detected ---");
                    let error_message = "The previous code resulted in a compilation or runtime error. Please fix it and provide the corrected code. Please write out the full file and make sure the filename is included in the markdown. PLEASE ONLY DO THINGS IN PAX THAT ARE SHOWN TO WORK IN THE SYSTEM PROMPT. DO NOT PUT ARBITRARY CODE IN PAX FILES OR ASSUME APIS IN RUST".to_string();
                    messages.push(Message {
                        role: "user".to_string(),
                        content: error_message,
                    });
                    println!("Sending error message to AI for correction.");
                }
                Err(e) => {
                    println!("Error parsing response: {}", e);
                    println!("Sending error message to AI for correction.");
                    let error_message = format!("The previous response could not be parsed correctly. Error: {}. Please provide the code again, ensuring that each file is properly formatted with the correct filename in a code block. For Rust files, use ```rust filename=filename.rs, and for Pax files, use ```pax filename=filename.pax.", e);
                    messages.push(Message {
                        role: "user".to_string(),
                        content: error_message,
                    });
                }
            }
        }
    }

    async fn send_prompt(&self, messages: &[Message]) -> Result<String, Box<dyn Error>> {
        let client = reqwest::Client::new();
    
        let api_messages: Vec<Value> = messages
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect();
    
        let auth_header = format!("Bearer {}", self.api_key);
    
        let (url, headers, body) = match self.model {
            AIModel::Claude3 => {
                let body = json!({
                    "model": self.model.as_str(),
                    "max_tokens": 4096,
                    "messages": api_messages,
                });
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
            AIModel::GPT4 => {
                let body = json!({
                    "model": self.model.as_str(),
                    "messages": api_messages,
                    "max_tokens": 4096,
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
    
        println!("Raw API response: {:?}", response);  // Debug print
    
        match self.model {
            AIModel::Claude3 => {
                response["content"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|obj| obj["text"].as_str())
                    .ok_or_else(|| "Unexpected response format for Claude".into())
                    .map(String::from)
            }
            AIModel::GPT4 => {
                response["choices"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|obj| obj["message"]["content"].as_str())
                    .ok_or_else(|| {
                        let error_msg = format!("Unexpected response format for GPT-4. Response: {:?}", response);
                        error_msg.into()
                    })
                    .map(String::from)
            }
        }
    }

    fn parse_response(
        &self,
        response: &str,
    ) -> Result<(Vec<(String, String)>, Vec<(String, String)>), Box<dyn Error>> {
        let rust_regex = Regex::new(r"(?s)```rust filename=(.*?\.rs)\n(.*?)```")?;
        let pax_regex = Regex::new(r"(?s)```pax filename=(.*?\.pax)\n(.*?)```")?;

        let mut rust_files = Vec::new();
        for cap in rust_regex.captures_iter(response) {
            let filename = Path::new(&cap[1])
                .file_name()
                .and_then(|f| f.to_str())
                .map(String::from)
                .unwrap_or_else(|| cap[1].to_string());
            let content = cap[2].trim().to_string();
            rust_files.push((filename, content));
        }

        let mut pax_files = Vec::new();
        for cap in pax_regex.captures_iter(response) {
            let filename = Path::new(&cap[1])
                .file_name()
                .and_then(|f| f.to_str())
                .map(String::from)
                .unwrap_or_else(|| cap[1].to_string());
            let content = cap[2].trim().to_string();
            pax_files.push((filename, content));
        }

        if rust_files.is_empty() && pax_files.is_empty() {
            return Err("No Rust or PAX files found in response".into());
        }

        Ok((rust_files, pax_files))
    }

    fn write_files_to_directory(&self, files: &[(String, String)]) -> io::Result<()> {
        let src_dir = Path::new(OUTPUT_DIR).join("src");

        // Create the src directory if it doesn't exist
        fs::create_dir_all(&src_dir)?;

        // Write or update files
        for (filename, content) in files {
            fs::write(src_dir.join(filename), content)?;
            println!("Wrote file: {}", filename);
        }

        Ok(())
    }

    fn compile_and_run_project(&self) -> Result<bool, Box<dyn Error>> {
        let output = Command::new("./pax")
            .current_dir(OUTPUT_DIR)
            .arg("build")
            .output()?;

        if output.status.success() {
            println!("Project built successfully");
            Ok(true)
        } else {
            println!("Build failed. Error: {}", String::from_utf8_lossy(&output.stderr));
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let claude_api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set in .env file");
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in .env file");
    let system_prompt = fs::read_to_string("system_prompt.txt")?;

    println!("Choose AI model:");
    println!("1. Claude 3");
    println!("2. GPT-4");
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    let model = match choice.trim() {
        "1" => AIModel::Claude3,
        "2" => AIModel::GPT4,
        _ => {
            println!("Invalid choice. Defaulting to Claude 3.");
            AIModel::Claude3
        }
    };

    let api_key = match model {
        AIModel::Claude3 => claude_api_key,
        AIModel::GPT4 => openai_api_key,
    };

    println!("Initializing PaxAppGenerator...");
    let generator = PaxAppGenerator::new(api_key, system_prompt, model);

    loop {
        println!("\n=== New Session ===");
        println!("Enter your prompt (or type 'exit' to quit):");
        let mut prompt = String::new();
        io::stdin().read_line(&mut prompt)?;
        prompt = prompt.trim().to_string();

        if prompt.to_lowercase() == "exit" {
            println!("Exiting program.");
            break;
        }

        let files = generator.generate_app(&prompt, Some(&Path::new(OUTPUT_DIR).join("src"))).await?;
        println!("\n=== App Generation Complete ===");
        println!("Files created:");
        for (filename, _) in &files {
            println!("- {}", filename);
        }

        loop {
            println!("\n--- Modification Session ---");
            println!("Enter modifications (or type 'done' to finish, 'exit' to quit):");
            let mut modifications = String::new();
            io::stdin().read_line(&mut modifications)?;
            modifications = modifications.trim().to_string();

            if modifications.to_lowercase() == "done" {
                println!("Modification session finished.");
                break;
            } else if modifications.to_lowercase() == "exit" {
                println!("Exiting program.");
                return Ok(());
            }

            let files = generator.generate_app(&modifications, Some(&Path::new(OUTPUT_DIR).join("src"))).await?;
            println!("\n=== App Modification Complete ===");
            println!("Updated files:");
            for (filename, _) in &files {
                println!("- {}", filename);
            }
        }
    }

    Ok(())
}