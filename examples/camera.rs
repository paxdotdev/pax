use std::env;
use std::process::Command;

fn main() {
    let current_exe_path = env::current_exe().expect("Failed to get current executable path");
    let file_name = current_exe_path.file_stem()
        .expect("Failed to get file name")
        .to_str()
        .expect("Failed to convert to string");

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let target_dir = current_dir.join(format!("examples/src/{}",file_name));
    env::set_current_dir(&target_dir).expect("Failed to change directory");

    Command::new("cargo")
        .arg("run")
        .status()
        .expect("Failed to run cargo");
}
