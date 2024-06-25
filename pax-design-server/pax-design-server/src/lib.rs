use actix::Addr;
use actix_web::middleware::Logger;

use actix_web::web::Data;
use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;
use colored::Colorize;

use notify::{Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pax_compiler::helpers::PAX_BADGE;
use pax_compiler::RunContext;
use pax_designtime::messages::LLMHelpResponse;
use pax_designtime::orm::template::NodeAction;
use pax_manifest::{PaxManifest, TypeId};

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use websocket::PrivilegedAgentWebSocket;

pub mod code_serialization;
mod llm;
pub mod websocket;
const PORT: u16 = 8252;

pub struct AppState {
    serve_dir: PathBuf,
    userland_project_root: PathBuf,
    active_websocket_client: Mutex<Option<Addr<PrivilegedAgentWebSocket>>>,
    request_id_counter: Mutex<usize>,
    manifest: Option<PaxManifest>,
    last_written_timestamp: Mutex<SystemTime>,
}

impl AppState {
    pub fn new_empty() -> Self {
        Self {
            serve_dir: PathBuf::new(),
            userland_project_root: PathBuf::new(),
            active_websocket_client: Mutex::new(None),
            request_id_counter: Mutex::new(0),
            manifest: None,
            last_written_timestamp: Mutex::new(UNIX_EPOCH),
        }
    }
    pub fn new(serve_dir: PathBuf, project_root: PathBuf, manifest: PaxManifest) -> Self {
        AppState {
            serve_dir,
            userland_project_root: project_root,
            active_websocket_client: Mutex::new(None),
            request_id_counter: Mutex::new(0),
            manifest: Some(manifest),
            last_written_timestamp: Mutex::new(UNIX_EPOCH),
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
pub async fn start_server(folder_to_watch: &str) -> std::io::Result<()> {
    std::env::set_var("PAX_WORKSPACE_ROOT", "../pax");
    let ctx = RunContext {
        target: pax_compiler::RunTarget::Web,
        path: "../pax-designer".to_string(),
        verbose: false,
        should_also_run: false,
        is_libdev_mode: true,
        process_child_ids: Arc::new(Mutex::new(vec![])),
        is_release: false,
    };

    let (manifest, fs_path) = pax_compiler::perform_build(&ctx).unwrap();

    let state = Data::new(AppState::new(
        fs_path.clone().expect("serve directory should exist"),
        PathBuf::from_str(folder_to_watch).unwrap(),
        manifest,
    ));
    let _watcher =
        setup_file_watcher(state.clone(), folder_to_watch).expect("Failed to setup file watcher");

    let _watcher =
        setup_file_watcher(state.clone(), "../pax-designer").expect("Failed to setup file watcher");

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(state.clone())
            .service(web_socket)
            .service(
                actix_files::Files::new("/*", fs_path.clone().unwrap()).index_file("index.html"),
            )
    })
    .bind(("127.0.0.1", PORT))?;

    let address_msg = format!("http://127.0.0.1:{}", PORT).blue();
    let server_running_at_msg = format!("Server running at {}", address_msg).bold();
    println!("{} 📠 {}", *PAX_BADGE, server_running_at_msg);

    server.run().await
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
