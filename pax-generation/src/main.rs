use dotenv::dotenv;
use regex::Regex;
use reqwest;
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Read};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const OUTPUT_DIR: &str = "generated_project";
const MODEL: &str = "claude-3-5-sonnet-20240620";

#[derive(Clone)]
struct Message {
    role: String,
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set in .env file");

    let system_prompt = read_system_prompt("system_prompt.txt")?;
    let user_prompt = get_user_prompt()?;

    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: system_prompt,
        },
        Message {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    loop {
        println!("Sending prompt to Claude...");
        let response = send_prompt_to_claude(&api_key, &messages).await?;
        messages.push(Message {
            role: "assistant".to_string(),
            content: response.clone(),
        });

        println!("Parsing response...");
        let (rust_file, pax_files) = parse_response(&response)?;

        println!("Writing files to directory...");
        write_files_to_directory(&rust_file, &pax_files)?;

        println!("Compiling and running project...");
        match compile_and_run_project() {
            Ok(output) => {
                println!("Project ran successfully. Output:\n{}", output);

                // Run the web server
                let web_dir = Path::new(OUTPUT_DIR)
                    .join(".pax")
                    .join("build")
                    .join("debug")
                    .join("web");
                run_web_server(&web_dir)?;

                break;
            }
            Err(error) => {
                println!("Compilation or runtime error: {}", error);
                let error_message = format!(
                    "The previous code resulted in the following error: {}.\n 
                     Please fix it and provide the corrected code. Please write out the full file and make sure the filename is included in the markdown.",
                    error
                );
                messages.push(Message {
                    role: "user".to_string(),
                    content: error_message,
                });
            }
        }
    }

    Ok(())
}

fn read_system_prompt(filename: &str) -> io::Result<String> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn get_user_prompt() -> io::Result<String> {
    println!("Enter the type of application you want to generate: (e.g. bouncing balls)");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

async fn send_prompt_to_claude(
    api_key: &str,
    messages: &[Message],
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // Separate system message from other messages
    let (system_message, user_messages): (Option<&Message>, Vec<&Message>) = if !messages.is_empty()
    {
        if messages[0].role == "system" {
            (Some(&messages[0]), messages[1..].iter().collect())
        } else {
            (None, messages.iter().collect())
        }
    } else {
        (None, Vec::new())
    };

    let mut request_body = json!({
        "model": MODEL,
        "max_tokens": 4096,
        "messages": user_messages.iter().map(|m| json!({
            "role": &m.role,
            "content": &m.content
        })).collect::<Vec<_>>(),
    });

    // Add system message as a top-level parameter if present
    if let Some(sys_msg) = system_message {
        request_body["system"] = json!(&sys_msg.content);
    }

    let response = client
        .post(API_URL)
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request_body)
        .send()
        .await?
        .json::<Value>()
        .await?;

    println!("API Response: {:?}", response);

    if let Some(error) = response.get("error") {
        return Err(format!("API Error: {:?}", error).into());
    }

    response["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|obj| obj["text"].as_str())
        .ok_or_else(|| "Unexpected response format".into())
        .map(String::from)
}

fn parse_response(
    response: &str,
) -> Result<(Option<(String, String)>, Vec<(String, String)>), Box<dyn std::error::Error>> {
    let rust_regex = Regex::new(r"(?s)```rust filename=(.*?\.rs)\n(.*?)```")?;
    let pax_regex = Regex::new(r"(?s)```pax filename=(.*?\.pax)\n(.*?)```")?;

    let rust_file = rust_regex.captures(response).map(|cap| {
        let filename = Path::new(&cap[1])
            .file_name()
            .and_then(|f| f.to_str())
            .map(String::from)
            .unwrap_or_else(|| cap[1].to_string());
        let content = cap[2].trim().to_string();
        (filename, content)
    });

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

    if rust_file.is_none() && pax_files.is_empty() {
        return Err("No Rust or PAX files found in response".into());
    }

    Ok((rust_file, pax_files))
}

fn write_files_to_directory(
    rust_file: &Option<(String, String)>,
    pax_files: &[(String, String)],
) -> io::Result<()> {
    let src_dir = Path::new(OUTPUT_DIR).join("src");

    // Clear the src directory if it exists
    if src_dir.exists() {
        fs::remove_dir_all(&src_dir)?;
    }

    // Recreate the src directory
    fs::create_dir_all(&src_dir)?;

    // Write Rust file if present
    if let Some((filename, content)) = rust_file {
        fs::write(src_dir.join(filename), content)?;
    }

    // Write PAX files
    for (filename, content) in pax_files {
        fs::write(src_dir.join(filename), content)?;
    }

    Ok(())
}

fn compile_and_run_project() -> Result<String, String> {
    let output = Command::new("./pax")
        .current_dir(OUTPUT_DIR)
        .arg("build")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn run_web_server(web_dir: &Path) -> Result<(), Box<dyn Error>> {
    println!("Starting web server in {:?}", web_dir);

    let mut child = Command::new("serve")
        .current_dir(web_dir)
        .arg(".")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    // Spawn a thread to handle stdout
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("Server: {}", line);
            }
        }
    });

    // Spawn a thread to handle stderr
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("Server error: {}", line);
            }
        }
    });

    println!("Web server is running. Press Ctrl+C to stop.");

    // Wait for the child process to exit (which it won't unless manually stopped)
    child.wait()?;

    Ok(())
}
