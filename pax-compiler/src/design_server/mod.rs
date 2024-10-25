use actix::Addr;
use actix_web::middleware::Logger;

use actix_web::web::Data;
use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};
use actix_web::{post, HttpResponse, Result};
use actix_web_actors::ws;
use colored::Colorize;
use pax_generation::{AIModel, PaxAppGenerator};
use serde_json::json;
use std::net::TcpListener;
use std::{env, fs};

use env_logger;
use std::io::Write;

use crate::helpers::PAX_BADGE;
use crate::{RunContext, RunTarget};
use notify::{Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pax_manifest::PaxManifest;

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use websocket::PrivilegedAgentWebSocket;
use websocket::SocketMessageAccumulator;

#[allow(unused)]
mod llm;
pub mod static_server;
pub mod websocket;

pub struct AppState {
    serve_dir: Mutex<PathBuf>,
    userland_project_root: Mutex<PathBuf>,
    active_websocket_client: Mutex<Option<Addr<PrivilegedAgentWebSocket>>>,
    request_id_counter: Mutex<usize>,
    manifest: Mutex<Option<PaxManifest>>,
    last_written_timestamp: Mutex<SystemTime>,
}

impl AppState {
    pub fn new_empty() -> Self {
        Self {
            serve_dir: Mutex::new(PathBuf::new()),
            userland_project_root: Mutex::new(PathBuf::new()),
            active_websocket_client: Mutex::new(None),
            request_id_counter: Mutex::new(0),
            manifest: Mutex::new(None),
            last_written_timestamp: Mutex::new(UNIX_EPOCH),
        }
    }
    pub fn new(serve_dir: PathBuf, project_root: PathBuf, manifest: PaxManifest) -> Self {
        AppState {
            serve_dir: Mutex::new(serve_dir),
            userland_project_root: Mutex::new(project_root),
            active_websocket_client: Mutex::new(None),
            request_id_counter: Mutex::new(0),
            manifest: Mutex::new(Some(manifest)),
            last_written_timestamp: Mutex::new(SystemTime::now()),
        }
    }

    fn generate_request_id(&self) -> usize {
        let mut counter = self.request_id_counter.lock().unwrap();
        *counter += 1;
        *counter
    }

    pub fn update_last_written_timestamp(&self) {
        let mut last_written = self.last_written_timestamp.lock().unwrap();
        *last_written = SystemTime::now();
    }
}

#[get("/ws")]
pub async fn web_socket(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> impl Responder {
    ws::WsResponseBuilder::new(PrivilegedAgentWebSocket::new(state), &req, stream)
        .frame_size(2_000_000)
        .start()
}

#[allow(unused_assignments)]
pub fn start_server(
    static_file_path: &str,
    src_folder_to_watch: &str,
    manifest: PaxManifest,
) -> std::io::Result<()> {
    // Initialize logging
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_env(env_logger::Env::default())
        .format(|buf, record| writeln!(buf, "{} 🍱 Served {}", *PAX_BADGE, record.args()))
        .init();

    let initial_state = AppState::new(
        PathBuf::from(static_file_path),
        PathBuf::from_str(src_folder_to_watch).unwrap(),
        manifest,
    );
    let fs_path = initial_state.serve_dir.lock().unwrap().clone();
    let state = Data::new(initial_state);
    let _watcher = setup_file_watcher(state.clone(), src_folder_to_watch)
        .expect("Failed to setup file watcher");

    // Create a Runtime
    let runtime = actix_web::rt::System::new().block_on(async {
        let mut port = 8080;
        let server = loop {
            // Check if the port is available
            if TcpListener::bind(("127.0.0.1", port)).is_ok() {
                // Log the server details
                println!(
                    "{} 🗂️  Serving static files from {}",
                    *PAX_BADGE,
                    &fs_path.to_str().unwrap()
                );
                let address_msg = format!("http://127.0.0.1:{}", port).blue();
                let server_running_at_msg = format!("Server running at {}", address_msg).bold();
                println!("{} 📠 {}", *PAX_BADGE, server_running_at_msg);
                break HttpServer::new(move || {
                    App::new()
                        .wrap(Logger::new("| %s | %U"))
                        .app_data(state.clone())
                        .service(web_socket)
                        .service(
                            actix_files::Files::new("/*", fs_path.clone()).index_file("index.html"),
                        )
                })
                .bind(("127.0.0.1", port))
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

#[derive(Default)]
pub enum FileContent {
    Pax(String),
    Rust(String),
    #[default]
    Unknown,
}

#[derive(Default)]
struct WatcherFileChanged {
    pub contents: FileContent,
    pub path: String,
}

impl actix::Message for WatcherFileChanged {
    type Result = ();
}

pub fn setup_file_watcher(state: Data<AppState>, path: &str) -> Result<RecommendedWatcher, Error> {
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, Error>| match res {
            Ok(e) => {
                if let Some(addr) = &*state.active_websocket_client.lock().unwrap() {
                    let now = SystemTime::now();
                    // check last written time so we don't spam file changes when we serialize
                    let last_written = *state.last_written_timestamp.lock().unwrap();
                    if now
                        .duration_since(last_written)
                        .unwrap_or_default()
                        .as_millis()
                        > 1000
                    {
                        if let EventKind::Modify(_) = e.kind {
                            if let Some(path) = e.paths.first() {
                                match fs::read_to_string(path) {
                                    Ok(contents) => {
                                        let extension = path.extension();
                                        let msg = WatcherFileChanged {
                                            contents: match extension.and_then(|e| e.to_str()) {
                                                Some("pax") => FileContent::Pax(contents),
                                                Some("rs") => FileContent::Rust(contents),
                                                _ => FileContent::Unknown,
                                            },
                                            path: path.to_str().unwrap().to_string(),
                                        };
                                        addr.do_send(msg);
                                        state.update_last_written_timestamp();
                                    }
                                    Err(_) => (),
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("File system watch error: {:?}", e);
            }
        },
        Default::default(),
    )?;
    watcher.watch(Path::new(path), RecursiveMode::Recursive)?;
    Ok(watcher)
}

#[get("/ai")]
async fn ai_page() -> Result<HttpResponse> {
    let html_content = fs::read_to_string("static/ai_chat.html")?;
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(html_content))
}

#[derive(Deserialize)]
struct AiMessage {
    message: String,
}

fn create_designer_run_context() -> RunContext {
    RunContext {
        target: RunTarget::Web,
        project_path: PathBuf::from("../pax-designer".to_string()),
        verbose: false,
        should_also_run: false,
        is_libdev_mode: true,
        should_run_designer: true,
        process_child_ids: Arc::new(Mutex::new(vec![])),
        is_release: false,
    }
}

fn perform_build() -> std::io::Result<(PaxManifest, Option<PathBuf>)> {
    let ctx = create_designer_run_context();
    crate::perform_build(&ctx).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

fn perform_build_and_update_state(state: &AppState, folder_to_watch: &str) -> std::io::Result<()> {
    let (manifest, fs_path) = perform_build()?;

    // Update the state
    *state.serve_dir.lock().unwrap() = fs_path.expect("serve directory should exist");
    *state.userland_project_root.lock().unwrap() = PathBuf::from_str(folder_to_watch).unwrap();
    *state.manifest.lock().unwrap() = Some(manifest);

    Ok(())
}
