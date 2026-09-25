use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut pax_args = vec!["run"];
    pax_args.extend(args.iter().map(String::as_str));

    let project_dir = env!("CARGO_MANIFEST_DIR");
    let status = Command::new(std::path::Path::new(project_dir).join("pax"))
        .args(&pax_args)
        .current_dir(project_dir)
        .status()
        .expect("Failed to execute pax-cli");

    std::process::exit(status.code().unwrap_or(1));
}
