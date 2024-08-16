use actix::Addr;
use actix_web::middleware::Logger;

use actix_web::web::Data;
use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};
use actix_web::{post, HttpResponse, Result};
use actix_web_actors::ws;
use colored::Colorize;
use pax_generation::{AIModel, PaxAppGenerator};
use serde_with::serde::de::Deserialize;
use serde_with::serde::ser::Serialize;
use serde_json::json;
use std::{env, fs};
use std::net::TcpListener;

use notify::{Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use crate::helpers::PAX_BADGE;
use crate::{RunContext, RunTarget};
use pax_designtime::messages::LLMHelpResponse;
use pax_designtime::orm::template::NodeAction;
use pax_manifest::{PaxManifest, TypeId};

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use websocket::PrivilegedAgentWebSocket;

pub mod code_serialization;
#[allow(unused)]
mod llm;
pub mod websocket;
pub mod static_server;


const PORT: u16 = 8252;

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
pub async fn start_server(folder_to_watch: &str, with_designer: bool, is_libdev_mode: bool) -> std::io::Result<()> {

    if is_libdev_mode {
        // Allows libdev of web chassis TS files
        std::env::set_var("PAX_WORKSPACE_ROOT", "../../../");
    }

    // Initialize logging
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_env(env_logger::Env::default())
        .format(|buf, record| writeln!(buf, "{} 🍱 Served {}", *PAX_BADGE, record.args()))
        .init();



    let design_server_details = if with_designer {
        {
            let initial_state = perform_build_and_create_state(folder_to_watch)?;
            let fs_path = initial_state.serve_dir.lock().unwrap().clone();
            let state = Data::new(initial_state);
            let _watcher =
                setup_file_watcher(state.clone(), folder_to_watch).expect("Failed to setup file watcher");

            Some((fs_path,state,_watcher))
        }

    } else {
        None
    };




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
                    &folder_to_watch.to_str().unwrap()
                );
                let address_msg = format!("http://127.0.0.1:{}", port).blue();
                let server_running_at_msg = format!("Server running at {}", address_msg).bold();
                println!("{} 📠 {}", *PAX_BADGE, server_running_at_msg);
                break HttpServer::new(move || {
                    let mut app = App::new().wrap(Logger::new("| %s | %U")).service(
                        actix_files::Files::new("/*", folder_to_watch.clone()).index_file("index.html"),
                    );

                    if with_designer {
                        app = app.app_data(design_server_details.unwrap().1.clone())
                            .service(ai_page)
                            .service(ai_submit)
                            .service(web_socket);
                    }

                    app
                }).bind(("127.0.0.1", port))
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

struct LLMHelpResponseMessage {
    pub request_id: String,
    pub component: TypeId,
    pub actions: Vec<NodeAction>,
}

impl actix::Message for LLMHelpResponseMessage {
    type Result = ();
}

impl From<LLMHelpResponseMessage> for LLMHelpResponse {
    fn from(value: LLMHelpResponseMessage) -> Self {
        LLMHelpResponse {
            request_id: value.request_id,
            component_type_id: value.component,
            response: value.actions,
        }
    }
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
                                    Err(e) => println!("Error reading file: {:?}", e),
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

#[post("/ai")]
async fn ai_submit(message: web::Json<AiMessage>, state: web::Data<AppState>) -> HttpResponse {
    let userland_project_root = state.userland_project_root.lock().unwrap().clone();
    let claude_api_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": "ANTHROPIC_API_KEY not set in environment"
            }))
        }
    };

    let pax_app_generator = PaxAppGenerator::new(claude_api_key, AIModel::Claude3);
    let output = userland_project_root.clone().join("src");

    match pax_app_generator
        .generate_app(&message.message, Some(&output), true)
        .await
    {
        Ok(_) => {
            match perform_build_and_update_state(&state, userland_project_root.to_str().unwrap()) {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "status": "success",
                    "response": "App generated and built successfully.",
                })),
                Err(e) => {
                    println!("Error performing build and updating state: {:?}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "status": "error",
                        "message": "Failed to build the generated app"
                    }))
                }
            }
        }
        Err(e) => {
            println!("Error generating app: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": "Failed to generate app"
            }))
        }
    }
}

#[derive(Deserialize)]
struct AiMessage {
    message: String,
}

fn create_run_context() -> RunContext {
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
    std::env::set_var("PAX_WORKSPACE_ROOT", "../pax");
    let ctx = create_run_context();
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

fn perform_build_and_create_state(folder_to_watch: &str) -> std::io::Result<AppState> {
    let (manifest, fs_path) = perform_build()?;

    Ok(AppState::new(
        fs_path.expect("serve directory should exist"),
        PathBuf::from_str(folder_to_watch).unwrap(),
        manifest,
    ))
}
