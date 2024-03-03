use std::env;

use pax_design_server::start_server;

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <path_to_watch>", args[0]);
        std::process::exit(1);
    }
    let path_to_watch = &args[1];
    start_server(path_to_watch).await
}
