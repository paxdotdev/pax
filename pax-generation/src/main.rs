use dotenv::dotenv;
use futures::channel::mpsc;
use futures::StreamExt;
use pax_generation::AIModel;
use pax_generation::PaxAppGenerator;
use std::env;
use std::error::Error;
use std::fs;
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

fn pax_file_path() -> PathBuf {
    output_dir().join("src").join("lib.pax")
}

fn ensure_seed_pax_file(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(
        path,
        r#"#[main]
component Example {
    <Text text="Hello Pax" />
}
"#,
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let claude_api_key =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set in .env file");
    let openai_api_key =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in .env file");

    println!("Choose AI model:");
    println!("1. Claude 3");
    println!("2. GPT-4");
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    let model = match choice.trim() {
        "1" => AIModel::Claude3,
        "2" => AIModel::GPT4o,
        _ => {
            println!("Invalid choice. Defaulting to Claude 3.");
            AIModel::Claude3
        }
    };

    let api_key = match model {
        AIModel::Claude3 => claude_api_key,
        AIModel::GPT4o | AIModel::GPT4oMini | AIModel::O1 | AIModel::O1Mini => openai_api_key,
    };

    println!("Is this a designer project? (yes/no):");
    let mut is_designer = String::new();
    io::stdin().read_line(&mut is_designer)?;
    let is_designer_project = is_designer.trim().to_lowercase() == "yes";

    println!("Initializing PaxAppGenerator...");
    let generator = PaxAppGenerator::new(api_key, model);
    let pax_file = pax_file_path();
    ensure_seed_pax_file(&pax_file)?;

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
        println!("pax_file: {:?}", pax_file);
        let current_pax = fs::read_to_string(&pax_file)?;
        let (tx, mut rx) = mpsc::unbounded();
        let (updated_pax, response) = generator
            .update_pax_file(&current_pax, &prompt, 0, tx, None, &model)
            .await?;
        fs::write(&pax_file, &updated_pax)?;

        println!("\n=== App Generation Complete ===");
        println!("Updated file:");
        println!("- {}", pax_file.display());
        while let Some((_, message)) = rx.next().await {
            println!("{}", message);
        }
        println!("\nAssistant response:\n{}", response);

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

            let current_pax = fs::read_to_string(&pax_file)?;
            let (tx, mut rx) = mpsc::unbounded();
            let (updated_pax, response) = generator
                .update_pax_file(&current_pax, &modifications, 0, tx, None, &model)
                .await?;
            fs::write(&pax_file, &updated_pax)?;
            println!("\n=== App Modification Complete ===");
            println!("Updated file:");
            println!("- {}", pax_file.display());
            while let Some((_, message)) = rx.next().await {
                println!("{}", message);
            }
            println!("\nAssistant response:\n{}", response);
        }
    }

    Ok(())
}
