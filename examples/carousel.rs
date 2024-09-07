mod scripts;
use scripts::run_example;

fn main() {
    if let Err(error) = run_example() {
        eprintln!("Error: {}", error);
    }
}
