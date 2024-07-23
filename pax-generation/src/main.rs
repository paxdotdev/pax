use dotenv::dotenv;
use pax_generation::AIModel;
use pax_generation::PaxAppGenerator;
use std::env;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};

macro_rules! project_root {
    () => {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    };
}

fn output_dir() -> PathBuf {
    project_root!().join("generated_project")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let claude_api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set in .env file");
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in .env file");

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

    println!("Is this a designer project? (yes/no):");
    let mut is_designer = String::new();
    io::stdin().read_line(&mut is_designer)?;
    let is_designer_project = is_designer.trim().to_lowercase() == "yes";

    println!("Initializing PaxAppGenerator...");
    let generator = PaxAppGenerator::new(api_key, model);

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
        let files = generator.generate_app(&prompt, Some(&output_dir().join("src")), is_designer_project).await?;
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

            let files = generator.generate_app(&modifications, Some(&output_dir().join("src")), is_designer_project).await?;
            println!("\n=== App Modification Complete ===");
            println!("Updated files:");
            for (filename, _) in &files {
                println!("- {}", filename);
            }
        }
    }

    Ok(())
}