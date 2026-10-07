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
    start_server_with_ready_callback(fs_path, public_dir, None, None)
}

pub(crate) fn start_server_with_ready_callback(
    fs_path: PathBuf,
    public_dir: Option<PathBuf>,
    requested_port: Option<u16>,
    ready_callback: Option<Box<dyn FnOnce() + Send>>,
) -> std::io::Result<()> {
    let server_owned_prefixes = crate::project_metadata::load_web_server_owned_prefixes(&fs_path)?;
    // Initialize logging
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_env(env_logger::Env::default())
        .format(|buf, record| writeln!(buf, "{} 🍱 Served {}", *PAX_BADGE, record.args()))
        .try_init()
        .ok();

    // Create a Runtime
    let mut ready_callback = ready_callback;
    let runtime = actix_web::rt::System::new().block_on(async {
        let listener = bind_listener(requested_port)?;
        let port = listener.local_addr()?.port();
        println!(
            "{} 🗂️  Serving static files from {}",
            *PAX_BADGE,
            fs_path.display()
        );
        println!(
            "{} 📠 {}",
            *PAX_BADGE,
            format!("Server running at {}", display_address_links(port)).bold()
        );
        let server = HttpServer::new(move || {
            App::new()
                .wrap(Logger::new("| %s | %U"))
                .service(static_files_service(
                    fs_path.clone(),
                    public_dir.clone(),
                    server_owned_prefixes.clone(),
                ))
        })
        .listen(listener)?
        .workers(2);

        let server = server.run();
        if let Some(ready_callback) = ready_callback.take() {
            ready_callback();
        }
        server.await
    });

    runtime
}

fn bind_listener(requested: Option<u16>) -> std::io::Result<TcpListener> {
    let mut port = requested.unwrap_or(8080);
    loop {
        match TcpListener::bind((DEFAULT_BIND_HOST, port)) {
            Ok(listener) => return Ok(listener),
            Err(error) if requested.is_some() || error.kind() != std::io::ErrorKind::AddrInUse => {
                return Err(error)
            }
            Err(_) => {
                port = port.checked_add(1).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::AddrNotAvailable,
                        "no available static-server port at or above 8080",
                    )
                })?
            }
        }
    }
}

#[cfg(test)]
mod port_tests {
    use super::*;
    #[test]
    fn explicit_port_is_reserved_and_cannot_silently_move() {
        let first = bind_listener(Some(0)).unwrap();
        let port = first.local_addr().unwrap().port();
        assert_ne!(port, 0);
        assert_eq!(
            bind_listener(Some(port)).unwrap_err().kind(),
            std::io::ErrorKind::AddrInUse
        );
    }
}
