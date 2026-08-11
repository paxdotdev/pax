use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut pax_args = vec!["run"];
    pax_args.extend(args.iter().map(String::as_str));

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let status = Command::new("./pax")
        .args(&pax_args)
        .current_dir(current_dir)
        .status()
        .expect("Failed to execute pax-cli");

    std::process::exit(status.code().unwrap_or(1));
}
