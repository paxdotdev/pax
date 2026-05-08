use crate::design_server::{display_addresses, static_files_service, DEFAULT_BIND_HOST};
use crate::helpers::PAX_BADGE;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use colored::Colorize;
use env_logger;
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;

pub fn start_server(fs_path: PathBuf) -> std::io::Result<()> {
    // Initialize logging
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_env(env_logger::Env::default())
        .format(|buf, record| writeln!(buf, "{} 🍱 Served {}", *PAX_BADGE, record.args()))
        .init();

    // Create a Runtime
    let runtime = actix_web::rt::System::new().block_on(async {
        let mut port = 8080;
        let server = loop {
            // Check if the port is available
            if TcpListener::bind((DEFAULT_BIND_HOST, port)).is_ok() {
                // Log the server details
                println!(
                    "{} 🗂️  Serving static files from {}",
                    *PAX_BADGE,
                    &fs_path.to_str().unwrap()
                );
                let address_msg = display_addresses(port).join(" or ").blue();
                let server_running_at_msg = format!("Server running at {}", address_msg).bold();
                println!("{} 📠 {}", *PAX_BADGE, server_running_at_msg);
                break HttpServer::new(move || {
                    App::new()
                        .wrap(Logger::new("| %s | %U"))
                        .service(static_files_service(fs_path.clone()))
                })
                .bind((DEFAULT_BIND_HOST, port))
                .expect("Error binding to address")
                .workers(2);
            } else {
                port += 1; // Try the next port
            }
        };

        server.run().await
    });

    runtime
}
