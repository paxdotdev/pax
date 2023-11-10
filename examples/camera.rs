mod scripts;
use scripts::run_example;

mod


fn main() {
    if let Err(error) = run_example() {
        eprintln!("Error: {}", error);
    }
}
