use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let pax_args = std::iter::once("run")
        .chain(args.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let current_dir = env::current_dir().expect("failed to get current directory");

    let status = Command::new("./pax")
        .args(&pax_args)
        .current_dir(current_dir)
        .status()
        .expect("failed to execute pax-cli");

    std::process::exit(status.code().unwrap_or(1));
}
