use crate::design_server::{display_address_links, static_files_service, DEFAULT_BIND_HOST};
use crate::helpers::PAX_BADGE;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use colored::Colorize;
use env_logger;
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;

pub fn start_server(fs_path: PathBuf, public_dir: Option<PathBuf>) -> std::io::Result<()> {
    start_server_with_ready_callback(fs_path, public_dir, None)
}

pub(crate) fn start_server_with_ready_callback(
    fs_path: PathBuf,
    public_dir: Option<PathBuf>,
    ready_callback: Option<Box<dyn FnOnce() + Send>>,
) -> std::io::Result<()> {
    let server_owned_prefixes = crate::project_metadata::load_web_server_owned_prefixes(&fs_path)?;
    // Initialize logging
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_env(env_logger::Env::default())
        .format(|buf, record| writeln!(buf, "{} 🍱 Served {}", *PAX_BADGE, record.args()))
        .init();

    // Create a Runtime
    let mut ready_callback = ready_callback;
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
                let address_msg = display_address_links(port);
                let server_running_at_msg = format!("Server running at {}", address_msg).bold();
                println!("{} 📠 {}", *PAX_BADGE, server_running_at_msg);
                break HttpServer::new(move || {
                    App::new()
                        .wrap(Logger::new("| %s | %U"))
                        .service(static_files_service(
                            fs_path.clone(),
                            public_dir.clone(),
                            server_owned_prefixes.clone(),
                        ))
                })
                .bind((DEFAULT_BIND_HOST, port))?
                .workers(2);
            } else {
                port = port.checked_add(1).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::AddrNotAvailable,
                        "no available static-server port at or above 8080",
                    )
                })?;
            }
        };

        let server = server.run();
        if let Some(ready_callback) = ready_callback.take() {
            ready_callback();
        }
        server.await
    });

    runtime
}
