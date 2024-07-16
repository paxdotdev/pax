use std::process::Command;
use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let pax_args = {
        let mut extended_args = vec!["run"];
        extended_args.extend(args.iter().map(|arg| arg.as_str()));
        extended_args
    };

    let current_dir = env::current_dir().expect("Failed to get current directory");

    let status = Command::new("./pax")
        .args(&pax_args)
        .current_dir(current_dir)
        .status()
        .expect("Failed to execute pax-cli");

    std::process::exit(status.code().unwrap_or(1));
}