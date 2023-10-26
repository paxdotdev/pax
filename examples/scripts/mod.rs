use std::env;
use std::path::Path;
use std::process::Command;

/// Helper function to run the example in the `examples/src` directory.
/// This is needed to proxy default cargo run --examples which expects 
/// a named .rs file in the root of the examples directory.
pub fn run_example() -> Result<(), String> {
    let current_exe_path = env::current_exe().map_err(|_| "Failed to get current executable path")?;
    let file_name = current_exe_path.file_stem()
        .ok_or("Failed to get file name")?
        .to_str()
        .ok_or("Failed to convert to string")?;

    let current_dir = env::current_dir().map_err(|_| "Failed to get current directory")?;
    let target_dir = current_dir.join(format!("examples/src/{}", file_name));

    // In the case of a fresh clone with no .pax folder, clean in-case we have rel. paths
    if !Path::new(&target_dir).join(".pax").exists() {
        // Build local pax-cli
        Command::new("cargo")
            .arg("build")
            .current_dir(&current_dir.join("pax-cli"))
            .status()
            .map_err(|_| "Failed to build pax-cli")?;
        // clean .pax dependencies
        Command::new("../../../target/debug/pax-cli")
            .arg("clean")
            .current_dir(&target_dir)
            .status()
            .map_err(|_| "Failed to run pax-cli clean")?;
    }

    Command::new("cargo")
        .arg("run")
        .current_dir(target_dir)
        .status()
        .map_err(|_| "Failed to run cargo")?;
    
    Ok(())
}